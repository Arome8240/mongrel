# Aggregation Pipeline

`model.aggregate()` returns an `AggregationPipeline<T>` that lets you compose a MongoDB aggregation pipeline with a fluent Rust API. Stages execute in the order you chain them.

---

## Basic usage

```rust
let results: Vec<User> = users
    .aggregate()
    .match_stage(doc! { "active": true })
    .sort(doc! { "created_at": -1 })
    .limit(100)
    .exec()      // deserializes each result into User
    .await?;
```

---

## Terminal methods

| Method | Return | When to use |
|---|---|---|
| `.exec()` | `Vec<T>` | Output shape matches your schema type |
| `.exec_raw()` | `Vec<Document>` | Output is shaped by `$project`/`$group` and doesn't match `T` |

Use `.exec_raw()` whenever the pipeline reshapes the documents — for example after `$group` or `$project`:

```rust
let stats: Vec<bson::Document> = orders
    .aggregate()
    .group(doc! {
        "_id": "$status",
        "count": { "$sum": 1 },
        "revenue": { "$sum": "$amount_cents" },
    })
    .sort(doc! { "revenue": -1 })
    .exec_raw()
    .await?;

for doc in &stats {
    println!(
        "{}: {} orders, {} cents",
        doc.get_str("_id").unwrap_or("?"),
        doc.get_i32("count").unwrap_or(0),
        doc.get_i64("revenue").unwrap_or(0),
    );
}
```

---

## All stages

### `$match` — filter

```rust
.match_stage(doc! { "age": { "$gte": 18 }, "active": true })
```

### `$sort` — order

```rust
.sort(doc! { "score": -1, "name": 1 })   // descending score, then name ascending
```

### `$limit` and `$skip`

```rust
.skip(20).limit(10)   // page 3 of 10-per-page results
```

### `$project` — reshape

```rust
.project(doc! {
    "name": 1,
    "email": 1,
    "_id": 0,                         // exclude _id
    "display": { "$concat": ["$first_name", " ", "$last_name"] },
})
```

### `$unwind` — flatten arrays

```rust
// Simple — drops documents where the field is null/missing
.unwind("$tags")

// With options — keeps documents where the field is null/missing
.unwind_opts("$tags", true)
```

### `$group` — aggregate

```rust
.group(doc! {
    "_id": "$department",
    "headcount": { "$sum": 1 },
    "avg_salary": { "$avg": "$salary" },
    "names": { "$push": "$name" },
})
```

### `$lookup` — join

```rust
.lookup(
    "orders",          // from collection
    "user_id",         // local field
    "_id",             // foreign field
    "user_orders",     // output array field name
)
```

### `$addFields`

```rust
.add_fields(doc! {
    "full_name": { "$concat": ["$first_name", " ", "$last_name"] },
    "age_next_year": { "$add": ["$age", 1] },
})
```

### `$replaceRoot`

```rust
.replace_root("$profile")   // promotes the nested "profile" subdocument to top level
```

### `$count`

```rust
.count("total")   // outputs { "total": N }
```

### Raw escape hatch

Push any pipeline stage not covered by the typed API:

```rust
.raw_stage(doc! { "$sample": { "size": 5 } })       // random 5 documents
.raw_stage(doc! { "$out": "exported_users" })         // write to another collection
```

---

## Full example: user report with join

```rust
let report: Vec<bson::Document> = users
    .aggregate()
    .match_stage(doc! { "active": true })
    .lookup("orders", "_id", "user_id", "orders")
    .add_fields(doc! {
        "order_count": { "$size": "$orders" },
        "total_spent":  { "$sum": "$orders.amount_cents" },
    })
    .project(doc! {
        "name": 1, "email": 1, "order_count": 1, "total_spent": 1, "_id": 0
    })
    .sort(doc! { "total_spent": -1 })
    .limit(10)
    .exec_raw()
    .await?;
```
