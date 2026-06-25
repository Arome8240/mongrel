# Model CRUD Operations

`#[derive(Model)]` generates a `{Name}Model` struct that wraps `Model<T>`. Construct it once at startup with a shared database handle and reuse it across your application.

```rust
use std::sync::Arc;

let db = Mongrel::connect("mongodb://localhost:27017", "myapp").await?;
let users = UserModel::new(Arc::clone(&db));
```

All methods are async and return `mongrel::Result<T>`.

---

## Create

```rust
let user = users.create(User {
    id: None,
    name: "Alice".into(),
    email: "alice@example.com".into(),
    role: "admin".into(),
    age: Some(30),
}).await?;

// user.id is now Some(ObjectId(...))
```

**Lifecycle executed:**
```
pre_validate → validate (field attributes) → post_validate
→ pre_save → INSERT → post_save
```

If `#[schema(timestamps)]` is active, `created_at` and `updated_at` are injected automatically.

---

## Find

### By id

```rust
// Returns Ok(None) if not found — does NOT error
let user: Option<User> = users.find_by_id("64b1f9...").await?;
```

### Chainable query (see [Query Builder](./03-query-builder.md) for full docs)

```rust
let admins: Vec<User> = users
    .find()
    .where_field("role").eq("admin")
    .sort("name", SortDir::Asc)
    .limit(50)
    .exec()
    .await?;
```

### Raw filter

```rust
let results = users
    .find_many(doc! { "age": { "$gte": 18 }, "active": true })
    .exec()
    .await?;
```

### Single document

```rust
let one: Option<User> = users
    .find_one_where(doc! { "email": "alice@example.com" })
    .exec_one()
    .await?;
```

---

## Update

### By id

```rust
// Returns the document AFTER the update, or None if not found
let updated: Option<User> = users
    .find_by_id_and_update("64b1f9...", doc! { "$set": { "role": "moderator" } })
    .await?;
```

### By filter

```rust
let updated: Option<User> = users
    .find_one_and_update(
        doc! { "email": "alice@example.com" },
        doc! { "$inc": { "login_count": 1 } },
    )
    .await?;
```

### Many documents

```rust
let modified: u64 = users
    .update_many(
        doc! { "role": "user" },
        doc! { "$set": { "email_verified": false } },
    )
    .await?;
```

---

## Delete

### By id

```rust
let deleted: Option<User> = users.find_by_id_and_delete("64b1f9...").await?;
```

### By filter

```rust
let deleted: Option<User> = users
    .find_one_and_delete(doc! { "email": "alice@example.com" })
    .await?;
```

### Many documents

```rust
let count: u64 = users.delete_many(doc! { "active": false }).await?;
```

---

## Upsert

Find a document and update it; if none exists, insert a new one:

```rust
let doc: User = users
    .find_one_and_upsert(
        doc! { "email": "new@example.com" },
        doc! {
            "$setOnInsert": { "name": "New User", "role": "user" },
            "$set": { "last_seen": bson::DateTime::now() },
        },
    )
    .await?;
```

Unlike `find_one_and_update`, this always returns the document (never `None`) — or `MongooseError::NotFound` if the upsert somehow fails to return one.

---

## Count

```rust
// Count all documents
let total: u64 = users.count_documents(doc! {}).await?;

// Count with a filter
let admins: u64 = users.count_documents(doc! { "role": "admin" }).await?;
```

---

## Ensure indexes

Call at startup after constructing your model:

```rust
// Unique indexes from #[field(unique)]
users.ensure_indexes().await?;

// Compound/sparse/TTL indexes from MongooseIndexes impl
users.ensure_custom_indexes().await?;
```

See [Index Builder](./07-indexes.md) for custom index declaration.
