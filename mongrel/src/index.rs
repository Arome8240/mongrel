use bson::Document;
use mongodb::{options::IndexOptions, IndexModel};

// ── IndexDef ──────────────────────────────────────────────────────────────────
// A description of a single index. Build one with IndexBuilder, then pass a
// slice of IndexDef to Model::create_indexes().

#[derive(Debug, Clone)]
pub struct IndexDef {
    pub keys: Document,
    pub unique: bool,
    pub sparse: bool,
    pub ttl_seconds: Option<u64>,
    pub name: Option<String>,
    pub partial_filter: Option<Document>,
}

// ── IndexBuilder ──────────────────────────────────────────────────────────────

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

    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    pub fn sparse(mut self) -> Self {
        self.sparse = true;
        self
    }

    /// TTL index: MongoDB will automatically delete documents after `seconds`.
    pub fn ttl(mut self, seconds: u64) -> Self {
        self.ttl_seconds = Some(seconds);
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn partial_filter(mut self, filter: Document) -> Self {
        self.partial_filter = Some(filter);
        self
    }

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

// ── MongooseIndexes trait ─────────────────────────────────────────────────────
// Implement this on your schema to declare extra indexes beyond `unique_fields`.

pub trait MongooseIndexes {
    fn indexes() -> Vec<IndexDef> { vec![] }
}
