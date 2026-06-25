use bson::Document;
use futures_util::TryStreamExt;
use mongodb::options::FindOptions;
use serde::{de::DeserializeOwned, Serialize};

use crate::error::Result;
use crate::schema::MongooseSchema;

/// The result of a paginated query, returned by
/// [`PaginateBuilder::exec`].
#[derive(Debug)]
pub struct PaginatedResult<T> {
    /// Documents for the requested page.
    pub docs: Vec<T>,
    /// Total number of documents matching the filter (across all pages).
    pub total: u64,
    /// Current page number (1-based).
    pub page: u64,
    /// Maximum documents per page as requested.
    pub per_page: u64,
    /// `ceil(total / per_page)`.
    pub total_pages: u64,
    /// `true` when there is a page after this one.
    pub has_next: bool,
    /// `true` when there is a page before this one.
    pub has_prev: bool,
}

impl<T> PaginatedResult<T> {
    fn new(docs: Vec<T>, total: u64, page: u64, per_page: u64) -> Self {
        let total_pages = if per_page == 0 {
            0
        } else {
            (total + per_page - 1) / per_page
        };
        PaginatedResult {
            docs,
            total,
            page,
            per_page,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}

/// Chainable builder for paginated queries, returned by
/// [`Model::paginate`](crate::model::Model::paginate).
///
/// # Example
///
/// ```rust,ignore
/// let result = users
///     .paginate(2, 10)
///     .filter(doc! { "active": true })
///     .sort(doc! { "name": 1 })
///     .exec()
///     .await?;
///
/// println!("{}/{} pages", result.page, result.total_pages);
/// ```
pub struct PaginateBuilder<T: Send + Sync> {
    collection: mongodb::Collection<T>,
    filter: Document,
    sort: Option<Document>,
    page: u64,
    per_page: u64,
    projection: Option<Document>,
}

impl<T> PaginateBuilder<T>
where
    T: MongooseSchema + DeserializeOwned + Serialize + Unpin + Send + Sync + 'static,
{
    pub(crate) fn new(collection: mongodb::Collection<T>, page: u64, per_page: u64) -> Self {
        Self {
            collection,
            filter: bson::doc! {},
            sort: None,
            page: page.max(1),
            per_page: per_page.max(1),
            projection: None,
        }
    }

    pub fn filter(mut self, filter: Document) -> Self {
        self.filter = filter;
        self
    }

    pub fn sort(mut self, sort: Document) -> Self {
        self.sort = Some(sort);
        self
    }

    pub fn select(mut self, fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut proj = Document::new();
        for f in fields {
            proj.insert(f.into(), 1);
        }
        self.projection = Some(proj);
        self
    }

    pub async fn exec(self) -> Result<PaginatedResult<T>> {
        let skip = (self.page - 1) * self.per_page;

        let total = self
            .collection
            .count_documents(self.filter.clone())
            .await?;

        let mut opts = FindOptions::default();
        opts.skip = Some(skip);
        opts.limit = Some(self.per_page as i64);
        opts.sort = self.sort;
        opts.projection = self.projection;

        let mut cursor = self
            .collection
            .find(self.filter)
            .with_options(opts)
            .await?;

        let mut docs = Vec::new();
        while let Some(doc) = cursor.try_next().await? {
            docs.push(doc);
        }

        Ok(PaginatedResult::new(docs, total, self.page, self.per_page))
    }
}
