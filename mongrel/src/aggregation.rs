use bson::{doc, Document};
use futures_util::TryStreamExt;
use serde::de::DeserializeOwned;

use crate::error::Result;

// ── AggregationPipeline ───────────────────────────────────────────────────────

pub struct AggregationPipeline<T: Send + Sync> {
    collection: mongodb::Collection<T>,
    stages: Vec<Document>,
}

impl<T> AggregationPipeline<T>
where
    T: DeserializeOwned + Unpin + Send + Sync + 'static,
{
    pub(crate) fn new(collection: mongodb::Collection<T>) -> Self {
        Self {
            collection,
            stages: Vec::new(),
        }
    }

    // ── Stages ────────────────────────────────────────────────────────────────

    pub fn match_stage(mut self, filter: Document) -> Self {
        self.stages.push(doc! { "$match": filter });
        self
    }

    pub fn sort(mut self, sort: Document) -> Self {
        self.stages.push(doc! { "$sort": sort });
        self
    }

    pub fn limit(mut self, n: i64) -> Self {
        self.stages.push(doc! { "$limit": n });
        self
    }

    pub fn skip(mut self, n: i64) -> Self {
        self.stages.push(doc! { "$skip": n });
        self
    }

    pub fn project(mut self, projection: Document) -> Self {
        self.stages.push(doc! { "$project": projection });
        self
    }

    pub fn unwind(mut self, path: impl Into<String>) -> Self {
        self.stages.push(doc! { "$unwind": path.into() });
        self
    }

    pub fn unwind_opts(
        mut self,
        path: impl Into<String>,
        preserve_null: bool,
    ) -> Self {
        self.stages.push(doc! {
            "$unwind": {
                "path": path.into(),
                "preserveNullAndEmptyArrays": preserve_null,
            }
        });
        self
    }

    pub fn group(mut self, group: Document) -> Self {
        self.stages.push(doc! { "$group": group });
        self
    }

    pub fn lookup(
        mut self,
        from: impl Into<String>,
        local_field: impl Into<String>,
        foreign_field: impl Into<String>,
        as_field: impl Into<String>,
    ) -> Self {
        self.stages.push(doc! {
            "$lookup": {
                "from": from.into(),
                "localField": local_field.into(),
                "foreignField": foreign_field.into(),
                "as": as_field.into(),
            }
        });
        self
    }

    pub fn add_fields(mut self, fields: Document) -> Self {
        self.stages.push(doc! { "$addFields": fields });
        self
    }

    pub fn replace_root(mut self, new_root: impl Into<String>) -> Self {
        self.stages.push(doc! { "$replaceRoot": { "newRoot": new_root.into() } });
        self
    }

    pub fn count(mut self, field: impl Into<String>) -> Self {
        self.stages.push(doc! { "$count": field.into() });
        self
    }

    /// Escape hatch: push any raw pipeline stage document.
    pub fn raw_stage(mut self, stage: Document) -> Self {
        self.stages.push(stage);
        self
    }

    // ── Execute ───────────────────────────────────────────────────────────────

    pub async fn exec(self) -> Result<Vec<T>> {
        let mut cursor = self.collection.aggregate(self.stages).await?;
        let mut results = Vec::new();
        while let Some(raw) = cursor.try_next().await? {
            let item: T = bson::from_document(raw)
                .map_err(|e| crate::error::MongooseError::Serialization(e.to_string()))?;
            results.push(item);
        }
        Ok(results)
    }

    /// Execute returning raw Documents (no deserialization).
    pub async fn exec_raw(self) -> Result<Vec<Document>> {
        let raw_col: mongodb::Collection<Document> = self
            .collection
            .clone_with_type::<Document>();
        let mut cursor = raw_col.aggregate(self.stages).await?;
        let mut results = Vec::new();
        while let Some(doc) = cursor.try_next().await? {
            results.push(doc);
        }
        Ok(results)
    }
}
