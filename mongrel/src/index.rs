use bson::Document;
use mongodb::{options::IndexOptions, IndexModel};

/// A fully-described MongoDB index, produced by [`IndexBuilder::build`].
///
/// Pass a `Vec<IndexDef>` from [`MongooseIndexes::indexes`] to have
/// [`Model::ensure_custom_indexes`](crate::model::Model::ensure_custom_indexes)
/// create them at startup.
#[derive(Debug, Clone)]
pub struct IndexDef {
    pub keys: Document,
    pub unique: bool,
    pub sparse: bool,
    pub ttl_seconds: Option<u64>,
    pub name: Option<String>,
    pub partial_filter: Option<Document>,
}

/// Fluent builder for [`IndexDef`].
///
/// # Example
///
/// ```rust,ignore
/// // Compound unique index
/// IndexBuilder::new()
///     .field("email")
///     .unique()
///     .build();
///
/// // TTL index — expire sessions after 1 hour
/// IndexBuilder::new()
///     .field("created_at")
///     .ttl(3600)
///     .name("session_ttl")
///     .build();
///
/// // Full-text search across multiple fields
/// IndexBuilder::new()
///     .text("title")
///     .text("body")
///     .name("post_text_search")
///     .build();
/// ```
pub struct IndexBuilder {
    keys: Document,
    unique: bool,
    sparse: bool,
    ttl_seconds: Option<u64>,
    name: Option<String>,
    partial_filter: Option<Document>,
}

impl IndexBuilder {
    pub fn new() -> Self {
        Self {
            keys: Document::new(),
            unique: false,
            sparse: false,
            ttl_seconds: None,
            name: None,
            partial_filter: None,
        }
    }

    /// Add a field to the index key (ascending).
    pub fn field(mut self, name: impl Into<String>) -> Self {
        self.keys.insert(name.into(), 1i32);
        self
    }

    /// Add a field to the index key (descending).
    pub fn field_desc(mut self, name: impl Into<String>) -> Self {
        self.keys.insert(name.into(), -1i32);
        self
    }

    /// Add a text index field.
    pub fn text(mut self, name: impl Into<String>) -> Self {
        self.keys.insert(name.into(), "text");
        self
    }

    /// Enforce uniqueness — no two documents may share the same key value(s).
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    /// Only index documents where the key field(s) exist.
    pub fn sparse(mut self) -> Self {
        self.sparse = true;
        self
    }

    /// TTL index: MongoDB will automatically delete documents after `seconds`.
    pub fn ttl(mut self, seconds: u64) -> Self {
        self.ttl_seconds = Some(seconds);
        self
    }

    /// Give the index a custom name (shown in MongoDB shell / Atlas).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Only index documents matching `filter` — a partial index.
    /// Reduces index size and write overhead when only a subset of documents
    /// need to be indexed (e.g., only active users).
    pub fn partial_filter(mut self, filter: Document) -> Self {
        self.partial_filter = Some(filter);
        self
    }

    /// Finalise the builder and return an [`IndexDef`].
    pub fn build(self) -> IndexDef {
        IndexDef {
            keys: self.keys,
            unique: self.unique,
            sparse: self.sparse,
            ttl_seconds: self.ttl_seconds,
            name: self.name,
            partial_filter: self.partial_filter,
        }
    }
}

impl Default for IndexBuilder {
    fn default() -> Self { Self::new() }
}

// ── Helpers to convert IndexDef → mongodb::IndexModel ────────────────────────

pub(crate) fn def_to_model(def: &IndexDef) -> IndexModel {
    let mut opts = IndexOptions::default();
    if def.unique { opts.unique = Some(true); }
    if def.sparse { opts.sparse = Some(true); }
    if let Some(secs) = def.ttl_seconds {
        opts.expire_after = Some(std::time::Duration::from_secs(secs));
    }
    if let Some(ref n) = def.name { opts.name = Some(n.clone()); }
    if let Some(ref pf) = def.partial_filter {
        opts.partial_filter_expression = Some(pf.clone());
    }

    IndexModel::builder()
        .keys(def.keys.clone())
        .options(opts)
        .build()
}

/// Declare compound, sparse, TTL, text, or partial indexes on a schema type.
///
/// Implement this on your struct and call
/// [`Model::ensure_custom_indexes`](crate::model::Model::ensure_custom_indexes)
/// at startup. Fields marked `#[field(unique)]` are handled separately by
/// [`Model::ensure_indexes`](crate::model::Model::ensure_indexes).
///
/// # Example
///
/// ```rust,ignore
/// impl MongooseIndexes for User {
///     fn indexes() -> Vec<IndexDef> {
///         vec![
///             IndexBuilder::new().field("last_name").field("first_name").build(),
///             IndexBuilder::new().field("phone").sparse().build(),
///         ]
///     }
/// }
/// ```
pub trait MongooseIndexes {
    /// Return the list of custom indexes to create for this collection.
    fn indexes() -> Vec<IndexDef> { vec![] }
}
