use bson::{doc, oid::ObjectId, to_bson, Document};
use chrono::Utc;
use mongodb::options::{FindOneAndUpdateOptions, ReturnDocument};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

use crate::{
    aggregation::AggregationPipeline,
    error::{MongooseError, Result},
    hooks::Hooks,
    index::{def_to_model, MongooseIndexes},
    pagination::PaginateBuilder,
    query::QueryBuilder,
    schema::MongooseSchema,
};

pub use crate::query::SortDir;

// ── Model<T> ─────────────────────────────────────────────────────────────────

pub struct Model<T> {
    db: Arc<mongodb::Database>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Model<T>
where
    T: MongooseSchema + Serialize + DeserializeOwned + Hooks + Unpin + Send + Sync + 'static,
{
    pub fn new(db: Arc<mongodb::Database>) -> Self {
        Self {
            db,
            _marker: std::marker::PhantomData,
        }
    }

    fn col(&self) -> Arc<mongodb::Collection<T>> {
        Arc::new(self.db.collection::<T>(T::collection_name()))
    }

    fn raw_col(&self) -> mongodb::Collection<Document> {
        self.db.collection::<Document>(T::collection_name())
    }

    // ── Create ────────────────────────────────────────────────────────────────

    pub async fn create(&self, mut doc: T) -> Result<T> {
        doc.pre_validate().await?;
        doc.validate()?;
        doc.post_validate().await?;
        doc.pre_save().await?;

        let mut raw = to_document(&doc)?;

        if T::timestamps() {
            let now = bson::DateTime::from_chrono(Utc::now());
            raw.insert("created_at", now);
            raw.insert("updated_at", now);
        }

        let result = self.raw_col().insert_one(raw).await?;
        let id = result
            .inserted_id
            .as_object_id()
            .ok_or_else(|| MongooseError::Serialization("No ObjectId returned".into()))?;

        let saved = self
            .col()
            .find_one(doc! { "_id": id })
            .await?
            .ok_or(MongooseError::NotFound)?;

        saved.post_save().await?;
        Ok(saved)
    }

    // ── Find ──────────────────────────────────────────────────────────────────

    pub fn find(&self) -> QueryBuilder<T> {
        QueryBuilder::new(self.col())
    }

    pub fn find_many(&self, filter: Document) -> QueryBuilder<T> {
        QueryBuilder::new(self.col()).filter(filter)
    }

    pub fn find_one_where(&self, filter: Document) -> QueryBuilder<T> {
        QueryBuilder::new(self.col()).filter(filter)
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<T>> {
        let oid = parse_oid(id)?;
        Ok(self.col().find_one(doc! { "_id": oid }).await?)
    }

    // ── Update ────────────────────────────────────────────────────────────────

    pub async fn find_by_id_and_update(
        &self,
        id: &str,
        update: Document,
    ) -> Result<Option<T>> {
        let oid = parse_oid(id)?;
        self.find_one_and_update(doc! { "_id": oid }, update).await
    }

    pub async fn find_one_and_update(
        &self,
        filter: Document,
        mut update: Document,
    ) -> Result<Option<T>> {
        if T::timestamps() {
            let set = update
                .entry("$set".to_string())
                .or_insert_with(|| bson::Bson::Document(Document::new()));
            if let bson::Bson::Document(d) = set {
                d.insert("updated_at", bson::DateTime::from_chrono(Utc::now()));
            }
        }

        let opts = FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();

        Ok(self
            .col()
            .find_one_and_update(filter, update)
            .with_options(opts)
            .await?)
    }

    pub async fn update_many(&self, filter: Document, update: Document) -> Result<u64> {
        let res = self.col().update_many(filter, update).await?;
        Ok(res.modified_count)
    }

    // ── Delete ────────────────────────────────────────────────────────────────

    pub async fn find_by_id_and_delete(&self, id: &str) -> Result<Option<T>> {
        let oid = parse_oid(id)?;
        self.find_one_and_delete(doc! { "_id": oid }).await
    }

    pub async fn find_one_and_delete(&self, filter: Document) -> Result<Option<T>> {
        Ok(self.col().find_one_and_delete(filter).await?)
    }

    pub async fn delete_many(&self, filter: Document) -> Result<u64> {
        let res = self.col().delete_many(filter).await?;
        Ok(res.deleted_count)
    }

    // ── Upsert ────────────────────────────────────────────────────────────────

    pub async fn find_one_and_upsert(&self, filter: Document, update: Document) -> Result<T> {
        let opts = FindOneAndUpdateOptions::builder()
            .upsert(true)
            .return_document(ReturnDocument::After)
            .build();

        self.col()
            .find_one_and_update(filter, update)
            .with_options(opts)
            .await?
            .ok_or(MongooseError::NotFound)
    }

    // ── Count ─────────────────────────────────────────────────────────────────

    pub async fn count_documents(&self, filter: Document) -> Result<u64> {
        Ok(self.col().count_documents(filter).await?)
    }

    // ── Pagination ────────────────────────────────────────────────────────────

    pub fn paginate(&self, page: u64, per_page: u64) -> PaginateBuilder<T> {
        PaginateBuilder::new((*self.col()).clone(), page, per_page)
    }

    // ── Aggregation ───────────────────────────────────────────────────────────

    pub fn aggregate(&self) -> AggregationPipeline<T> {
        AggregationPipeline::new((*self.col()).clone())
    }

    // ── Indexes ───────────────────────────────────────────────────────────────

    /// Create unique-field indexes declared via `#[field(unique)]`.
    pub async fn ensure_indexes(&self) -> Result<()> {
        use mongodb::IndexModel;

        for field in T::unique_fields() {
            let keys = doc! { *field: 1 };
            let mut opts = mongodb::options::IndexOptions::default();
            opts.unique = Some(true);
            let index = IndexModel::builder().keys(keys).options(opts).build();
            self.col().create_index(index).await?;
        }

        Ok(())
    }

    /// Create compound/sparse/TTL indexes declared via `MongooseIndexes::indexes()`.
    /// Call this in addition to `ensure_indexes()` if your type implements `MongooseIndexes`.
    pub async fn ensure_custom_indexes(&self) -> Result<()>
    where
        T: MongooseIndexes,
    {
        for def in T::indexes() {
            self.col().create_index(def_to_model(&def)).await?;
        }
        Ok(())
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn parse_oid(s: &str) -> Result<ObjectId> {
    ObjectId::parse_str(s).map_err(|e| MongooseError::InvalidId(e.to_string()))
}

fn to_document<T: Serialize>(val: &T) -> Result<Document> {
    match to_bson(val).map_err(|e| MongooseError::Serialization(e.to_string()))? {
        bson::Bson::Document(d) => Ok(d),
        _ => Err(MongooseError::Serialization("Expected document".into())),
    }
}
