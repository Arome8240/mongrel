# Mongrel

A Mongoose-style MongoDB ODM for Rust — schema definitions, lifecycle hooks, a chainable query builder, population, aggregation, pagination, and middleware, all driven by derive macros.

```toml
[dependencies]
mongrel = "0.1"
```

---

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
- [Schema Definition](#schema-definition)
- [Model Operations](#model-operations)
- [Query Builder](#query-builder)
- [Population (Refs)](#population-refs)
- [Virtuals](#virtuals)
- [Aggregation Pipeline](#aggregation-pipeline)
- [Pagination](#pagination)
- [Lifecycle Hooks](#lifecycle-hooks)
- [Middleware Chaining](#middleware-chaining)
- [Index Builder](#index-builder)
- [Error Handling](#error-handling)
- [Project Structure](#project-structure)

---

## Features

- **`#[derive(Schema)]`** — declare your collection, field constraints, timestamps, and unique indexes on a plain Rust struct
- **`#[derive(Model)]`** — auto-generates a `XModel` handle with full CRUD, query, and aggregation methods
- **Chainable query builder** — `where_field().gte().sort().limit().exec()` — no raw BSON required for common queries
- **Typed `Ref<T>`** — store ObjectId references, populate them into full documents on demand
- **Virtuals** — computed fields that never touch MongoDB, serialized alongside real fields for API responses
- **Aggregation pipeline** — fluent builder for `$match`, `$group`, `$lookup`, `$project`, `$sort`, `$limit`, `$unwind`, and more
- **Pagination** — `model.paginate(page, per_page)` returns total, total_pages, has_next, has_prev alongside docs
- **Lifecycle hooks** — `pre_save`, `post_save`, `pre_delete`, `post_delete`, `pre_validate`, `post_validate`
- **Middleware chaining** — `MiddlewareRegistry<T>` runs an ordered async stack of functions per event
- **Index builder** — compound, sparse, TTL, text, and partial-filter indexes via `IndexBuilder`
- **Lean queries** — `.lean()` skips deserialization and returns raw `Document` for performance-critical paths
- **Async-first** — built on Tokio and the official `mongodb` async driver

---

## Quick Start

```rust
use async_trait::async_trait;
use mongrel::{bson::doc, hooks::Hooks, error::Result, Model, Mongrel, Schema, SortDir};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Schema, Model)]
#[schema(collection = "users", timestamps)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,

    #[field(required, min_length = 2, max_length = 50)]
    pub name: String,

    #[field(required, unique)]
    pub email: String,

    #[field(enum_values = "admin, user, moderator")]
    pub role: String,

    pub age: Option<i32>,
}

#[async_trait]
impl Hooks for User {}

#[tokio::main]
async fn main() -> Result<()> {
    let db = Mongrel::connect("mongodb://localhost:27017", "myapp").await?;
    let users = UserModel::new(db);

    users.ensure_indexes().await?;

    // Create
    let user = users.create(User {
        id: None,
        name: "Alice".into(),
        email: "alice@example.com".into(),
        role: "admin".into(),
        age: Some(30),
    }).await?;

    // Query
    let adults = users
        .find()
        .where_field("age").gte(18)
        .sort("name", SortDir::Asc)
        .limit(10)
        .exec()
        .await?;

    // Update
    users.find_by_id_and_update(
        &user.id.unwrap().to_hex(),
        doc! { "$set": { "role": "moderator" } },
    ).await?;

    // Delete
    users.find_by_id_and_delete(&user.id.unwrap().to_hex()).await?;

    Ok(())
}
```

---

## Schema Definition

Annotate any named struct with `#[derive(Schema, Model)]`.

### `#[schema(...)]` — struct-level options

| Attribute | Type | Description |
|---|---|---|
| `collection = "name"` | string | MongoDB collection name. Defaults to snake_case plural of the struct name. |
| `timestamps` | flag | Auto-injects `created_at` and `updated_at` (`bson::DateTime`) on create/update. |

```rust
#[derive(Serialize, Deserialize, Schema, Model)]
#[schema(collection = "blog_posts", timestamps)]
pub struct BlogPost { /* ... */ }
```

### `#[field(...)]` — field-level options

| Attribute | Type | Description |
|---|---|---|
| `required` | flag | Marks the field as required (enforced at validation time). |
| `unique` | flag | Creates a unique index on this field via `ensure_indexes()`. |
| `min_length = N` | usize | Minimum character length for `String` / `Option<String>` fields. |
| `max_length = N` | usize | Maximum character length for `String` / `Option<String>` fields. |
| `enum_values = "a, b, c"` | string | Comma-separated list of allowed string values. |
| `rename = "mongo_name"` | string | Override the field name stored in MongoDB. |

```rust
#[derive(Serialize, Deserialize, Schema, Model)]
#[schema(collection = "products")]
pub struct Product {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,

    #[field(required, min_length = 3, max_length = 120)]
    pub name: String,

    #[field(enum_values = "draft, published, archived")]
    pub status: String,

    #[field(unique)]
    pub sku: String,
}
```

### Custom validation

Override `validate()` directly on `MongooseSchema` for logic that can't be expressed with attributes:

```rust
use mongrel::schema::MongooseSchema;
use mongrel::error::MongooseError;

impl MongooseSchema for Product {
    fn collection_name() -> &'static str { "products" }

    fn validate(&self) -> std::result::Result<(), MongooseError> {
        if self.name.contains('<') {
            return Err(MongooseError::Validation("name must not contain HTML".into()));
        }
        Ok(())
    }
}
```

---

## Model Operations

`#[derive(Model)]` generates a `{Name}Model` struct. Construct it with a shared `Arc<Database>`:

```rust
let db = Mongrel::connect("mongodb://localhost:27017", "myapp").await?;
let users = UserModel::new(Arc::clone(&db));
```

### Create

```rust
let user = users.create(User { id: None, name: "Bob".into(), /* ... */ }).await?;
// Runs: pre_validate → validate → post_validate → pre_save → insert → post_save
```

### Find

```rust
// All documents
let all = users.find().exec().await?;

// By id
let one = users.find_by_id("64b1f...").await?;  // Option<User>

// Raw filter
let admins = users.find_many(doc! { "role": "admin" }).exec().await?;

// Single doc
let first = users.find_one_where(doc! { "email": "x@y.com" }).exec_one().await?;
```

### Update

```rust
// By id — returns the updated document
let updated = users
    .find_by_id_and_update("64b1f...", doc! { "$set": { "name": "New Name" } })
    .await?;

// By filter
let updated = users
    .find_one_and_update(doc! { "email": "x@y.com" }, doc! { "$inc": { "age": 1 } })
    .await?;

// Many
let modified_count = users
    .update_many(doc! { "role": "user" }, doc! { "$set": { "active": true } })
    .await?;
```

### Delete

```rust
let deleted = users.find_by_id_and_delete("64b1f...").await?;      // Option<User>
let deleted = users.find_one_and_delete(doc! { "email": "x" }).await?;
let count   = users.delete_many(doc! { "active": false }).await?;
```

### Upsert

```rust
let doc = users
    .find_one_and_upsert(
        doc! { "email": "x@y.com" },
        doc! { "$setOnInsert": { "name": "X", "role": "user" } },
    )
    .await?;
```

### Count

```rust
let total = users.count_documents(doc! {}).await?;
let admins = users.count_documents(doc! { "role": "admin" }).await?;
```

---

## Query Builder

`model.find()` returns a `QueryBuilder<T>` with a fluent API. All filters are composed with AND semantics.

### Comparison operators

```rust
users.find()
    .where_field("age").gte(18)
    .where_field("age").lte(65)
    .where_field("role").eq("admin")
    .where_field("score").ne(0)
    .where_field("rank").gt(10)
    .where_field("rank").lt(100)
    .exec().await?;
```

### Array / set operators

```rust
users.find()
    .where_field("role").in_list(["admin", "moderator"])
    .where_field("status").nin(["banned", "deleted"])
    .exec().await?;
```

### String pattern

```rust
users.find()
    .where_field("name").regex("^alice", "i")   // case-insensitive
    .exec().await?;
```

### Existence

```rust
users.find()
    .where_field("phone").field_exists(true)
    .exec().await?;
```

### Sorting, limiting, projection

```rust
users.find()
    .sort("created_at", SortDir::Desc)
    .sort("name", SortDir::Asc)
    .limit(20)
    .skip(40)
    .select(["name", "email", "role"])
    .exec().await?;
```

### Terminal methods

| Method | Returns | Description |
|---|---|---|
| `.exec()` | `Vec<T>` | All matching documents |
| `.exec_one()` | `Option<T>` | First matching document |
| `.count()` | `u64` | Number of matching documents |
| `.any()` | `bool` | `true` if at least one match exists |
| `.lean()` | `Vec<Document>` | Raw BSON — skips deserialization |
| `.lean_one()` | `Option<Document>` | Raw BSON, single document |

### Raw filter escape hatch

```rust
users.find()
    .filter(doc! { "$or": [{ "role": "admin" }, { "age": { "$gt": 60 } }] })
    .exec().await?;
```

---

## Population (Refs)

Use `Ref<T>` to store a typed reference to another collection's document.

### Define a reference field

```rust
use mongrel::Ref;

#[derive(Serialize, Deserialize, Schema, Model)]
#[schema(collection = "posts")]
pub struct Post {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,
    pub title: String,
    pub author: Ref<User>,          // stored as ObjectId in MongoDB
}
```

### Populate on demand

Implement the `Populate` trait on your struct:

```rust
use mongrel::populate::{Populate, resolve_ref};
use async_trait::async_trait;

#[async_trait]
impl Populate for Post {
    async fn populate(mut self, db: Arc<mongodb::Database>) -> Result<Self> {
        self.author = resolve_ref(self.author, Arc::clone(&db)).await?;
        Ok(self)
    }
}
```

Then call it after fetching:

```rust
let post = posts.find_by_id("64b1f...").await?.unwrap();
let populated = post.populate(Arc::clone(&db)).await?;

match &populated.author {
    Ref::Populated(user) => println!("Author: {}", user.name),
    Ref::Id(id)          => println!("Unpopulated id: {}", id),
}
```

### `Ref<T>` API

| Method | Description |
|---|---|
| `.id()` | Returns `Option<&ObjectId>` if unpopulated |
| `.populated()` | Returns `Option<&T>` if populated |
| `.is_populated()` | `true` if the document has been loaded |

`Ref<T>` serializes and deserializes as a plain `ObjectId`, so it stores correctly in MongoDB regardless of population state.

---

## Virtuals

Computed properties derived at runtime — never written to MongoDB.

```rust
use mongrel::virtual_fields::Virtuals;

pub struct User {
    pub first_name: String,
    pub last_name: String,
    pub age: i32,
}

impl Virtuals for User {}

impl User {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    pub fn is_adult(&self) -> bool {
        self.age >= 18
    }
}
```

### Serialize virtuals alongside real fields

Use `WithVirtuals<T>` to merge computed values into a JSON response:

```rust
use mongrel::WithVirtuals;

let response = WithVirtuals::new(user)
    .add("full_name", user.full_name())
    .add("is_adult", user.is_adult());

let json = serde_json::to_string(&response)?;
// { "first_name": "Alice", "last_name": "Smith", ..., "full_name": "Alice Smith", "is_adult": true }
```

---

## Aggregation Pipeline

`model.aggregate()` returns an `AggregationPipeline<T>` — chain stages and call `exec()` or `exec_raw()`.

```rust
let results = users
    .aggregate()
    .match_stage(doc! { "age": { "$gte": 18 } })
    .sort(doc! { "age": -1 })
    .limit(100)
    .exec()
    .await?;
```

### Available stages

| Method | MongoDB stage |
|---|---|
| `.match_stage(filter)` | `$match` |
| `.sort(doc)` | `$sort` |
| `.limit(n)` | `$limit` |
| `.skip(n)` | `$skip` |
| `.project(doc)` | `$project` |
| `.unwind(path)` | `$unwind` |
| `.unwind_opts(path, preserve_null)` | `$unwind` with options |
| `.group(doc)` | `$group` |
| `.lookup(from, local, foreign, as)` | `$lookup` |
| `.add_fields(doc)` | `$addFields` |
| `.replace_root(expr)` | `$replaceRoot` |
| `.count(field)` | `$count` |
| `.raw_stage(doc)` | any custom stage |

### Example: join and group

```rust
let stats = orders
    .aggregate()
    .match_stage(doc! { "status": "completed" })
    .lookup("users", "user_id", "_id", "user")
    .unwind("$user")
    .group(doc! {
        "_id": "$user._id",
        "total": { "$sum": "$amount" },
        "count": { "$sum": 1 },
    })
    .sort(doc! { "total": -1 })
    .exec_raw()
    .await?;
```

---

## Pagination

```rust
let page = users
    .paginate(2, 10)                        // page 2, 10 per page
    .filter(doc! { "active": true })
    .sort(doc! { "created_at": -1 })
    .exec()
    .await?;

println!("{} total users", page.total);
println!("Page {} of {}", page.page, page.total_pages);
println!("Has next: {} | Has prev: {}", page.has_next, page.has_prev);

for user in page.docs {
    println!("- {}", user.name);
}
```

### `PaginatedResult<T>` fields

| Field | Type | Description |
|---|---|---|
| `docs` | `Vec<T>` | Documents for this page |
| `total` | `u64` | Total matching documents |
| `page` | `u64` | Current page (1-based) |
| `per_page` | `u64` | Page size requested |
| `total_pages` | `u64` | `ceil(total / per_page)` |
| `has_next` | `bool` | `page < total_pages` |
| `has_prev` | `bool` | `page > 1` |

---

## Lifecycle Hooks

Implement `Hooks` on your schema struct. All methods are async and have default no-op implementations — override only what you need.

```rust
use async_trait::async_trait;
use mongrel::{hooks::Hooks, error::Result};

#[async_trait]
impl Hooks for User {
    async fn pre_validate(&self) -> Result<()> {
        Ok(())
    }

    async fn pre_save(&mut self) -> Result<()> {
        self.email = self.email.to_lowercase();
        Ok(())
    }

    async fn post_save(&self) -> Result<()> {
        println!("Saved: {}", self.email);
        Ok(())
    }

    async fn pre_delete(&self) -> Result<()> {
        Ok(())
    }
}
```

### Execution order on `create`

```
pre_validate → validate (field attributes) → post_validate
→ pre_save → insert into MongoDB → post_save
```

---

## Middleware Chaining

`MiddlewareRegistry<T>` lets you register multiple ordered async functions per event — useful for cross-cutting concerns (logging, auditing, encryption) without polluting your schema's `Hooks` impl.

```rust
use mongrel::{MiddlewareRegistry, ModelWithMiddleware};

let registry = MiddlewareRegistry::new()
    .pre_save(|doc: &mut User| async {
        println!("[audit] saving user: {}", doc.email);
        Ok(())
    })
    .post_delete(|doc: &mut User| async {
        println!("[audit] deleted: {}", doc.email);
        Ok(())
    });

let users_mw = ModelWithMiddleware::new(UserModel::new(Arc::clone(&db)), registry);

let user = users_mw.create(User { /* ... */ }).await?;
```

Middleware runs **before** the built-in `Hooks` trait methods, in registration order.

---

## Index Builder

### Unique indexes (via schema attribute)

Fields marked `#[field(unique)]` have their indexes created by `ensure_indexes()`:

```rust
users.ensure_indexes().await?;
```

### Custom indexes (compound, sparse, TTL, text)

Implement `MongooseIndexes` and call `ensure_custom_indexes()`:

```rust
use mongrel::index::{IndexBuilder, MongooseIndexes};

impl MongooseIndexes for User {
    fn indexes() -> Vec<mongrel::IndexDef> {
        vec![
            IndexBuilder::new()
                .field("last_name")
                .field("first_name")
                .name("full_name_idx")
                .build(),

            IndexBuilder::new()
                .field("phone")
                .sparse()
                .build(),

            IndexBuilder::new()
                .field("created_at")
                .ttl(3600)
                .name("session_ttl")
                .build(),

            IndexBuilder::new()
                .text("bio")
                .text("name")
                .name("text_search")
                .build(),

            IndexBuilder::new()
                .field("email")
                .unique()
                .partial_filter(doc! { "active": true })
                .build(),
        ]
    }
}

users.ensure_indexes().await?;
users.ensure_custom_indexes().await?;
```

---

## Error Handling

All fallible methods return `mongrel::Result<T>`, which is `std::result::Result<T, MongooseError>`.

```rust
use mongrel::error::MongooseError;

match users.find_by_id("bad_id").await {
    Ok(Some(user)) => println!("Found: {}", user.name),
    Ok(None)       => println!("Not found"),
    Err(MongooseError::InvalidId(msg))     => eprintln!("Bad ObjectId: {msg}"),
    Err(MongooseError::Validation(msg))    => eprintln!("Validation failed: {msg}"),
    Err(MongooseError::NotFound)           => eprintln!("Document not found"),
    Err(MongooseError::Driver(e))          => eprintln!("MongoDB error: {e}"),
    Err(MongooseError::Serialization(msg)) => eprintln!("BSON error: {msg}"),
}
```

### Error variants

| Variant | When |
|---|---|
| `Driver(mongodb::error::Error)` | Any error from the MongoDB driver |
| `Validation(String)` | Field constraint or custom validation failed |
| `NotFound` | Expected document was absent |
| `Serialization(String)` | BSON serialization / deserialization failure |
| `InvalidId(String)` | Malformed ObjectId string |

---

## Project Structure

```
mongrel/                    ← library crate
├── src/
│   ├── lib.rs              — public re-exports
│   ├── connection.rs       — Mongrel::connect() + global Arc<Database>
│   ├── schema.rs           — MongooseSchema trait
│   ├── model.rs            — Model<T>: CRUD, paginate, aggregate, indexes
│   ├── query.rs            — QueryBuilder: chainable filter/sort/limit/lean
│   ├── populate.rs         — Ref<T>, resolve_ref(), Populate trait
│   ├── virtual_fields.rs   — Virtuals trait + WithVirtuals<T>
│   ├── aggregation.rs      — AggregationPipeline builder
│   ├── pagination.rs       — PaginateBuilder → PaginatedResult<T>
│   ├── middleware.rs        — MiddlewareRegistry<T>, ModelWithMiddleware<T>
│   ├── index.rs            — IndexBuilder, MongooseIndexes trait
│   ├── hooks.rs            — Hooks trait (pre/post save/delete/validate)
│   └── error.rs            — MongooseError, Result<T>
├── examples/
│   └── basic.rs            — end-to-end usage demo
└── tests/
    └── integration.rs      — integration tests (testcontainers + real MongoDB)

mongrel-macros/             ← proc-macro crate (internal)
└── src/
    └── lib.rs              — #[derive(Schema)] + #[derive(Model)]
```

---

## Running the Example

Requires a running MongoDB instance on `localhost:27017`:

```bash
cargo run --example basic
```

## Running Tests

Tests spin up a real MongoDB container via Docker:

```bash
cargo test
```

---

## License

MIT
