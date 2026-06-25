use async_trait::async_trait;
use crate::error::Result;

/// Async lifecycle hooks that run around database operations.
///
/// Implement this trait on your schema struct and override the methods you
/// need. All methods have default no-op implementations so you only pay for
/// what you use.
///
/// # Execution order during `Model::create`
///
/// ```text
/// pre_validate → validate (field attributes) → post_validate
/// → pre_save → INSERT into MongoDB → post_save
/// ```
///
/// # Example
///
/// ```rust,ignore
/// use async_trait::async_trait;
/// use mongrel::{hooks::Hooks, error::Result};
///
/// #[async_trait]
/// impl Hooks for User {
///     async fn pre_save(&mut self) -> Result<()> {
///         self.email = self.email.to_lowercase();
///         Ok(())
///     }
///
///     async fn post_save(&self) -> Result<()> {
///         println!("Saved user: {}", self.email);
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Hooks: Sized + Send + Sync {
    /// Runs before field-level validation. Mutates `self`.
    async fn pre_validate(&self) -> Result<()> { Ok(()) }
    /// Runs after validation passes, before the document is saved.
    async fn post_validate(&self) -> Result<()> { Ok(()) }
    /// Runs immediately before the INSERT / the caller's write. Mutates `self`
    /// — useful for normalizing data, hashing passwords, etc.
    async fn pre_save(&mut self) -> Result<()> { Ok(()) }
    /// Runs after a successful INSERT. Use for side-effects like sending emails
    /// or emitting events.
    async fn post_save(&self) -> Result<()> { Ok(()) }
    /// Runs before a document is deleted. Returning an error aborts the delete.
    async fn pre_delete(&self) -> Result<()> { Ok(()) }
    /// Runs after a successful delete.
    async fn post_delete(&self) -> Result<()> { Ok(()) }
}
