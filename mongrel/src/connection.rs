use mongodb::{Client, Database};
use once_cell::sync::OnceCell;
use std::sync::Arc;

use crate::error::{MongooseError, Result};

static GLOBAL_DB: OnceCell<Arc<Database>> = OnceCell::new();

pub struct Mongrel;

impl Mongrel {
    /// Connect to MongoDB and store the connection globally.
    /// Call this once at app startup — analogous to `Mongrel::connect(uri)`.
    pub async fn connect(uri: &str, db_name: &str) -> Result<Arc<Database>> {
        let client = Client::with_uri_str(uri).await?;
        let db = Arc::new(client.database(db_name));

        GLOBAL_DB
            .set(Arc::clone(&db))
            .map_err(|_| MongooseError::Validation("Already connected".into()))?;

        Ok(db)
    }

    /// Get the global DB handle (must call `connect` first).
    pub fn db() -> Result<Arc<Database>> {
        GLOBAL_DB
            .get()
            .cloned()
            .ok_or_else(|| MongooseError::Validation("Not connected. Call Mongoose::connect() first.".into()))
    }
}
