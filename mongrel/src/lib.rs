pub mod aggregation;
pub mod connection;
pub mod error;
pub mod hooks;
pub mod index;
pub mod middleware;
pub mod model;
pub mod pagination;
pub mod populate;
pub mod query;
pub mod schema;
pub mod virtual_fields;

pub use connection::Mongrel;
pub use error::{MongooseError, Result};
pub use model::Model;
pub use mongrel_macros::{Model, Schema};
pub use populate::{resolve_opt_ref, resolve_ref, Populate, Ref};
pub use aggregation::AggregationPipeline;
pub use index::{IndexBuilder, IndexDef, MongooseIndexes};
pub use middleware::{HookEvent, MiddlewareRegistry, ModelWithMiddleware};
pub use pagination::{PaginateBuilder, PaginatedResult};
pub use virtual_fields::{Virtuals, WithVirtuals};
pub use query::SortDir;

// Re-export bson/serde for convenience
pub use bson;
pub use mongodb;
pub use serde;
