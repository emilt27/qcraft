# Getting Started

Your first query in 5 minutes.

## Installation

Add `rquery` to your project. Both the `postgres` and `sqlite` features are enabled by default:

```bash
cargo add rquery
```

To use only one backend:

```bash
cargo add rquery --no-default-features --features postgres
cargo add rquery --no-default-features --features sqlite
```

## Your First SELECT

### 1. Import types

```rust
use rquery::ast::query::{QueryStmt, SelectColumn, FromItem};
use rquery::ast::common::SchemaRef;
use rquery::ast::value::Value;
use qcraft_postgres::PostgresRenderer;
```

### 2. Build a QueryStmt

Construct a `SELECT "users"."name", "users"."email" FROM "users"` statement:

```rust
let stmt = QueryStmt {
    columns: vec![
        SelectColumn::field("users", "name"),
        SelectColumn::field("users", "email"),
    ],
    from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
    ..Default::default()
};
```

`QueryStmt` derives `Default`, so all optional fields (`joins`, `where_clause`, `order_by`, etc.) start as `None`.

### 3. Render with PostgresRenderer

```rust
let renderer = PostgresRenderer::new();
let (sql, params) = renderer.render_query_stmt(&stmt).unwrap();
```

### 4. Inspect the output

```rust
println!("{}", sql);
// => SELECT "users"."name", "users"."email" FROM "users"

println!("{:?}", params);
// => []
```

### Adding a WHERE clause

Use `Conditions` to add filtering. Parameters are collected automatically:

```rust
use rquery::ast::common::FieldRef;
use rquery::ast::conditions::Conditions;
use rquery::ast::expr::Expr;

let stmt = QueryStmt {
    columns: vec![SelectColumn::field("users", "name")],
    from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
    where_clause: Some(
        Conditions::eq(FieldRef::new("users", "age"), Expr::value(18))
    ),
    ..Default::default()
};

let (sql, params) = PostgresRenderer::new().render_query_stmt(&stmt).unwrap();
println!("{}", sql);
// => SELECT "users"."name" FROM "users" WHERE "users"."age" = $1

println!("{:?}", params);
// => [Int(18)]
```

## Your First INSERT

Use `InsertStmt` and wrap it in `MutationStmt::Insert`:

```rust
use rquery::ast::dml::{InsertStmt, MutationStmt};
use rquery::ast::expr::Expr;

let insert = InsertStmt::values(
    "users",
    vec!["name", "email"],
    vec![
        vec![Expr::value("Alice"), Expr::value("alice@example.com")],
    ],
);

let stmt = MutationStmt::Insert(insert);
let (sql, params) = PostgresRenderer::new().render_mutation_stmt(&stmt).unwrap();

println!("{}", sql);
// => INSERT INTO "users" ("name", "email") VALUES ($1, $2)

println!("{:?}", params);
// => [Str("Alice"), Str("alice@example.com")]
```

## Binding Parameters to a Driver

`render_query_stmt` and `render_mutation_stmt` return `(String, Vec<Value>)`. You convert `Value` variants to your driver's parameter types.

### postgres crate (Rust)

```rust
use postgres::{Client, NoTls};
use postgres::types::ToSql;

// Convert rquery Value to Box<dyn ToSql>
fn to_sql_param(v: &Value) -> Box<dyn ToSql + Sync> {
    match v {
        Value::Int(i) => Box::new(*i),
        Value::Str(s) => Box::new(s.clone()),
        Value::Bool(b) => Box::new(*b),
        Value::Float(f) => Box::new(*f),
        Value::Null => Box::new(None::<String>),
        // handle other variants as needed
        _ => unimplemented!(),
    }
}

let (sql, params) = PostgresRenderer::new().render_query_stmt(&stmt).unwrap();
let sql_params: Vec<Box<dyn ToSql + Sync>> = params.iter().map(to_sql_param).collect();
let refs: Vec<&(dyn ToSql + Sync)> = sql_params.iter().map(|p| p.as_ref()).collect();

let mut client = Client::connect("host=localhost dbname=mydb", NoTls).unwrap();
let rows = client.query(&sql, &refs).unwrap();
```

### rusqlite crate

```rust
use rusqlite::{Connection, params_from_iter};
use rusqlite::types::Value as RValue;

fn to_rusqlite(v: &Value) -> RValue {
    match v {
        Value::Int(i) => RValue::Integer(*i),
        Value::Str(s) => RValue::Text(s.clone()),
        Value::Float(f) => RValue::Real(*f),
        Value::Null => RValue::Null,
        Value::Bool(b) => RValue::Integer(*b as i64),
        _ => unimplemented!(),
    }
}

let (sql, params) = SqliteRenderer::new().render_query_stmt(&stmt).unwrap();
let rusqlite_params: Vec<RValue> = params.iter().map(to_rusqlite).collect();

let conn = Connection::open("my.db").unwrap();
let mut prepared = conn.prepare(&sql).unwrap();
let rows = prepared.query(params_from_iter(rusqlite_params)).unwrap();
```

### psycopg (Python)

If you are generating SQL for a Python backend, use `ParamStyle::Percent` to produce `%s` placeholders compatible with psycopg and DB-API 2.0:

```rust
use rquery::render::ctx::ParamStyle;

let renderer = PostgresRenderer::new().with_param_style(ParamStyle::Percent);
let (sql, params) = renderer.render_query_stmt(&stmt).unwrap();
// sql uses %s placeholders: SELECT "users"."name" FROM "users" WHERE "users"."age" = %s
```

Serialize the `params` vector (e.g. as JSON) and pass it to your Python layer.

## Choosing a Renderer

rquery ships two renderers. Both expose the same convenience methods (`render_query_stmt`, `render_mutation_stmt`, `render_schema_stmt`, `render_transaction_stmt`).

| Renderer | Placeholder style | Crate |
|---|---|---|
| `PostgresRenderer::new()` | `$1, $2, ...` | `qcraft-postgres` |
| `SqliteRenderer::new()` | `?` | `qcraft-sqlite` |

`PostgresRenderer` also supports `ParamStyle::Percent` via `.with_param_style(ParamStyle::Percent)` for psycopg/DB-API compatibility.

## Next Steps

- [SELECT Queries](select-queries.md) -- joins, subqueries, CTEs, window functions, set operations
- [INSERT / UPDATE / DELETE](insert-update-delete.md) -- upserts, RETURNING, conflict resolution
- [DDL Statements](ddl.md) -- CREATE TABLE, indexes, constraints
- [Transaction Control](transactions.md) -- BEGIN, COMMIT, ROLLBACK, isolation levels
