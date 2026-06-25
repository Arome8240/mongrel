use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::error::Result;

// ── Hook event types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookEvent {
    PreSave,
    PostSave,
    PreDelete,
    PostDelete,
    PreValidate,
    PostValidate,
    PreFind,
    PostFind,
}

// ── BoxFuture alias ───────────────────────────────────────────────────────────

type BoxFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

// ── MiddlewareFn type ─────────────────────────────────────────────────────────
// A middleware function receives a mutable reference to the document (as Any)
// and returns a boxed future. We use Arc<dyn Fn> so the registry can clone it.

pub type MiddlewareFn<T> = Arc<dyn Fn(&mut T) -> BoxFuture<'_> + Send + Sync>;

// ── MiddlewareRegistry ────────────────────────────────────────────────────────
// Holds an ordered list of middleware per event. Register at startup;
// run all in order before/after each operation.

pub struct MiddlewareRegistry<T: Send + Sync + 'static> {
    pre_save: Vec<MiddlewareFn<T>>,
    post_save: Vec<MiddlewareFn<T>>,
    pre_delete: Vec<MiddlewareFn<T>>,
    post_delete: Vec<MiddlewareFn<T>>,
    pre_validate: Vec<MiddlewareFn<T>>,
    post_validate: Vec<MiddlewareFn<T>>,
}

impl<T: Send + Sync + 'static> Default for MiddlewareRegistry<T> {
    fn default() -> Self {
        Self {
            pre_save: Vec::new(),
            post_save: Vec::new(),
            pre_delete: Vec::new(),
            post_delete: Vec::new(),
            pre_validate: Vec::new(),
            post_validate: Vec::new(),
        }
    }
}

impl<T: Send + Sync + 'static> MiddlewareRegistry<T> {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Registration ──────────────────────────────────────────────────────────

    pub fn pre_save<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(&mut T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.pre_save.push(Arc::new(move |doc| Box::pin(f(doc))));
        self
    }

    pub fn post_save<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(&mut T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.post_save.push(Arc::new(move |doc| Box::pin(f(doc))));
        self
    }

    pub fn pre_delete<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(&mut T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.pre_delete.push(Arc::new(move |doc| Box::pin(f(doc))));
        self
    }

    pub fn post_delete<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(&mut T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.post_delete.push(Arc::new(move |doc| Box::pin(f(doc))));
        self
    }

    pub fn pre_validate<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(&mut T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.pre_validate.push(Arc::new(move |doc| Box::pin(f(doc))));
        self
    }

    pub fn post_validate<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(&mut T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.post_validate.push(Arc::new(move |doc| Box::pin(f(doc))));
        self
    }

    // ── Execution ─────────────────────────────────────────────────────────────

    fn stack_for(&self, event: HookEvent) -> &[MiddlewareFn<T>] {
        match event {
            HookEvent::PreSave => &self.pre_save,
            HookEvent::PostSave => &self.post_save,
            HookEvent::PreDelete => &self.pre_delete,
            HookEvent::PostDelete => &self.post_delete,
            HookEvent::PreValidate => &self.pre_validate,
            HookEvent::PostValidate => &self.post_validate,
            // Find hooks are registered separately (they don't mutate a T)
            HookEvent::PreFind | HookEvent::PostFind => &[],
        }
    }

    pub async fn run(&self, event: HookEvent, doc: &mut T) -> Result<()> {
        for mw in self.stack_for(event) {
            mw(doc).await?;
        }
        Ok(())
    }
}

// ── ModelWithMiddleware ───────────────────────────────────────────────────────
// Wraps Model<T> + a MiddlewareRegistry so the registry runs before/after
// the built-in Hooks trait. This is the "power user" alternative to Hooks.

use crate::{hooks::Hooks, model::Model, schema::MongooseSchema};
use serde::{de::DeserializeOwned, Serialize};

pub struct ModelWithMiddleware<T>
where
    T: MongooseSchema + Serialize + DeserializeOwned + Hooks + Unpin + Send + Sync + 'static,
{
    pub model: Model<T>,
    pub middleware: Arc<MiddlewareRegistry<T>>,
}

impl<T> ModelWithMiddleware<T>
where
    T: MongooseSchema + Serialize + DeserializeOwned + Hooks + Unpin + Send + Sync + 'static,
{
    pub fn new(model: Model<T>, middleware: MiddlewareRegistry<T>) -> Self {
        Self {
            model,
            middleware: Arc::new(middleware),
        }
    }

    pub async fn create(&self, mut doc: T) -> Result<T> {
        self.middleware.run(HookEvent::PreValidate, &mut doc).await?;
        self.middleware.run(HookEvent::PreSave, &mut doc).await?;
        let saved = self.model.create(doc).await?;
        Ok(saved)
    }

    pub async fn find_by_id_and_delete(&self, id: &str) -> Result<Option<T>> {
        if let Some(mut existing) = self.model.find_by_id(id).await? {
            self.middleware.run(HookEvent::PreDelete, &mut existing).await?;
        }
        let deleted = self.model.find_by_id_and_delete(id).await?;
        Ok(deleted)
    }
}
