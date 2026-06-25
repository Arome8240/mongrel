use async_trait::async_trait;
use crate::error::Result;

/// Mongoose-style lifecycle hooks. Override any method you need.
#[async_trait]
pub trait Hooks: Sized + Send + Sync {
    async fn pre_save(&mut self) -> Result<()> { Ok(()) }
    async fn post_save(&self) -> Result<()> { Ok(()) }
    async fn pre_delete(&self) -> Result<()> { Ok(()) }
    async fn post_delete(&self) -> Result<()> { Ok(()) }
    async fn pre_validate(&self) -> Result<()> { Ok(()) }
    async fn post_validate(&self) -> Result<()> { Ok(()) }
}
