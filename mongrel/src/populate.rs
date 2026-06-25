use bson::oid::ObjectId;
use mongodb::Database;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;

use crate::{
    error::{MongooseError, Result},
    schema::MongooseSchema,
};

// ── Ref<T> ────────────────────────────────────────────────────────────────────
// Stores either just an ObjectId (un-populated) or the full document (populated).
// Serializes/deserializes as a plain ObjectId so it stores correctly in MongoDB.

#[derive(Debug, Clone)]
pub enum Ref<T> {
    Id(ObjectId),
    Populated(Box<T>),
}

impl<T> Ref<T> {
    pub fn id(&self) -> Option<&ObjectId> {
        match self {
            Ref::Id(id) => Some(id),
            Ref::Populated(_) => None,
        }
    }

    pub fn populated(&self) -> Option<&T> {
        match self {
            Ref::Populated(doc) => Some(doc),
            Ref::Id(_) => None,
        }
    }

    pub fn is_populated(&self) -> bool {
        matches!(self, Ref::Populated(_))
    }
}

// ── Serde: always serializes as the ObjectId ──────────────────────────────────

impl<T> Serialize for Ref<T> {
    fn serialize<S: Serializer>(&self, ser: S) -> std::result::Result<S::Ok, S::Error> {
        let id = match self {
            Ref::Id(id) => id,
            Ref::Populated(_) => {
                return Err(serde::ser::Error::custom(
                    "Cannot serialize a populated Ref — save the inner document separately",
                ))
            }
        };
        id.serialize(ser)
    }
}

impl<'de, T> Deserialize<'de> for Ref<T> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> std::result::Result<Self, D::Error> {
        let id = ObjectId::deserialize(de)?;
        Ok(Ref::Id(id))
    }
}

// ── Populate trait ────────────────────────────────────────────────────────────
// Anything that holds Ref<T> fields implements this to resolve them.

#[async_trait::async_trait]
pub trait Populate: Sized + Send + Sync {
    async fn populate(self, db: Arc<Database>) -> Result<Self>;
}

// ── Standalone populate helper ────────────────────────────────────────────────
// Used inside custom Populate impls to resolve a single Ref<T>.

pub async fn resolve_ref<T>(r: Ref<T>, db: Arc<Database>) -> Result<Ref<T>>
where
    T: MongooseSchema
        + serde::de::DeserializeOwned
        + serde::Serialize
        + Unpin
        + Send
        + Sync
        + 'static,
{
    match r {
        Ref::Populated(_) => Ok(r), // already populated
        Ref::Id(id) => {
            let col = db.collection::<T>(T::collection_name());
            let doc = col
                .find_one(bson::doc! { "_id": id })
                .await?
                .ok_or(MongooseError::NotFound)?;
            Ok(Ref::Populated(Box::new(doc)))
        }
    }
}

// ── Optional Ref support ──────────────────────────────────────────────────────

pub async fn resolve_opt_ref<T>(
    r: Option<Ref<T>>,
    db: Arc<Database>,
) -> Result<Option<Ref<T>>>
where
    T: MongooseSchema
        + serde::de::DeserializeOwned
        + serde::Serialize
        + Unpin
        + Send
        + Sync
        + 'static,
{
    match r {
        None => Ok(None),
        Some(r) => Ok(Some(resolve_ref(r, db).await?)),
    }
}
