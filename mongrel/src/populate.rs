use bson::oid::ObjectId;
use mongodb::Database;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;

use crate::{
    error::{MongooseError, Result},
    schema::MongooseSchema,
};

/// A typed reference to another collection's document.
///
/// `Ref<T>` acts like Mongoose's `ObjectId` ref field: it stores the raw
/// `ObjectId` in MongoDB, but can be _populated_ in memory to hold the full
/// deserialized document.
///
/// # Storing a reference
///
/// ```rust,ignore
/// pub struct Post {
///     pub author: Ref<User>,   // stored as ObjectId in MongoDB
/// }
///
/// // Create with an id:
/// Post { author: Ref::Id(user.id.unwrap()), .. }
/// ```
///
/// # Populating
///
/// Call [`resolve_ref`] inside a [`Populate`] impl to load the referenced doc:
///
/// ```rust,ignore
/// self.author = resolve_ref(self.author, Arc::clone(&db)).await?;
/// ```
///
/// # Serde behaviour
///
/// `Ref<T>` always serializes as a plain BSON `ObjectId`. Attempting to
/// serialize a `Ref::Populated` variant returns a serde error — populate only
/// in memory, persist the id variant to MongoDB.
#[derive(Debug, Clone)]
pub enum Ref<T> {
    /// The document has not been loaded — only the `ObjectId` is known.
    Id(ObjectId),
    /// The document has been loaded from MongoDB.
    Populated(Box<T>),
}

impl<T> Ref<T> {
    /// Return the inner `ObjectId` if this ref is unpopulated, or `None`.
    pub fn id(&self) -> Option<&ObjectId> {
        match self {
            Ref::Id(id) => Some(id),
            Ref::Populated(_) => None,
        }
    }

    /// Return a reference to the inner document if populated, or `None`.
    pub fn populated(&self) -> Option<&T> {
        match self {
            Ref::Populated(doc) => Some(doc),
            Ref::Id(_) => None,
        }
    }

    /// `true` if the document has been loaded from MongoDB.
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

/// Implement this trait on a schema that contains [`Ref<T>`] fields to enable
/// loading referenced documents from MongoDB.
///
/// # Example
///
/// ```rust,ignore
/// #[async_trait]
/// impl Populate for Post {
///     async fn populate(mut self, db: Arc<Database>) -> Result<Self> {
///         self.author = resolve_ref(self.author, Arc::clone(&db)).await?;
///         Ok(self)
///     }
/// }
///
/// let post = posts.find_by_id("...").await?.unwrap();
/// let post = post.populate(Arc::clone(&db)).await?;
/// println!("{}", post.author.populated().unwrap().name);
/// ```
#[async_trait::async_trait]
pub trait Populate: Sized + Send + Sync {
    /// Load all `Ref<T>` fields by fetching the referenced documents from `db`.
    async fn populate(self, db: Arc<Database>) -> Result<Self>;
}

/// Resolve a single [`Ref<T>`] by fetching the referenced document from MongoDB.
///
/// If the `Ref` is already populated this is a no-op. If the referenced
/// document is not found, returns [`MongooseError::NotFound`].
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

/// Like [`resolve_ref`] but for `Option<Ref<T>>` fields. Returns `Ok(None)`
/// if the field is `None`.
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
