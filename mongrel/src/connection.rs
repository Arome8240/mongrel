use mongodb::{Client, Database};
use once_cell::sync::OnceCell;
use std::sync::Arc;

use crate::error::{MongooseError, Result};

static GLOBAL_DB: OnceCell<Arc<Database>> = OnceCell::new();

/// Entry point for establishing a MongoDB connection.
///
/// Call [`Mongrel::connect`] once at application startup. The returned
/// `Arc<Database>` is also stored in a process-global slot so you can
/// retrieve it later with [`Mongrel::db`] without threading it through
/// every function.
///
/// # Example
///
/// ```rust,ignore
/// let db = Mongrel::connect("mongodb://localhost:27017", "myapp").await?;
/// let users = UserModel::new(Arc::clone(&db));
/// ```
pub struct Mongrel;

impl Mongrel {
    /// Connect to MongoDB and cache the handle globally.
    ///
    /// Panics (via `Err`) if called more than once in the same process.
    /// Pass the returned `Arc<Database>` to every `XModel::new()` call.
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
