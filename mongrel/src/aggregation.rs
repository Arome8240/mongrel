use bson::{doc, Document};
use futures_util::TryStreamExt;
use serde::de::DeserializeOwned;

use crate::error::Result;

/// A fluent builder for MongoDB aggregation pipelines, returned by
/// [`Model::aggregate`](crate::model::Model::aggregate).
///
/// Chain stage methods in the order you want them executed, then call
/// [`exec`](Self::exec) (deserializes into `T`) or
/// [`exec_raw`](Self::exec_raw) (returns raw [`Document`]s).
///
/// # Example
///
/// ```rust,ignore
/// let totals = orders
///     .aggregate()
///     .match_stage(doc! { "status": "completed" })
///     .group(doc! { "_id": "$user_id", "total": { "$sum": "$amount" } })
///     .sort(doc! { "total": -1 })
///     .exec_raw()
///     .await?;
/// ```
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

    /// `$match` — filter documents. Equivalent to a `WHERE` clause.
    pub fn match_stage(mut self, filter: Document) -> Self {
        self.stages.push(doc! { "$match": filter });
        self
    }

    /// `$sort` — order results. Use `doc! { "field": 1 }` for ascending,
    /// `doc! { "field": -1 }` for descending.
    pub fn sort(mut self, sort: Document) -> Self {
        self.stages.push(doc! { "$sort": sort });
        self
    }

    /// `$limit` — keep only the first `n` documents.
    pub fn limit(mut self, n: i64) -> Self {
        self.stages.push(doc! { "$limit": n });
        self
    }

    /// `$skip` — discard the first `n` documents.
    pub fn skip(mut self, n: i64) -> Self {
        self.stages.push(doc! { "$skip": n });
        self
    }

    /// `$project` — reshape each document: include, exclude, or compute fields.
    pub fn project(mut self, projection: Document) -> Self {
        self.stages.push(doc! { "$project": projection });
        self
    }

    /// `$unwind` — deconstruct an array field into one document per element.
    pub fn unwind(mut self, path: impl Into<String>) -> Self {
        self.stages.push(doc! { "$unwind": path.into() });
        self
    }

    /// `$unwind` with options. Set `preserve_null` to `true` to keep documents
    /// where the array is `null` or missing.
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

    /// `$group` — group documents by `_id` and apply accumulators
    /// (`$sum`, `$avg`, `$push`, etc.).
    pub fn group(mut self, group: Document) -> Self {
        self.stages.push(doc! { "$group": group });
        self
    }

    /// `$lookup` — left-outer join with another collection.
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

    /// `$addFields` — add or overwrite fields in each document without
    /// removing existing ones.
    pub fn add_fields(mut self, fields: Document) -> Self {
        self.stages.push(doc! { "$addFields": fields });
        self
    }

    /// `$replaceRoot` — promote a nested document to the top level.
    pub fn replace_root(mut self, new_root: impl Into<String>) -> Self {
        self.stages.push(doc! { "$replaceRoot": { "newRoot": new_root.into() } });
        self
    }

    /// `$count` — output a single document with the count stored in `field`.
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

    /// Execute the pipeline and deserialize results into `T`.
    /// Use [`exec_raw`](Self::exec_raw) if the output shape doesn't match `T`.
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
