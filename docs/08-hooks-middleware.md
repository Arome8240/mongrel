# Hooks & Middleware

Mongrel provides two complementary systems for running code around database operations:

- **`Hooks` trait** — implement directly on your schema struct; the simplest option for most cases.
- **`MiddlewareRegistry<T>`** — a stack of closures registered at construction time; ideal for cross-cutting concerns like logging, auditing, or encryption that shouldn't live in the schema itself.

---

## `Hooks` trait

Implement `Hooks` on your schema type and override only the methods you need. All methods default to a no-op `Ok(())`.

```rust
use async_trait::async_trait;
use mongrel::{hooks::Hooks, error::Result};

#[async_trait]
impl Hooks for User {
    /// Runs before field-level validation.
    async fn pre_validate(&self) -> Result<()> {
        Ok(())
    }

    /// Runs after validation passes, before the INSERT.
    async fn post_validate(&self) -> Result<()> {
        Ok(())
    }

    /// Mutates `self` just before writing to MongoDB.
    async fn pre_save(&mut self) -> Result<()> {
        self.email = self.email.to_lowercase().trim().to_string();
        Ok(())
    }

    /// Runs after a successful INSERT.
    async fn post_save(&self) -> Result<()> {
        println!("Saved user: {}", self.email);
        Ok(())
    }

    /// Runs before a delete — return Err to abort.
    async fn pre_delete(&self) -> Result<()> {
        if self.role == "admin" {
            return Err(mongrel::MongooseError::Validation(
                "Cannot delete admin users".into()
            ));
        }
        Ok(())
    }

    /// Runs after a successful delete.
    async fn post_delete(&self) -> Result<()> {
        Ok(())
    }
}
```

### Execution order on `create`

```
pre_validate
    → validate (field attribute checks)
        → post_validate
            → pre_save
                → INSERT into MongoDB
                    → post_save
```

Returning `Err` from any hook aborts the operation and propagates the error to the caller.

---

## `MiddlewareRegistry<T>`

For cross-cutting concerns — logging, auditing, rate-limiting, encryption — use `MiddlewareRegistry<T>` instead of polluting your schema's `Hooks` impl.

### Creating a registry

```rust
use mongrel::{MiddlewareRegistry, ModelWithMiddleware};

let registry = MiddlewareRegistry::new()
    .pre_save(|doc: &mut User| async move {
        println!("[audit] saving: {}", doc.email);
        Ok(())
    })
    .pre_save(|doc: &mut User| async move {
        // Second pre_save runs after the first
        doc.email = doc.email.to_lowercase();
        Ok(())
    })
    .post_delete(|doc: &mut User| async move {
        println!("[audit] deleted: {}", doc.email);
        Ok(())
    });
```

### Wrapping a model

```rust
let users_mw = ModelWithMiddleware::new(
    UserModel::new(Arc::clone(&db)),
    registry,
);

// Use just like a regular model — middleware runs automatically
let user = users_mw.create(User { /* ... */ }).await?;
users_mw.find_by_id_and_delete(&id).await?;
```

### Supported events

| Method | Event |
|---|---|
| `.pre_save(f)` | Before INSERT |
| `.post_save(f)` | After INSERT |
| `.pre_delete(f)` | Before DELETE |
| `.post_delete(f)` | After DELETE |
| `.pre_validate(f)` | Before validation |
| `.post_validate(f)` | After validation |

Each event has an **ordered stack** — functions run in registration order. All registered functions for an event must succeed (return `Ok(())`) for the operation to proceed.

### Middleware vs Hooks

Use `Hooks` for logic that belongs to the domain model — password hashing, email normalization, business-rule enforcement.

Use `MiddlewareRegistry` for infrastructure concerns — structured logging, audit trails, metrics, that apply to many models or need to be swappable at runtime.

Both systems compose: middleware runs **before** the schema's `Hooks` methods.

---

## Full example: audit + normalization

```rust
// schema.rs — domain logic in Hooks
#[async_trait]
impl Hooks for User {
    async fn pre_save(&mut self) -> Result<()> {
        if self.password.len() < 8 {
            return Err(MongooseError::Validation("Password too short".into()));
        }
        self.password = hash_password(&self.password);
        Ok(())
    }
}

// main.rs — infrastructure logic in middleware
let registry = MiddlewareRegistry::new()
    .pre_save(|u: &mut User| async move {
        tracing::info!(email = %u.email, "pre_save");
        Ok(())
    })
    .post_save(|u: &mut User| async move {
        metrics::increment_counter!("users.created");
        Ok(())
    });

let users = ModelWithMiddleware::new(UserModel::new(Arc::clone(&db)), registry);
```
