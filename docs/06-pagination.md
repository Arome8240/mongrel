# Pagination

`model.paginate(page, per_page)` returns a `PaginateBuilder<T>`. Call `.exec()` to run both the count and the data query in a single async chain, receiving a `PaginatedResult<T>` with all the metadata you need to build a paging API response.

---

## Basic usage

```rust
let result = users
    .paginate(1, 20)       // page 1, 20 documents per page (1-based)
    .exec()
    .await?;

println!("Page {} of {}", result.page, result.total_pages);
println!("{} total users", result.total);
println!("Has next page: {}", result.has_next);

for user in &result.docs {
    println!("- {}", user.name);
}
```

---

## `PaginatedResult<T>` fields

| Field | Type | Description |
|---|---|---|
| `docs` | `Vec<T>` | Documents for this page |
| `total` | `u64` | Total documents matching the filter (all pages) |
| `page` | `u64` | Current page (1-based; clamped to min 1) |
| `per_page` | `u64` | Page size requested (clamped to min 1) |
| `total_pages` | `u64` | `ceil(total / per_page)` |
| `has_next` | `bool` | `page < total_pages` |
| `has_prev` | `bool` | `page > 1` |

---

## Filtering

```rust
let result = users
    .paginate(2, 10)
    .filter(doc! { "role": "user", "active": true })
    .exec()
    .await?;
```

---

## Sorting

```rust
let result = users
    .paginate(1, 25)
    .sort(doc! { "created_at": -1 })    // most recent first
    .exec()
    .await?;
```

---

## Field projection

```rust
let result = users
    .paginate(1, 50)
    .select(["name", "email", "role"])
    .exec()
    .await?;
```

---

## Combining options

```rust
let result = users
    .paginate(3, 15)
    .filter(doc! { "active": true })
    .sort(doc! { "score": -1 })
    .select(["name", "score"])
    .exec()
    .await?;
```

---

## Typical API response pattern

```rust
use serde::Serialize;

#[derive(Serialize)]
struct PagedResponse<T: Serialize> {
    data:        Vec<T>,
    page:        u64,
    per_page:    u64,
    total:       u64,
    total_pages: u64,
    has_next:    bool,
    has_prev:    bool,
}

let result = users.paginate(page, per_page).exec().await?;

let response = PagedResponse {
    data:        result.docs,
    page:        result.page,
    per_page:    result.per_page,
    total:       result.total,
    total_pages: result.total_pages,
    has_next:    result.has_next,
    has_prev:    result.has_prev,
};
```

---

## Notes

- Pages are **1-based**. Passing `page = 0` is treated as `page = 1`.
- `per_page = 0` is treated as `per_page = 1` to avoid a division-by-zero.
- Two queries are issued per `.exec()` call: one `countDocuments` and one `find`. For very large collections, consider caching the total separately or using the aggregation pipeline with `$facet` for a single round-trip.
