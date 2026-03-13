# SELECT Queries

Comprehensive reference for building `SELECT` queries with qcraft.
All examples use `PostgresRenderer` for SQL output unless stated otherwise.
Parameters are rendered as `$1`, `$2`, ... for PostgreSQL and `?` for SQLite.

```rust
use qcraft_core::ast::common::{FieldRef, NullsOrder, OrderByDef, OrderDir, SchemaRef};
use qcraft_core::ast::conditions::{CompareOp, Comparison, ConditionNode, Conditions, Connector};
use qcraft_core::ast::expr::*;
use qcraft_core::ast::query::*;
use qcraft_core::ast::value::Value;
use qcraft_postgres::PostgresRenderer;

fn render(stmt: &QueryStmt) -> String {
    let renderer = PostgresRenderer::new();
    let (sql, _) = renderer.render_query_stmt(stmt).unwrap();
    sql
}
```

---

## 1. Basic SELECT

### SELECT *

```rust
let stmt = QueryStmt {
    columns: vec![SelectColumn::all()],
    from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
    ..QueryStmt::default()
};
```

```sql
SELECT * FROM "users"
```

### SELECT table.*

```rust
let stmt = QueryStmt {
    columns: vec![SelectColumn::all_from("u")],
    from: Some(vec![FromItem::table(
        SchemaRef::new("users").with_alias("u"),
    )]),
    ..QueryStmt::default()
};
```

```sql
SELECT "u".* FROM "users" AS "u"
```

### SELECT columns

```rust
let stmt = QueryStmt {
    columns: vec![
        SelectColumn::field("u", "id"),
        SelectColumn::field("u", "name"),
    ],
    from: Some(vec![FromItem::table(
        SchemaRef::new("users").with_alias("u"),
    )]),
    ..QueryStmt::default()
};
```

```sql
SELECT "u"."id", "u"."name" FROM "users" AS "u"
```

### Field with schema namespace

```rust
SelectColumn::Field {
    field: FieldRef {
        field: FieldDef::new("id"),
        table_name: "users".into(),
        namespace: Some("public".into()),
    },
    alias: None,
}
```

```sql
"public"."users"."id"
```

### JSON path access (FieldDef child)

`FieldDef.child` chains produce `->` operators for JSON traversal:

```rust
SelectColumn::Field {
    field: FieldRef {
        field: FieldDef {
            name: "data".into(),
            child: Some(Box::new(FieldDef {
                name: "address".into(),
                child: Some(Box::new(FieldDef::new("city"))),
            })),
        },
        table_name: "users".into(),
        namespace: None,
    },
    alias: None,
}
```

```sql
"users"."data"->'address'->'city'
```

> All child levels use `->` (returns JSON). For text extraction, wrap with `Expr::cast(expr, "text")` to get `->>` equivalent behavior.

### SELECT with alias

```rust
let stmt = QueryStmt {
    columns: vec![
        SelectColumn::field_aliased("users", "name", "user_name"),
        SelectColumn::aliased(Expr::Value(Value::Int(1)), "one"),
    ],
    from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
    ..QueryStmt::default()
};
```

```sql
SELECT "users"."name" AS "user_name", $1 AS "one" FROM "users"
-- params: [Int(1)]
```

### SELECT expression (no alias)

```rust
let stmt = QueryStmt {
    columns: vec![SelectColumn::expr(Expr::func("NOW", vec![]))],
    ..QueryStmt::default()
};
```

```sql
SELECT NOW()
```

### SELECT without FROM

```rust
let stmt = QueryStmt {
    columns: vec![SelectColumn::expr(Expr::Value(Value::Int(42)))],
    ..QueryStmt::default()
};
```

```sql
SELECT $1
-- params: [Int(42)]
```

---

## 2. FROM

### Simple table

```rust
FromItem::table(SchemaRef::new("users"))
```

```sql
FROM "users"
```

### Table with namespace (schema)

```rust
FromItem::table(SchemaRef::new("users").with_namespace("public"))
```

```sql
FROM "public"."users"
```

### Table with alias

```rust
FromItem::table(SchemaRef::new("users").with_alias("u"))
```

```sql
FROM "users" AS "u"
```

### ONLY (PG -- exclude inherited tables)

```rust
FromItem {
    source: TableSource::Table(SchemaRef::new("events")),
    only: true,
    sample: None,
    index_hint: None,
}
```

```sql
FROM ONLY "events"
```

### Subquery

```rust
let inner = QueryStmt {
    columns: vec![SelectColumn::all()],
    from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
    ..QueryStmt::default()
};
FromItem::subquery(inner, "sub".into())
```

```sql
FROM (SELECT * FROM "users") AS "sub"
```

### Table-valued function

```rust
FromItem::function(
    "generate_series",
    vec![Expr::Value(Value::Int(1)), Expr::Value(Value::Int(10))],
    "s",
)
```

```sql
FROM generate_series($1, $2) AS "s"
-- params: [Int(1), Int(10)]
```

### VALUES as table source

```rust
FromItem {
    source: TableSource::Values {
        rows: vec![
            vec![Expr::Value(Value::Int(1)), Expr::Value(Value::Str("a".into()))],
            vec![Expr::Value(Value::Int(2)), Expr::Value(Value::Str("b".into()))],
        ],
        alias: "t".into(),
        column_aliases: Some(vec!["id".into(), "name".into()]),
    },
    only: false,
    sample: None,
    index_hint: None,
}
```

```sql
FROM (VALUES ($1, $2), ($3, $4)) AS "t" ("id", "name")
-- params: [Int(1), Str("a"), Int(2), Str("b")]
```

### LATERAL

```rust
FromItem::lateral(FromItem::subquery(inner_query, "recent".into()))
```

```sql
FROM LATERAL (SELECT * FROM "orders") AS "recent"
```

**SQLite:** LATERAL is unsupported and returns an error.

### Multiple FROM items

```rust
from: Some(vec![
    FromItem::table(SchemaRef::new("users").with_alias("u")),
    FromItem::table(SchemaRef::new("orders").with_alias("o")),
])
```

```sql
FROM "users" AS "u", "orders" AS "o"
```

---

## 3. WHERE

### Simple equality

```rust
Conditions::eq(
    FieldRef::new("users", "id"),
    Expr::Value(Value::Int(1)),
)
```

```sql
WHERE "users"."id" = $1
-- params: [Int(1)]
```

### Comparison operators

```rust
Conditions::neq(FieldRef::new("t", "status"), Expr::Value(Value::Str("deleted".into())))
Conditions::gt(FieldRef::new("t", "age"), Expr::Value(Value::Int(18)))
Conditions::gte(FieldRef::new("t", "score"), Expr::Value(Value::Float(9.5)))
Conditions::lt(FieldRef::new("t", "rank"), Expr::Value(Value::Int(100)))
Conditions::lte(FieldRef::new("t", "price"), Expr::Value(Value::Float(19.99)))
```

```sql
"t"."status" != $1
"t"."age" > $1
"t"."score" >= $1
"t"."rank" < $1
"t"."price" <= $1
```

### IS NULL / IS NOT NULL

```rust
Conditions::is_null(FieldRef::new("users", "deleted_at"))
Conditions::is_not_null(FieldRef::new("users", "email"))
```

```sql
"users"."deleted_at" IS NULL
"users"."email" IS NOT NULL
```

### LIKE (raw)

```rust
Conditions::like(FieldRef::new("users", "name"), "%alice%")
```

```sql
"users"."name" LIKE $1
-- params: [Str("%alice%")]
```

> With raw `LIKE` the caller provides the full pattern including wildcards.
> Special characters (`%`, `_`, `\`) are **not** escaped automatically.

### Contains / StartsWith / EndsWith

High-level string matching — the renderer escapes special LIKE characters and wraps with `%` automatically.

```rust
Conditions::contains(FieldRef::new("users", "name"), "ali")
Conditions::starts_with(FieldRef::new("users", "name"), "Ali")
Conditions::ends_with(FieldRef::new("users", "name"), "ice")
```

```sql
-- PostgreSQL
"users"."name" LIKE $1   -- params: [Str("%ali%")]
"users"."name" LIKE $1   -- params: [Str("Ali%")]
"users"."name" LIKE $1   -- params: [Str("%ice")]

-- SQLite (adds explicit ESCAPE clause)
"users"."name" LIKE ? ESCAPE '\'
```

Special characters in the value are escaped automatically:

```rust
Conditions::contains(FieldRef::new("products", "name"), "50%")
// Pattern becomes: %50\%%  — matches literal "50%", not "500"
```

### Case-insensitive: IContains / IStartsWith / IEndsWith

```rust
Conditions::icontains(FieldRef::new("users", "name"), "alice")
Conditions::istarts_with(FieldRef::new("users", "name"), "ali")
Conditions::iends_with(FieldRef::new("users", "name"), "ICE")
```

```sql
-- PostgreSQL (uses ILIKE)
"users"."name" ILIKE $1   -- params: [Str("%alice%")]

-- SQLite (wraps with LOWER)
LOWER("users"."name") LIKE LOWER(?) ESCAPE '\'
```

### IN (values)

```rust
Conditions::and(vec![ConditionNode::Comparison(Box::new(Comparison::new(
    Expr::Field(FieldRef::new("users", "status")),
    CompareOp::In,
    Expr::Value(Value::Array(vec![
        Value::Str("active".into()),
        Value::Str("pending".into()),
    ])),
)))])
```

```sql
"users"."status" IN ($1, $2)
-- params: [Str("active"), Str("pending")]
```

> When the right side is `Value::Array`, the renderer expands it into separate parameters.

### IN (subquery)

```rust
let sub = QueryStmt {
    columns: vec![SelectColumn::field("orders", "user_id")],
    from: Some(vec![FromItem::table(SchemaRef::new("orders"))]),
    ..QueryStmt::default()
};
Conditions::in_subquery(FieldRef::new("users", "id"), sub)
```

```sql
"users"."id" IN (SELECT "orders"."user_id" FROM "orders")
```

### BETWEEN

```rust
Conditions::and(vec![ConditionNode::Comparison(Box::new(Comparison::new(
    Expr::Field(FieldRef::new("users", "age")),
    CompareOp::Between,
    Expr::Value(Value::Array(vec![Value::Int(18), Value::Int(65)])),
)))])
```

```sql
"users"."age" BETWEEN $1 AND $2
-- params: [Int(18), Int(65)]
```

> Pass exactly 2 values in the array. The renderer expands them into `BETWEEN $1 AND $2`.

### Combining: AND

```rust
Conditions::eq(FieldRef::new("u", "active"), Expr::Value(Value::Bool(true)))
    .and_also(Conditions::gt(FieldRef::new("u", "age"), Expr::Value(Value::Int(18))))
```

```sql
"u"."active" = $1 AND "u"."age" > $2
```

### Combining: OR

```rust
Conditions::eq(FieldRef::new("u", "role"), Expr::Value(Value::Str("admin".into())))
    .or_else(Conditions::eq(FieldRef::new("u", "role"), Expr::Value(Value::Str("moderator".into()))))
```

```sql
("u"."role" = $1) OR ("u"."role" = $2)
```

### Negation

```rust
Conditions::eq(FieldRef::new("u", "active"), Expr::Value(Value::Bool(true)))
    .negated()
```

```sql
NOT ("u"."active" = $1)
```

### Multiple AND conditions (direct)

```rust
Conditions::and(vec![
    ConditionNode::Comparison(Box::new(Comparison {
        left: Expr::Field(FieldRef::new("users", "active")),
        op: CompareOp::Eq,
        right: Expr::Value(Value::Bool(true)),
        negate: false,
    })),
    ConditionNode::Comparison(Box::new(Comparison {
        left: Expr::Field(FieldRef::new("users", "age")),
        op: CompareOp::Gt,
        right: Expr::Value(Value::Int(18)),
        negate: false,
    })),
])
```

```sql
WHERE "users"."active" = $1 AND "users"."age" > $2
```

### EXISTS

```rust
ConditionNode::Exists(Box::new(subquery))
```

```sql
EXISTS(SELECT ...)
```

### NOT EXISTS

```rust
ConditionNode::Exists(Box::new(subquery))
// Use inside Conditions with negated: true
```

```sql
NOT EXISTS(SELECT ...)
```

---

## 4. JOIN

### INNER JOIN

```rust
JoinDef::inner(
    FromItem::table(SchemaRef::new("orders").with_alias("o")),
    Conditions::eq(
        FieldRef::new("u", "id"),
        Expr::Field(FieldRef::new("o", "user_id")),
    ),
)
```

```sql
INNER JOIN "orders" AS "o" ON "u"."id" = "o"."user_id"
```

### LEFT JOIN

```rust
JoinDef::left(
    FromItem::table(SchemaRef::new("orders").with_alias("o")),
    Conditions::eq(
        FieldRef::new("u", "id"),
        Expr::Field(FieldRef::new("o", "user_id")),
    ),
)
```

```sql
LEFT JOIN "orders" AS "o" ON "u"."id" = "o"."user_id"
```

### RIGHT JOIN

```rust
JoinDef::right(
    FromItem::table(SchemaRef::new("orders").with_alias("o")),
    Conditions::eq(
        FieldRef::new("u", "id"),
        Expr::Field(FieldRef::new("o", "user_id")),
    ),
)
```

```sql
RIGHT JOIN "orders" AS "o" ON "u"."id" = "o"."user_id"
```

### FULL JOIN

```rust
JoinDef::full(
    FromItem::table(SchemaRef::new("b")),
    Conditions::eq(
        FieldRef::new("a", "id"),
        Expr::Field(FieldRef::new("b", "id")),
    ),
)
```

```sql
FULL JOIN "b" ON "a"."id" = "b"."id"
```

### CROSS JOIN

```rust
JoinDef::cross(FromItem::table(SchemaRef::new("colors")))
```

```sql
CROSS JOIN "colors"
```

> Any join condition provided on a `CROSS JOIN` is silently ignored, since `CROSS JOIN ... ON` is a syntax error in both PostgreSQL and SQLite.

### JOIN USING

```rust
JoinDef::using(
    JoinType::Inner,
    FromItem::table(SchemaRef::new("b")),
    vec!["id".into(), "name".into()],
)
```

```sql
INNER JOIN "b" USING ("id", "name")
```

### NATURAL JOIN

```rust
JoinDef::inner(
    FromItem::table(SchemaRef::new("profiles")),
    some_condition,
).natural()
// or directly:
JoinDef {
    source: FromItem::table(SchemaRef::new("profiles")),
    condition: None,
    join_type: JoinType::Inner,
    natural: true,
}
```

```sql
NATURAL INNER JOIN "profiles"
```

### LATERAL JOIN (PG only)

```rust
JoinDef {
    source: FromItem {
        source: TableSource::Lateral(Box::new(FromItem::subquery(
            inner_query,
            "recent_orders".into(),
        ))),
        only: false,
        sample: None,
        index_hint: None,
    },
    condition: Some(JoinCondition::On(on_condition)),
    join_type: JoinType::Left,
    natural: false,
}
```

```sql
LEFT JOIN LATERAL (SELECT * FROM "orders") AS "recent_orders" ON ...
```

**SQLite:** LATERAL is unsupported and returns an error.

---

## 5. GROUP BY + HAVING

### Simple GROUP BY

```rust
let stmt = QueryStmt {
    columns: vec![
        SelectColumn::field("users", "country"),
        SelectColumn::aliased(Expr::count_all(), "cnt"),
    ],
    from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
    group_by: Some(vec![
        GroupByItem::Expr(Expr::field("users", "country")),
    ]),
    ..QueryStmt::default()
};
```

```sql
SELECT "users"."country", COUNT(*) AS "cnt" FROM "users" GROUP BY "users"."country"
```

### ROLLUP (PG only)

```rust
group_by: Some(vec![GroupByItem::Rollup(vec![
    Expr::field("sales", "region"),
    Expr::field("sales", "product"),
])])
```

```sql
GROUP BY ROLLUP ("sales"."region", "sales"."product")
```

**SQLite:** ROLLUP is unsupported and returns an error.

### CUBE (PG only)

```rust
group_by: Some(vec![GroupByItem::Cube(vec![
    Expr::field("sales", "region"),
    Expr::field("sales", "product"),
])])
```

```sql
GROUP BY CUBE ("sales"."region", "sales"."product")
```

**SQLite:** CUBE is unsupported and returns an error.

### GROUPING SETS (PG only)

```rust
group_by: Some(vec![GroupByItem::GroupingSets(vec![
    vec![Expr::field("sales", "region"), Expr::field("sales", "product")],
    vec![Expr::field("sales", "region")],
    vec![],  // empty set = grand total
])])
```

```sql
GROUP BY GROUPING SETS (("sales"."region", "sales"."product"), ("sales"."region"), ())
```

**SQLite:** GROUPING SETS is unsupported and returns an error.

### HAVING

```rust
having: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
    Comparison {
        left: Expr::Func {
            name: "COUNT".into(),
            args: vec![Expr::Value(Value::Int(1))],
        },
        op: CompareOp::Gt,
        right: Expr::Value(Value::Int(5)),
        negate: false,
    }),
)]))
```

```sql
HAVING COUNT($1) > $2
-- params: [Int(1), Int(5)]
```

---

## 6. ORDER BY

### Ascending / Descending

```rust
order_by: Some(vec![
    OrderByDef::asc(Expr::field("users", "name")),
    OrderByDef::desc(Expr::field("users", "id")),
])
```

```sql
ORDER BY "users"."name" ASC, "users"."id" DESC
```

### NULLS FIRST / NULLS LAST

```rust
OrderByDef::desc(Expr::field("users", "score")).nulls_first()
OrderByDef::asc(Expr::field("users", "score")).nulls_last()
```

```sql
ORDER BY "users"."score" DESC NULLS FIRST
ORDER BY "users"."score" ASC NULLS LAST
```

Both PostgreSQL and SQLite support NULLS FIRST / NULLS LAST.

### COLLATE

Override the collation used for sorting or comparison. The `.collate()` method works on any `Expr`:

```rust
// In ORDER BY
OrderByDef::asc(Expr::field("users", "name").collate("C"))

// In WHERE
Conditions::and(vec![ConditionNode::Comparison(Box::new(Comparison {
    left: Expr::field("users", "name").collate("NOCASE"),
    op: CompareOp::Eq,
    right: Expr::Value(Value::Str("alice".into())),
    negate: false,
}))])
```

```sql
-- PostgreSQL (collation name quoted as identifier)
ORDER BY "users"."name" COLLATE "C" ASC
WHERE "users"."name" COLLATE "C" = $1

-- SQLite (collation name as keyword, unquoted)
ORDER BY "users"."name" COLLATE NOCASE ASC
WHERE "users"."name" COLLATE NOCASE = ?
```

Common collations:

| PostgreSQL | SQLite |
|---|---|
| `"C"`, `"POSIX"` — byte ordering | `BINARY` — byte comparison (default) |
| `"default"` — database default | `NOCASE` — case-insensitive ASCII |
| ICU collations (`"uk-x-icu"`, etc.) | `RTRIM` — ignores trailing spaces |

### Custom binary operators

`BinaryOp::Custom` allows dialect-specific operators. `qcraft-postgres` ships with pgvector distance operators:

```rust
use qcraft_postgres::PgVectorOp;

// ORDER BY nearest neighbor (L2 distance)
OrderByDef::asc(Expr::Binary {
    left: Box::new(Expr::field("items", "embedding")),
    op: PgVectorOp::L2Distance.into(),
    right: Box::new(Expr::Value(Value::Vector(vec![1.0, 2.0, 3.0]))),
})
```

```sql
ORDER BY "items"."embedding" <-> $1 ASC
```

Available operators: `L2Distance` (`<->`), `InnerProduct` (`<#>`), `CosineDistance` (`<=>`), `L1Distance` (`<+>`).

SQLite returns a render error for custom binary operators.

---

## 7. LIMIT / OFFSET

### LIMIT

```rust
LimitDef::limit(10)
```

```sql
LIMIT $1
-- params: [BigInt(10)]
```

> LIMIT and OFFSET values are parameterized as `Value::BigInt` (PG `int8`) in query mode. In DDL/literal mode they render as inline numbers.

### LIMIT with OFFSET

```rust
LimitDef::limit_offset(10, 20)
```

```sql
LIMIT $1 OFFSET $2
-- params: [BigInt(10), BigInt(20)]
```

### FETCH FIRST (PG)

```rust
LimitDef::fetch_first(5)
```

```sql
FETCH FIRST 5 ROWS ONLY
```

**SQLite:** `FetchFirst` is silently converted to `LIMIT`.

### FETCH FIRST WITH TIES (PG)

```rust
LimitDef::fetch_first_with_ties(10)
```

```sql
FETCH FIRST 10 ROWS WITH TIES
```

**SQLite:** `FETCH FIRST ... WITH TIES` is unsupported and returns an error.

### FETCH FIRST with OFFSET (PG)

```rust
LimitDef::fetch_first(5).offset(10)
```

```sql
OFFSET 10 ROWS FETCH FIRST 5 ROWS ONLY
```

### FETCH FIRST PERCENT (PG)

```rust
LimitDef {
    kind: LimitKind::FetchFirst {
        count: 10,
        with_ties: false,
        percent: true,
    },
    offset: None,
}
```

```sql
FETCH FIRST 10 PERCENT ROWS ONLY
```

### TOP (SQL Server syntax, converted to LIMIT by PG/SQLite)

```rust
LimitDef::top(5)
```

```sql
LIMIT $1
-- params: [BigInt(5)]
```

---

## 8. DISTINCT

### DISTINCT

```rust
let stmt = QueryStmt {
    distinct: Some(DistinctDef::Distinct),
    columns: vec![SelectColumn::all()],
    from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
    ..QueryStmt::default()
};
```

```sql
SELECT DISTINCT * FROM "users"
```

### DISTINCT ON (PG only)

```rust
let stmt = QueryStmt {
    distinct: Some(DistinctDef::DistinctOn(vec![
        Expr::field("users", "email"),
    ])),
    columns: vec![SelectColumn::all()],
    from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
    ..QueryStmt::default()
};
```

```sql
SELECT DISTINCT ON ("users"."email") * FROM "users"
```

**SQLite:** `DISTINCT ON` is unsupported and returns an error.

---

## 9. CTE (WITH)

### Simple CTE

```rust
let cte_query = QueryStmt {
    columns: vec![SelectColumn::all()],
    from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
    where_clause: Some(Conditions::eq(
        FieldRef::new("users", "active"),
        Expr::Value(Value::Bool(true)),
    )),
    ..QueryStmt::default()
};

let stmt = QueryStmt {
    ctes: Some(vec![CteDef::new("active_users", cte_query)]),
    columns: vec![SelectColumn::all()],
    from: Some(vec![FromItem::table(SchemaRef::new("active_users"))]),
    ..QueryStmt::default()
};
```

```sql
WITH "active_users" AS (SELECT * FROM "users" WHERE "users"."active" = $1) SELECT * FROM "active_users"
```

### Recursive CTE

```rust
CteDef::recursive("nums", base_query)
```

```sql
WITH RECURSIVE "nums" AS (SELECT $1 AS "n") SELECT * FROM "nums"
```

### CTE with explicit column names

```rust
CteDef::new("data", query).columns(vec!["id", "name"])
```

```sql
WITH "data" ("id", "name") AS (SELECT ...) SELECT * FROM "data"
```

### MATERIALIZED / NOT MATERIALIZED (PG only)

```rust
CteDef::new("cached", query).materialized()
CteDef::new("inlined", query).not_materialized()
```

```sql
WITH "cached" AS MATERIALIZED (SELECT ...) ...
WITH "inlined" AS NOT MATERIALIZED (SELECT ...) ...
```

**SQLite:** The MATERIALIZED / NOT MATERIALIZED hint is silently ignored; the CTE renders without it.

---

## 10. Set Operations

### UNION

```rust
SetOpDef::union(left_query, right_query)
```

```sql
(SELECT * FROM "users") UNION (SELECT * FROM "admins")
```

### UNION ALL

```rust
SetOpDef::union_all(left_query, right_query)
```

```sql
(SELECT * FROM "users") UNION ALL (SELECT * FROM "admins")
```

### INTERSECT

```rust
SetOpDef::intersect(left_query, right_query)
```

```sql
(SELECT * FROM "users") INTERSECT (SELECT * FROM "admins")
```

### EXCEPT

```rust
SetOpDef::except(left_query, right_query)
```

```sql
(SELECT * FROM "users") EXCEPT (SELECT * FROM "admins")
```

Set operations are used as a `TableSource` within a `FromItem`:

```rust
let stmt = QueryStmt {
    columns: vec![SelectColumn::all()],
    from: Some(vec![FromItem {
        source: TableSource::SetOp(Box::new(SetOpDef::union_all(left, right))),
        only: false,
        sample: None,
        index_hint: None,
    }]),
    ..QueryStmt::default()
};
```

Additional variants: `SetOperationType::IntersectAll`, `SetOperationType::ExceptAll`.

**SQLite:** `INTERSECT ALL` and `EXCEPT ALL` are unsupported and return an error.

---

## 11. Window Functions

### Inline window definition (WindowDef)

```rust
let stmt = QueryStmt {
    columns: vec![SelectColumn::aliased(
        Expr::Window(WindowDef {
            expression: Box::new(Expr::Aggregate(AggregationDef::new(
                "SUM",
                Expr::field("sales", "amount"),
            ))),
            partition_by: Some(vec![Expr::field("sales", "region")]),
            order_by: Some(vec![OrderByDef::asc(Expr::field("sales", "date"))]),
            frame: None,
        }),
        "running_total",
    )],
    from: Some(vec![FromItem::table(SchemaRef::new("sales"))]),
    ..QueryStmt::default()
};
```

```sql
SELECT SUM("sales"."amount") OVER (PARTITION BY "sales"."region" ORDER BY "sales"."date" ASC) AS "running_total"
FROM "sales"
```

### Named window (WindowNameDef) in WINDOW clause

```rust
window: Some(vec![WindowNameDef {
    name: "w".into(),
    base_window: None,
    partition_by: Some(vec![Expr::field("sales", "region")]),
    order_by: Some(vec![OrderByDef::desc(Expr::field("sales", "amount"))]),
    frame: None,
}])
```

```sql
WINDOW "w" AS (PARTITION BY "sales"."region" ORDER BY "sales"."amount" DESC)
```

### Window inheritance (base_window)

```rust
window: Some(vec![
    WindowNameDef {
        name: "w1".into(),
        base_window: None,
        partition_by: Some(vec![Expr::field("t", "a")]),
        order_by: None,
        frame: None,
    },
    WindowNameDef {
        name: "w2".into(),
        base_window: Some("w1".into()),
        partition_by: None,
        order_by: Some(vec![OrderByDef::asc(Expr::field("t", "b"))]),
        frame: None,
    },
])
```

```sql
WINDOW "w1" AS (PARTITION BY "t"."a"), "w2" AS ("w1" ORDER BY "t"."b" ASC)
```

### Window frame (WindowFrameDef)

```rust
frame: Some(WindowFrameDef {
    frame_type: WindowFrameType::Rows,
    start: WindowFrameBound::Preceding(Some(1)),
    end: Some(WindowFrameBound::Following(Some(1))),
})
```

```sql
ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING
```

Frame types: `WindowFrameType::Rows`, `WindowFrameType::Range`, `WindowFrameType::Groups`.

Frame bounds:
- `WindowFrameBound::CurrentRow` -- `CURRENT ROW`
- `WindowFrameBound::Preceding(Some(n))` -- `n PRECEDING`
- `WindowFrameBound::Preceding(None)` -- `UNBOUNDED PRECEDING`
- `WindowFrameBound::Following(Some(n))` -- `n FOLLOWING`
- `WindowFrameBound::Following(None)` -- `UNBOUNDED FOLLOWING`

### Aggregate with DISTINCT and FILTER

```rust
Expr::Aggregate(
    AggregationDef::new("COUNT", Expr::field("orders", "id"))
        .distinct()
        .filter(Conditions::gt(
            FieldRef::new("orders", "amount"),
            Expr::Value(Value::Float(100.0)),
        ))
)
```

```sql
COUNT(DISTINCT "orders"."id") FILTER (WHERE "orders"."amount" > $1)
```

---

## 12. FOR UPDATE / FOR SHARE (PG only)

### FOR UPDATE

```rust
lock: Some(vec![SelectLockDef {
    strength: LockStrength::Update,
    of: None,
    nowait: false,
    skip_locked: false,
    wait: None,
}])
```

```sql
FOR UPDATE
```

### FOR SHARE OF table

```rust
lock: Some(vec![SelectLockDef {
    strength: LockStrength::Share,
    of: Some(vec![SchemaRef::new("users")]),
    nowait: false,
    skip_locked: false,
    wait: None,
}])
```

```sql
FOR SHARE OF "users"
```

### FOR UPDATE NOWAIT

```rust
lock: Some(vec![SelectLockDef {
    strength: LockStrength::Update,
    of: None,
    nowait: true,
    skip_locked: false,
    wait: None,
}])
```

```sql
FOR UPDATE NOWAIT
```

### FOR UPDATE SKIP LOCKED

```rust
lock: Some(vec![SelectLockDef {
    strength: LockStrength::Update,
    of: None,
    nowait: false,
    skip_locked: true,
    wait: None,
}])
```

```sql
FOR UPDATE SKIP LOCKED
```

### FOR NO KEY UPDATE / FOR KEY SHARE

```rust
SelectLockDef { strength: LockStrength::NoKeyUpdate, .. }
SelectLockDef { strength: LockStrength::KeyShare, .. }
```

```sql
FOR NO KEY UPDATE
FOR KEY SHARE
```

### Multiple lock clauses

PG supports multiple lock clauses in one query:

```rust
lock: Some(vec![
    SelectLockDef {
        strength: LockStrength::Update,
        of: Some(vec![SchemaRef::new("users")]),
        nowait: true,
        skip_locked: false,
        wait: None,
    },
    SelectLockDef {
        strength: LockStrength::Share,
        of: Some(vec![SchemaRef::new("orders")]),
        nowait: false,
        skip_locked: false,
        wait: None,
    },
])
```

```sql
FOR UPDATE OF "users" NOWAIT FOR SHARE OF "orders"
```

**SQLite:** Any `FOR UPDATE` / `FOR SHARE` clause is unsupported and returns an error.

---

## 13. TABLESAMPLE (PG only)

### BERNOULLI

```rust
FromItem {
    source: TableSource::Table(SchemaRef::new("large_table")),
    only: false,
    sample: Some(TableSampleDef {
        method: SampleMethod::Bernoulli,
        percentage: 10.0,
        seed: None,
    }),
    index_hint: None,
}
```

```sql
FROM "large_table" TABLESAMPLE BERNOULLI (10)
```

### SYSTEM with REPEATABLE (seed)

```rust
FromItem {
    source: TableSource::Table(SchemaRef::new("big_table")),
    only: false,
    sample: Some(TableSampleDef {
        method: SampleMethod::System,
        percentage: 5.5,
        seed: Some(42),
    }),
    index_hint: None,
}
```

```sql
FROM "big_table" TABLESAMPLE SYSTEM (5.5) REPEATABLE (42)
```

Sampling methods: `SampleMethod::Bernoulli`, `SampleMethod::System`, `SampleMethod::Block` (Oracle).

**SQLite:** TABLESAMPLE is unsupported and returns an error.

---

## SQLite-Specific Features

### INDEXED BY

```rust
FromItem {
    source: TableSource::Table(SchemaRef::new("users")),
    only: false,
    sample: None,
    index_hint: Some(SqliteIndexHint::IndexedBy("idx_name".into())),
}
```

```sql
FROM "users" INDEXED BY "idx_name"
```

### NOT INDEXED

```rust
FromItem {
    source: TableSource::Table(SchemaRef::new("users")),
    only: false,
    sample: None,
    index_hint: Some(SqliteIndexHint::NotIndexed),
}
```

```sql
FROM "users" NOT INDEXED
```

---

## Full Example

```rust
let stmt = QueryStmt {
    ctes: None,
    columns: vec![
        SelectColumn::field("u", "name"),
        SelectColumn::aliased(
            Expr::Func {
                name: "COUNT".into(),
                args: vec![Expr::field("o", "id")],
            },
            "order_count",
        ),
    ],
    distinct: None,
    from: Some(vec![FromItem::table(
        SchemaRef::new("users").with_alias("u"),
    )]),
    joins: Some(vec![JoinDef::left(
        FromItem::table(SchemaRef::new("orders").with_alias("o")),
        Conditions::eq(
            FieldRef::new("u", "id"),
            Expr::field("o", "user_id"),
        ),
    )]),
    where_clause: Some(Conditions::eq(
        FieldRef::new("u", "active"),
        Expr::Value(Value::Bool(true)),
    )),
    group_by: Some(vec![GroupByItem::Expr(Expr::field("u", "name"))]),
    having: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
        Comparison {
            left: Expr::Func {
                name: "COUNT".into(),
                args: vec![Expr::field("o", "id")],
            },
            op: CompareOp::Gt,
            right: Expr::Value(Value::Int(0)),
            negate: false,
        }),
    )])),
    window: None,
    order_by: Some(vec![OrderByDef::asc(Expr::field("u", "name"))]),
    limit: Some(LimitDef::limit_offset(10, 0)),
    lock: None,
};
```

```sql
SELECT "u"."name", COUNT("o"."id") AS "order_count"
FROM "users" AS "u"
LEFT JOIN "orders" AS "o" ON "u"."id" = "o"."user_id"
WHERE "u"."active" = $1
GROUP BY "u"."name"
HAVING COUNT("o"."id") > $2
ORDER BY "u"."name" ASC
LIMIT $3 OFFSET $4
-- params: [Bool(true), Int(0), BigInt(10), BigInt(0)]
```
