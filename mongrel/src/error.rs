use thiserror::Error;

/// All errors that Mongrel operations can produce.
///
/// Most methods return [`Result<T>`] which is an alias for
/// `std::result::Result<T, MongooseError>`.
#[derive(Debug, Error)]
pub enum MongooseError {
    /// Wraps any error from the underlying `mongodb` async driver.
    #[error("MongoDB driver error: {0}")]
    Driver(#[from] mongodb::error::Error),

    /// A field constraint (`min_length`, `enum_values`, etc.) or a custom
    /// [`MongooseSchema::validate`](crate::schema::MongooseSchema::validate)
    /// check failed. The inner string is a human-readable description.
    #[error("Validation error: {0}")]
    Validation(String),

    /// An operation that expected a document to exist found nothing.
    /// Returned by, e.g., `find_one_and_upsert` when MongoDB returns no doc
    /// despite `upsert: true`.
    #[error("Document not found")]
    NotFound,

    /// BSON serialization or deserialization failed — typically a mismatch
    /// between the Rust struct layout and what MongoDB actually stored.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// A string passed as an ObjectId could not be parsed as a valid
    /// 24-hex-character BSON ObjectId.
    #[error("Invalid ObjectId: {0}")]
    InvalidId(String),
}

/// Shorthand `Result` type used throughout Mongrel.
///
/// Equivalent to `std::result::Result<T, MongooseError>`.
pub type Result<T> = std::result::Result<T, MongooseError>;
