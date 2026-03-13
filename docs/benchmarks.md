# Performance Benchmarks

qcraft is designed for fast SQL rendering with minimal allocations. This page documents benchmark results comparing qcraft against [sea-query](https://crates.io/crates/sea-query), the most popular standalone SQL query builder for Rust.

## Results

All benchmarks measure **parameterized SQL rendering** to PostgreSQL dialect (producing `$1, $2, ...` placeholders and a `Vec<Value>`).

| Scenario | qcraft | sea-query | Speedup |
|---|---|---|---|
| Simple SELECT + WHERE | 227 ns | 1,419 ns | **6.2x** |
| JOIN + GROUP BY + ORDER BY + LIMIT | 420 ns | 3,494 ns | **8.3x** |
| INSERT (3 rows) | 537 ns | 1,797 ns | **3.3x** |
| Complex CTE + JOIN + GROUP BY + HAVING | 549 ns | 7,341 ns | **13.4x** |

qcraft is **3.3x–13.4x faster** across all scenarios. The gap widens with query complexity.

## Memory allocations

Each call to build + render a query allocates heap memory. Fewer allocations mean less GC pressure and better cache behavior. Measured with a tracking global allocator, averaged over 1,000 iterations:

| Scenario | qcraft allocs | sea-query allocs | qcraft bytes | sea-query bytes |
|---|---|---|---|---|
| Simple SELECT + WHERE | 20 | 34 | 1,652 B | 8,353 B |
| JOIN + GROUP BY + ORDER BY | 42 | 82 | 2,509 B | 13,247 B |
| INSERT (3 rows) | 34 | 57 | 2,758 B | 4,767 B |
| Complex CTE + JOIN | 52 | 195 | 3,980 B | 35,604 B |

qcraft uses **1.7x–3.8x fewer allocations** and **1.7x–8.9x less memory** per query.

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

Speed benchmarks (criterion):

```bash
cargo bench -p qcraft-bench --bench compare
```

Results are saved to `target/criterion/` with HTML reports. To open them:

```bash
open target/criterion/report/index.html
```

Allocation benchmarks:

```bash
cargo bench -p qcraft-bench --bench allocations
```

## Environment

The results above were collected on:

- **CPU**: Apple Silicon
- **Rust**: 1.85 (edition 2024)
- **qcraft**: 1.0.0
- **sea-query**: 0.32.7
- **criterion**: 0.5.1

Results will vary by hardware, but the relative performance difference is consistent.
