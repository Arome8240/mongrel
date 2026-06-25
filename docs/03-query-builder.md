# Query Builder

`model.find()` returns a `QueryBuilder<T>`. Every method on it returns `Self`, so you can chain as many conditions, options, and projections as you need before calling a terminal method.

---

## Basic pattern

```rust
let results = users
    .find()
    .where_field("age").gte(18)       // condition
    .sort("name", SortDir::Asc)       // sort
    .limit(20)                        // limit
    .exec()                           // execute
    .await?;
```

---

## Filter operators

All operators follow the pattern `.where_field("fieldName").operator(value)`.

### Equality

```rust
.where_field("role").eq("admin")          // { role: "admin" }
.where_field("active").eq(true)
.where_field("score").eq(100)
```

### Comparison

```rust
.where_field("age").ne(0)                 // $ne
.where_field("age").gt(17)                // $gt
.where_field("age").gte(18)               // $gte
.where_field("age").lt(65)                // $lt
.where_field("age").lte(64)               // $lte
```

### Arrays / sets

```rust
.where_field("role").in_list(["admin", "moderator"])   // $in
.where_field("status").nin(["banned", "deleted"])       // $nin
```

### String pattern

```rust
// Case-insensitive match on names starting with "alice"
.where_field("name").regex("^alice", "i")

// Case-sensitive exact substring
.where_field("bio").regex("Rust developer", "")
```

### Field existence

```rust
.where_field("phone").field_exists(true)    // document has a "phone" field
.where_field("deleted_at").field_exists(false)  // field absent
```

---

## Multiple conditions

Each `.where_field(...).operator(...)` call adds an independent AND condition:

```rust
users.find()
    .where_field("age").gte(18)
    .where_field("age").lte(65)
    .where_field("role").eq("user")
    .where_field("active").eq(true)
    .exec().await?;
```

---

## Sort, limit, skip

```rust
users.find()
    .sort("created_at", SortDir::Desc)   // most recent first
    .sort("name", SortDir::Asc)          // then alphabetically
    .skip(20)                            // skip the first 20
    .limit(10)                           // return at most 10
    .exec().await?;
```

Calling `.sort` multiple times adds keys to the same sort document in order.

---

## Field projection (`select`)

Only fetch the listed fields (plus `_id` by default):

```rust
users.find()
    .select(["name", "email", "role"])
    .exec().await?;
```

Useful for bandwidth-sensitive reads. The returned `T` will have all other fields at their `Default` / `None` values.

---

## Raw filter escape hatch

For operators not covered by the typed API (`$or`, `$and`, `$elemMatch`, etc.):

```rust
users.find()
    .filter(doc! {
        "$or": [
            { "role": "admin" },
            { "age": { "$gt": 60 } }
        ]
    })
    .sort("name", SortDir::Asc)
    .exec().await?;
```

The raw filter **replaces** any conditions built up with `.where_field(...)`.

---

## Terminal methods

| Method | Return type | Description |
|---|---|---|
| `.exec()` | `Result<Vec<T>>` | All matching documents |
| `.exec_one()` | `Result<Option<T>>` | First matching document, or `None` |
| `.count()` | `Result<u64>` | Number of matching documents |
| `.any()` | `Result<bool>` | `true` if at least one document matches |
| `.lean()` | `Result<Vec<Document>>` | Raw BSON — no deserialization |
| `.lean_one()` | `Result<Option<Document>>` | First raw BSON document |

---

## Lean queries

`.lean()` skips deserializing results into `T` and returns raw `bson::Document`s instead. Use it when:

- You only need a handful of fields and don't want the overhead of full deserialization.
- The query output shape doesn't match `T` (e.g., after `$project`).

```rust
let docs: Vec<bson::Document> = users
    .find()
    .where_field("active").eq(true)
    .select(["name", "email"])
    .lean()
    .await?;

for doc in &docs {
    println!("{}", doc.get_str("name").unwrap_or("?"));
}
```

---

## `find_many` / `find_one_where` convenience methods

If you already have a BSON filter document, skip `.where_field()` chains:

```rust
// Multi-result
let users = users.find_many(doc! { "role": "admin" }).limit(5).exec().await?;

// Single result
let user = users.find_one_where(doc! { "email": "x@y.com" }).exec_one().await?;
```
