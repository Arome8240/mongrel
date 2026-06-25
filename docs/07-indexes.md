# Index Builder

Mongrel gives you two ways to create indexes:

1. **`#[field(unique)]`** — single-field unique indexes, created by `model.ensure_indexes()`.
2. **`MongooseIndexes` trait** — compound, sparse, TTL, text, and partial indexes, created by `model.ensure_custom_indexes()`.

Call both at startup:

```rust
users.ensure_indexes().await?;
users.ensure_custom_indexes().await?;
```

---

## Single-field unique indexes

Mark a field with `#[field(unique)]` in your schema:

```rust
#[field(unique)]
pub email: String,

#[field(unique)]
pub username: String,
```

Both get a `{ unique: true }` index created by `ensure_indexes()`. No additional code required.

---

## Custom indexes with `MongooseIndexes`

Implement `MongooseIndexes` on your schema struct to declare complex indexes:

```rust
use mongrel::index::{IndexBuilder, MongooseIndexes, IndexDef};

impl MongooseIndexes for User {
    fn indexes() -> Vec<IndexDef> {
        vec![
            // Compound index
            IndexBuilder::new()
                .field("last_name")
                .field("first_name")
                .name("full_name_idx")
                .build(),

            // Sparse — only indexes documents where "phone" exists
            IndexBuilder::new()
                .field("phone")
                .sparse()
                .name("phone_sparse")
                .build(),

            // TTL — MongoDB auto-deletes documents 1 hour after "expires_at"
            IndexBuilder::new()
                .field("expires_at")
                .ttl(3600)
                .name("session_expiry")
                .build(),

            // Full-text search across multiple fields
            IndexBuilder::new()
                .text("name")
                .text("bio")
                .name("user_text_search")
                .build(),

            // Compound unique — unique pair of (org_id, username)
            IndexBuilder::new()
                .field("org_id")
                .field("username")
                .unique()
                .name("org_username_unique")
                .build(),

            // Partial — unique email but only among active users
            IndexBuilder::new()
                .field("email")
                .unique()
                .partial_filter(bson::doc! { "active": true })
                .name("active_email_unique")
                .build(),
        ]
    }
}
```

Then at startup:

```rust
users.ensure_custom_indexes().await?;
```

---

## `IndexBuilder` reference

| Method | MongoDB option | Notes |
|---|---|---|
| `.field("name")` | ascending key (`1`) | Call multiple times for compound indexes |
| `.field_desc("name")` | descending key (`-1`) | |
| `.text("name")` | text index key | Call multiple times for multi-field text |
| `.unique()` | `unique: true` | |
| `.sparse()` | `sparse: true` | Only index docs where the key field exists |
| `.ttl(seconds)` | `expireAfterSeconds` | Auto-delete docs after N seconds |
| `.name("idx_name")` | `name` | Shown in Atlas / `db.collection.getIndexes()` |
| `.partial_filter(doc)` | `partialFilterExpression` | Only index docs matching the filter |
| `.build()` | — | Returns the `IndexDef` |

---

## When to call what

| Startup call | What it creates |
|---|---|
| `ensure_indexes()` | Unique indexes for all `#[field(unique)]` fields |
| `ensure_custom_indexes()` | All indexes returned by `MongooseIndexes::indexes()` |

Both methods are idempotent — MongoDB only creates an index if it doesn't already exist with the same key pattern, so it's safe to call them on every startup.
