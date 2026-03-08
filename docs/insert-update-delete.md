# INSERT / UPDATE / DELETE (DML Operations)

qcraft builds DML statements as typed AST nodes (`InsertStmt`, `UpdateStmt`, `DeleteStmt`), wraps them in `MutationStmt`, and renders via `renderer.render_mutation_stmt(&stmt)`. All literal values are parameterized -- they become `$1, $2, ...` (PostgreSQL) or `?, ?, ...` (SQLite), never inline literals.

The renderer returns `(String, Vec<Value>)` -- the SQL text and the ordered parameter vector.

```rust
use qcraft_core::ast::dml::*;
use qcraft_core::ast::expr::Expr;
use qcraft_core::ast::value::Value;
use qcraft_core::ast::query::SelectColumn;
use qcraft_core::ast::common::SchemaRef;
use qcraft_postgres::PostgresRenderer;

let renderer = PostgresRenderer::new();
let (sql, params) = renderer.render_mutation_stmt(&stmt).unwrap();
```

---

## 1. INSERT

### 1.1 Single row

```rust
let stmt = MutationStmt::Insert(
    InsertStmt::values("users", vec!["name"], vec![
        vec![Expr::value("Alice")],
    ])
);
```

```sql
-- PG
INSERT INTO "users" ("name") VALUES ($1)
-- params: ["Alice"]
```

### 1.2 Multiple rows

```rust
let stmt = MutationStmt::Insert(
    InsertStmt::values("users", vec!["name"], vec![
        vec![Expr::value("Alice")],
        vec![Expr::value("Bob")],
    ])
);
```

```sql
-- PG
INSERT INTO "users" ("name") VALUES ($1), ($2)
-- params: ["Alice", "Bob"]

-- SQLite
INSERT INTO "users" ("name") VALUES (?), (?)
```

### 1.3 Default values

```rust
let stmt = MutationStmt::Insert(
    InsertStmt::default_values("counters")
);
```

```sql
INSERT INTO "counters" DEFAULT VALUES
```

### 1.4 INSERT from SELECT

Build a `QueryStmt` and pass it to `InsertStmt::from_select`:

```rust
use qcraft_core::ast::query::QueryStmt;

let select = QueryStmt {
    columns: vec![SelectColumn::field("employees", "name")],
    from: Some(vec![/* ... */]),
    ..Default::default()
};

let stmt = MutationStmt::Insert(
    InsertStmt::from_select("users", vec!["name"], select)
);
```

```sql
INSERT INTO "users" ("name") SELECT "employees"."name" FROM ...
```

### 1.5 RETURNING

Chain `.returning()` on any `InsertStmt`:

```rust
let stmt = MutationStmt::Insert(
    InsertStmt::values("users", vec!["name"], vec![
        vec![Expr::value("Alice")],
    ])
    .returning(vec![SelectColumn::all()])
);
```

```sql
INSERT INTO "users" ("name") VALUES ($1) RETURNING *
```

Return specific columns:

```rust
use qcraft_core::ast::common::FieldRef;

let stmt = MutationStmt::Insert(InsertStmt {
    table: SchemaRef::new("users"),
    columns: Some(vec!["name".into()]),
    source: InsertSource::Values(vec![vec![Expr::value("Alice")]]),
    returning: Some(vec![
        SelectColumn::Field {
            field: FieldRef::new("users", "id"),
            alias: None,
        },
        SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: Some("user_name".into()),
        },
    ]),
    ..Default::default()
});
```

```sql
INSERT INTO "users" ("name") VALUES ($1)
  RETURNING "users"."id", "users"."name" AS "user_name"
```

### 1.6 ON CONFLICT DO NOTHING / DO UPDATE (upsert)

**DO NOTHING** with conflict target:

```rust
let stmt = MutationStmt::Insert(
    InsertStmt::values("users", vec!["email", "name"], vec![
        vec![Expr::value("a@b.com"), Expr::value("Alice")],
    ])
    .on_conflict(OnConflictDef {
        target: Some(ConflictTarget::Columns {
            columns: vec!["email".into()],
            where_clause: None,
        }),
        action: ConflictAction::DoNothing,
    })
);
```

```sql
INSERT INTO "users" ("email", "name") VALUES ($1, $2)
  ON CONFLICT ("email") DO NOTHING
```

**DO UPDATE** (upsert) -- reference the proposed row via `EXCLUDED`:

```rust
let stmt = MutationStmt::Insert(
    InsertStmt::values("users", vec!["email", "name"], vec![
        vec![Expr::value("a@b.com"), Expr::value("Alice")],
    ])
    .on_conflict(OnConflictDef {
        target: Some(ConflictTarget::Columns {
            columns: vec!["email".into()],
            where_clause: None,
        }),
        action: ConflictAction::DoUpdate {
            assignments: vec![(
                "name".into(),
                Expr::Raw {
                    sql: "EXCLUDED.\"name\"".into(),
                    params: vec![],
                },
            )],
            where_clause: None,
        },
    })
);
```

```sql
INSERT INTO "users" ("email", "name") VALUES ($1, $2)
  ON CONFLICT ("email") DO UPDATE SET "name" = EXCLUDED."name"
```

**Convenience constructor** `OnConflictDef::do_nothing()` and `OnConflictDef::do_update(...)`:

```rust
// DO NOTHING without a target (catch-all, useful in SQLite)
let oc = OnConflictDef::do_nothing();

// DO UPDATE with column target + assignments
let oc = OnConflictDef::do_update(
    vec!["email"],
    vec![("name", Expr::Raw {
        sql: "EXCLUDED.\"name\"".into(),
        params: vec![],
    })],
);
```

**ON CONSTRAINT** (PostgreSQL only):

```rust
let stmt = MutationStmt::Insert(
    InsertStmt::values("users", vec!["email"], vec![
        vec![Expr::value("a@b.com")],
    ])
    .on_conflict(OnConflictDef {
        target: Some(ConflictTarget::Constraint("uq_email".into())),
        action: ConflictAction::DoNothing,
    })
);
```

```sql
INSERT INTO "users" ("email") VALUES ($1)
  ON CONFLICT ON CONSTRAINT "uq_email" DO NOTHING
```

### 1.7 OVERRIDING SYSTEM VALUE (PostgreSQL)

For identity columns, override the sequence-generated value:

```rust
let stmt = MutationStmt::Insert(InsertStmt {
    table: SchemaRef::new("users"),
    columns: Some(vec!["id".into(), "name".into()]),
    source: InsertSource::Values(vec![vec![
        Expr::Value(Value::Int(100)),
        Expr::value("Alice"),
    ]]),
    overriding: Some(OverridingKind::System),
    ..Default::default()
});
```

```sql
INSERT INTO "users" ("id", "name") OVERRIDING SYSTEM VALUE VALUES ($1, $2)
-- params: [100, "Alice"]
```

### 1.8 SQLite conflict resolution

SQLite uses `INSERT OR REPLACE`, `INSERT OR IGNORE`, etc. instead of (or alongside) ON CONFLICT:

```rust
let stmt = MutationStmt::Insert(InsertStmt {
    table: SchemaRef::new("users"),
    columns: Some(vec!["id".into(), "name".into()]),
    source: InsertSource::Values(vec![vec![
        Expr::Value(Value::Int(1)),
        Expr::value("Alice"),
    ]]),
    conflict_resolution: Some(ConflictResolution::Replace),
    ..Default::default()
});
```

```sql
INSERT OR REPLACE INTO "users" ("id", "name") VALUES (?, ?)
```

Available variants: `ConflictResolution::Rollback`, `Abort`, `Fail`, `Ignore`, `Replace`.

---

## 2. UPDATE

### 2.1 Simple update

```rust
let stmt = MutationStmt::Update(
    UpdateStmt::new("users", vec![
        ("name", Expr::value("Bob")),
    ])
);
```

```sql
UPDATE "users" SET "name" = $1
-- params: ["Bob"]
```

### 2.2 With WHERE

```rust
use qcraft_core::ast::conditions::*;

let stmt = MutationStmt::Update(
    UpdateStmt::new("users", vec![
        ("name", Expr::value("Bob")),
    ])
    .where_clause(Conditions {
        children: vec![ConditionNode::Comparison(Box::new(Comparison {
            left: Expr::Raw { sql: "\"id\"".into(), params: vec![] },
            op: CompareOp::Eq,
            right: Expr::Value(Value::Int(1)),
            negate: false,
        }))],
        connector: Connector::And,
        negated: false,
    })
);
```

```sql
UPDATE "users" SET "name" = $1 WHERE "id" = $2
-- params: ["Bob", 1]
```

### 2.3 With FROM (PostgreSQL)

Join another table into the UPDATE:

```rust
use qcraft_core::ast::query::TableSource;

let stmt = MutationStmt::Update(UpdateStmt {
    table: SchemaRef::new("orders").with_alias("o"),
    assignments: vec![("status".into(), Expr::value("shipped"))],
    from: Some(vec![TableSource::Table(
        SchemaRef::new("users").with_alias("u"),
    )]),
    where_clause: Some(Conditions {
        children: vec![ConditionNode::Comparison(Box::new(Comparison {
            left: Expr::Raw { sql: "\"o\".\"user_id\"".into(), params: vec![] },
            op: CompareOp::Eq,
            right: Expr::Raw { sql: "\"u\".\"id\"".into(), params: vec![] },
            negate: false,
        }))],
        connector: Connector::And,
        negated: false,
    }),
    ..Default::default()
});
```

```sql
UPDATE "orders" AS "o" SET "status" = $1
  FROM "users" AS "u"
  WHERE "o"."user_id" = "u"."id"
```

### 2.4 With RETURNING

```rust
let stmt = MutationStmt::Update(
    UpdateStmt::new("users", vec![("name", Expr::value("Bob"))])
        .where_clause(/* ... */)
        .returning(vec![SelectColumn::all()])
);
```

```sql
UPDATE "users" SET "name" = $1 WHERE "id" = $2 RETURNING *
```

### 2.5 SQLite: ORDER BY + LIMIT in UPDATE

SQLite (and MySQL) support `ORDER BY` and `LIMIT` on UPDATE statements:

```rust
use qcraft_core::ast::common::{OrderByDef, FieldRef, OrderDir};

let stmt = MutationStmt::Update(UpdateStmt {
    table: SchemaRef::new("logs"),
    assignments: vec![("archived".into(), Expr::Value(Value::Bool(true)))],
    order_by: Some(vec![OrderByDef {
        expr: Expr::Field(FieldRef::new("logs", "created_at")),
        direction: OrderDir::Asc,
        nulls: None,
    }]),
    limit: Some(100),
    offset: Some(10),
    ..Default::default()
});
```

```sql
UPDATE "logs" SET "archived" = ?
  ORDER BY "logs"."created_at" ASC LIMIT 100 OFFSET 10
```

---

## 3. DELETE

### 3.1 Simple delete

```rust
let stmt = MutationStmt::Delete(DeleteStmt::new("users"));
```

```sql
DELETE FROM "users"
```

### 3.2 With WHERE

```rust
let stmt = MutationStmt::Delete(
    DeleteStmt::new("users")
        .where_clause(Conditions {
            children: vec![ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Raw { sql: "\"id\"".into(), params: vec![] },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            }))],
            connector: Connector::And,
            negated: false,
        })
);
```

```sql
DELETE FROM "users" WHERE "id" = $1
-- params: [1]
```

### 3.3 With RETURNING

```rust
let stmt = MutationStmt::Delete(
    DeleteStmt::new("users")
        .where_clause(Conditions {
            children: vec![ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Raw { sql: "\"active\"".into(), params: vec![] },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(false)),
                negate: false,
            }))],
            connector: Connector::And,
            negated: false,
        })
        .returning(vec![SelectColumn::all()])
);
```

```sql
DELETE FROM "users" WHERE "active" = $1 RETURNING *
-- params: [false]
```

### 3.4 With USING (PostgreSQL)

Join another table into the DELETE:

```rust
use qcraft_core::ast::query::TableSource;

let stmt = MutationStmt::Delete(DeleteStmt {
    table: SchemaRef::new("orders").with_alias("o"),
    using: Some(vec![TableSource::Table(
        SchemaRef::new("users").with_alias("u"),
    )]),
    where_clause: Some(Conditions {
        children: vec![ConditionNode::Comparison(Box::new(Comparison {
            left: Expr::Raw { sql: "\"o\".\"user_id\"".into(), params: vec![] },
            op: CompareOp::Eq,
            right: Expr::Raw { sql: "\"u\".\"id\"".into(), params: vec![] },
            negate: false,
        }))],
        connector: Connector::And,
        negated: false,
    }),
    ..Default::default()
});
```

```sql
DELETE FROM "orders" AS "o"
  USING "users" AS "u"
  WHERE "o"."user_id" = "u"."id"
```

---

## 4. Wrapping in MutationStmt

Every DML statement is wrapped in the `MutationStmt` enum before rendering:

```rust
use qcraft_core::ast::dml::{MutationStmt, InsertStmt, UpdateStmt, DeleteStmt};

// INSERT
let stmt = MutationStmt::Insert(InsertStmt::values(/* ... */));

// UPDATE
let stmt = MutationStmt::Update(UpdateStmt::new(/* ... */));

// DELETE
let stmt = MutationStmt::Delete(DeleteStmt::new(/* ... */));
```

Render with the dialect-specific renderer:

```rust
use qcraft_postgres::PostgresRenderer;
use qcraft_sqlite::SqliteRenderer;

// PostgreSQL -- parameters are $1, $2, ...
let pg = PostgresRenderer::new();
let (sql, params) = pg.render_mutation_stmt(&stmt).unwrap();

// SQLite -- parameters are ?, ?, ...
let sqlite = SqliteRenderer::new();
let (sql, params) = sqlite.render_mutation_stmt(&stmt).unwrap();
```

The `params` vector (`Vec<Value>`) contains every parameterized value in binding order. Pass it to your database driver alongside the SQL string.
