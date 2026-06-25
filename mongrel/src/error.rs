use thiserror::Error;

#[derive(Debug, Error)]
pub enum MongooseError {
    #[error("MongoDB driver error: {0}")]
    Driver(#[from] mongodb::error::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Document not found")]
    NotFound,

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid ObjectId: {0}")]
    InvalidId(String),
}

pub type Result<T> = std::result::Result<T, MongooseError>;
