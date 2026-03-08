# Performance Benchmarks

qcraft is designed for fast SQL rendering with minimal allocations. This page documents benchmark results comparing qcraft against [sea-query](https://crates.io/crates/sea-query), the most popular standalone SQL query builder for Rust.

## Results

All benchmarks measure **parameterized SQL rendering** to PostgreSQL dialect (producing `$1, $2, ...` placeholders and a `Vec<Value>`).

| Scenario | qcraft | sea-query | Speedup |
|---|---|---|---|
| Simple SELECT + WHERE | 201 ns | 1,345 ns | **6.7x** |
| JOIN + GROUP BY + ORDER BY + LIMIT | 362 ns | 3,168 ns | **8.8x** |
| INSERT (3 rows) | 479 ns | 1,662 ns | **3.5x** |
| Complex CTE + JOIN + GROUP BY + HAVING | 489 ns | 7,152 ns | **14.6x** |

qcraft is **3.5x–14.6x faster** across all scenarios. The gap widens with query complexity.

## Why qcraft is faster

qcraft and sea-query take fundamentally different approaches:

- **qcraft** — AST-first. You construct a typed struct (`QueryStmt`, `InsertStmt`, etc.) and pass it to a renderer. The renderer walks the struct and writes SQL directly. The AST is reusable: build once, render many times with zero overhead.
- **sea-query** — builder pattern. Each method call (`.column()`, `.from()`, `.and_where()`) allocates and mutates internal state. The builder itself is a hidden AST that gets constructed and rendered in one pass.

This means qcraft benchmarks measure **rendering only**, while sea-query benchmarks measure **building + rendering**. In real-world usage, this is a fair comparison — both libraries require you to construct and render a query on each call.

## Scenarios

### Simple SELECT with WHERE

```sql
SELECT "users"."id", "users"."name", "users"."email"
FROM "users"
WHERE "users"."age" > $1 AND "users"."active" = $2
```

### JOIN + GROUP BY + ORDER BY + LIMIT

```sql
SELECT "u"."name", COUNT("o"."id") AS "order_count"
FROM "users" AS "u"
LEFT JOIN "orders" AS "o" ON "u"."id" = "o"."user_id"
WHERE "o"."amount" > $1
GROUP BY "u"."name"
HAVING COUNT("o"."id") > $2
ORDER BY "u"."name" ASC
LIMIT 10 OFFSET 20
```

### INSERT (3 rows)

```sql
INSERT INTO "users" ("name", "email", "age")
VALUES ($1, $2, $3), ($4, $5, $6), ($7, $8, $9)
```

### Complex CTE + JOIN + GROUP BY + HAVING

```sql
WITH "active_users" AS (
    SELECT * FROM "users" WHERE "users"."active" = $1
)
SELECT "u"."id", "u"."name", SUM("o"."amount") AS "total"
FROM "active_users" AS "u"
INNER JOIN "orders" AS "o" ON "u"."id" = "o"."user_id"
GROUP BY "u"."id", "u"."name"
HAVING SUM("o"."amount") > $2
ORDER BY "u"."name" ASC
LIMIT 50
```

## Running benchmarks

```bash
cargo bench -p qcraft-bench
```

Results are saved to `target/criterion/` with HTML reports. To open them:

```bash
open target/criterion/report/index.html
```

## Environment

The results above were collected on:

- **CPU**: Apple Silicon
- **Rust**: 1.85 (edition 2024)
- **qcraft**: 0.1.0
- **sea-query**: 0.32.7
- **criterion**: 0.5.1

Results will vary by hardware, but the relative performance difference is consistent.
