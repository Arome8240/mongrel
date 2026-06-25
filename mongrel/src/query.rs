use bson::{doc, Bson, Document};
use mongodb::options::{FindOneOptions, FindOptions};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;

use crate::error::Result;
use crate::schema::MongooseSchema;
use futures_util::TryStreamExt;

/// Sort direction for [`QueryBuilder::sort`].
pub enum SortDir {
    /// Ascending order (1).
    Asc,
    /// Descending order (-1).
    Desc,
}

/// Chainable query builder returned by [`Model::find`](crate::model::Model::find).
///
/// Build up a filter with comparison operators, add sort/limit/skip/projection,
/// then call one of the terminal methods to execute.
///
/// # Example
///
/// ```rust,ignore
/// let results = users
///     .find()
///     .where_field("age").gte(18)
///     .where_field("role").eq("admin")
///     .sort("name", SortDir::Asc)
///     .limit(20)
///     .exec()
///     .await?;
/// ```
pub struct QueryBuilder<T: Send + Sync> {
    collection: Arc<mongodb::Collection<T>>,
    filter: Document,
    sort: Option<Document>,
    limit: Option<i64>,
    skip: Option<u64>,
    projection: Option<Document>,
    pending_field: Option<String>,
}

impl<T> QueryBuilder<T>
where
    T: MongooseSchema + DeserializeOwned + Serialize + Unpin + Send + Sync + 'static,
{
    pub(crate) fn new(collection: Arc<mongodb::Collection<T>>) -> Self {
        Self {
            collection,
            filter: doc! {},
            sort: None,
            limit: None,
            skip: None,
            projection: None,
            pending_field: None,
        }
    }

    pub(crate) fn with_filter(mut self, filter: Document) -> Self {
        self.filter = filter;
        self
    }

    // ── Field selection ───────────────────────────────────────────────────────

    /// Start a condition on `field`. Chain with a comparison operator:
    /// `.where_field("age").gte(18)`.
    pub fn where_field(mut self, field: impl Into<String>) -> Self {
        self.pending_field = Some(field.into());
        self
    }

    // ── Comparison operators ──────────────────────────────────────────────────

    fn add_condition(mut self, op: &str, value: impl Into<Bson>) -> Self {
        if let Some(field) = self.pending_field.take() {
            self.filter.insert(field, doc! { op: value.into() });
        }
        self
    }

    /// Field equals `value` exactly (`{ field: value }`).
    pub fn eq(mut self, value: impl Into<Bson>) -> Self {
        if let Some(field) = self.pending_field.take() {
            self.filter.insert(field, value.into());
        }
        self
    }

    /// Field is not equal to `value` (`$ne`).
    pub fn ne(self, value: impl Into<Bson>) -> Self { self.add_condition("$ne", value) }
    /// Field is strictly greater than `value` (`$gt`).
    pub fn gt(self, value: impl Into<Bson>) -> Self { self.add_condition("$gt", value) }
    /// Field is greater than or equal to `value` (`$gte`).
    pub fn gte(self, value: impl Into<Bson>) -> Self { self.add_condition("$gte", value) }
    /// Field is strictly less than `value` (`$lt`).
    pub fn lt(self, value: impl Into<Bson>) -> Self { self.add_condition("$lt", value) }
    /// Field is less than or equal to `value` (`$lte`).
    pub fn lte(self, value: impl Into<Bson>) -> Self { self.add_condition("$lte", value) }

    /// Field value is one of the given `values` (`$in`).
    pub fn in_list(mut self, values: impl IntoIterator<Item = impl Into<Bson>>) -> Self {
        if let Some(field) = self.pending_field.take() {
            let arr: Vec<Bson> = values.into_iter().map(Into::into).collect();
            self.filter.insert(field, doc! { "$in": arr });
        }
        self
    }

    /// Field value is **not** in the given list (`$nin`).
    pub fn nin(mut self, values: impl IntoIterator<Item = impl Into<Bson>>) -> Self {
        if let Some(field) = self.pending_field.take() {
            let arr: Vec<Bson> = values.into_iter().map(Into::into).collect();
            self.filter.insert(field, doc! { "$nin": arr });
        }
        self
    }

    /// Field matches a PCRE regular expression (`$regex`).
    /// `flags` is a string of regex option letters, e.g. `"i"` for
    /// case-insensitive.
    pub fn regex(mut self, pattern: impl Into<String>, flags: impl Into<String>) -> Self {
        if let Some(field) = self.pending_field.take() {
            self.filter.insert(
                field,
                doc! { "$regex": pattern.into(), "$options": flags.into() },
            );
        }
        self
    }

    /// Filter by field existence (`$exists`). Pass `true` to match documents
    /// where the field exists; `false` for documents where it is absent.
    pub fn field_exists(mut self, should_exist: bool) -> Self {
        if let Some(field) = self.pending_field.take() {
            self.filter.insert(field, doc! { "$exists": should_exist });
        }
        self
    }

    // ── Chainable query options ───────────────────────────────────────────────

    /// Maximum number of documents to return.
    pub fn limit(mut self, n: i64) -> Self {
        self.limit = Some(n);
        self
    }

    /// Number of documents to skip before returning results (for manual pagination).
    /// Prefer [`Model::paginate`](crate::model::Model::paginate) for page-based access.
    pub fn skip(mut self, n: u64) -> Self {
        self.skip = Some(n);
        self
    }

    /// Add a sort key. Call multiple times for multi-key sorts.
    pub fn sort(mut self, field: impl Into<String>, dir: SortDir) -> Self {
        let order: i32 = match dir {
            SortDir::Asc => 1,
            SortDir::Desc => -1,
        };
        self.sort
            .get_or_insert_with(Document::new)
            .insert(field.into(), order);
        self
    }

    /// Restrict which fields are returned (inclusion projection).
    /// Only the listed fields (plus `_id`) will be present in each document.
    pub fn select(mut self, fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut proj = Document::new();
        for f in fields {
            proj.insert(f.into(), 1);
        }
        self.projection = Some(proj);
        self
    }

    /// Replace the entire filter with a raw BSON `Document`.
    /// Use this escape hatch for operators not covered by the typed methods
    /// (e.g. `$or`, `$and`, `$elemMatch`).
    pub fn filter(mut self, raw: Document) -> Self {
        self.filter = raw;
        self
    }

    // ── Terminal operations ───────────────────────────────────────────────────

    /// Execute and return all matching documents deserialized into `T`.
    pub async fn exec(self) -> Result<Vec<T>> {
        let mut opts = FindOptions::default();
        opts.sort = self.sort;
        opts.limit = self.limit;
        opts.skip = self.skip;
        opts.projection = self.projection;

        let mut cursor = self
            .collection
            .find(self.filter)
            .with_options(opts)
            .await?;

        let mut results = Vec::new();
        while let Some(doc) = cursor.try_next().await? {
            results.push(doc);
        }
        Ok(results)
    }

    /// Execute and return the first matching document, or `None`.
    pub async fn exec_one(self) -> Result<Option<T>> {
        let mut opts = FindOneOptions::default();
        opts.sort = self.sort;
        opts.projection = self.projection;

        let result = self
            .collection
            .find_one(self.filter)
            .with_options(opts)
            .await?;
        Ok(result)
    }

    /// Return the number of documents matching the current filter.
    pub async fn count(self) -> Result<u64> {
        let n = self
            .collection
            .count_documents(self.filter)
            .await?;
        Ok(n)
    }

    /// Return `true` if at least one document matches the filter.
    pub async fn any(self) -> Result<bool> {
        Ok(self.count().await? > 0)
    }

    /// Execute and return raw BSON [`Document`]s — skips deserialization into `T`.
    ///
    /// Useful for performance-critical reads where you only need a subset of
    /// fields, or where the shape doesn't fit `T`.
    pub async fn lean(self) -> Result<Vec<Document>> {
        let raw_col = self.collection.clone_with_type::<Document>();

        let mut opts = FindOptions::default();
        opts.sort = self.sort;
        opts.limit = self.limit;
        opts.skip = self.skip;
        opts.projection = self.projection;

        let mut cursor = raw_col.find(self.filter).with_options(opts).await?;
        let mut results = Vec::new();
        while let Some(doc) = cursor.try_next().await? {
            results.push(doc);
        }
        Ok(results)
    }

    /// Like [`lean`](Self::lean) but returns only the first matching document.
    pub async fn lean_one(self) -> Result<Option<Document>> {
        let raw_col = self.collection.clone_with_type::<Document>();

        let mut opts = FindOneOptions::default();
        opts.sort = self.sort;
        opts.projection = self.projection;

        Ok(raw_col.find_one(self.filter).with_options(opts).await?)
    }
}
