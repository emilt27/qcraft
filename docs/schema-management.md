# Schema Management (DDL Operations)

rquery provides a type-safe Rust API for building DDL statements through `SchemaMutationStmt` and its associated types. All DDL statements use inline literals (not parameterized placeholders) and are rendered via:

```rust
let (sql, _params) = renderer.render_schema_stmt(&stmt).unwrap();
```

where `renderer` is either `PostgresRenderer::new()` or `SqliteRenderer::new()`.

---

## 1. CREATE TABLE

### Simple table with columns

```rust
use rquery_core::ast::ddl::*;

let mut schema = SchemaDef::new("users");
schema.columns = vec![
    ColumnDef::new("id", FieldType::scalar("BIGINT")).not_null(),
    ColumnDef::new("name", FieldType::scalar("TEXT")),
];
let stmt = SchemaMutationStmt::create_table(schema);
```

PostgreSQL:
```sql
CREATE TABLE "users" ("id" BIGINT NOT NULL, "name" TEXT)
```

### Column types

`FieldType` supports scalar, parameterized, array, and vector forms:

```rust
// Scalar — plain type name
FieldType::scalar("TEXT")
FieldType::scalar("BIGINT")
FieldType::scalar("TIMESTAMPTZ")

// Parameterized — type with size/precision arguments
FieldType::parameterized("VARCHAR", vec!["255"])
FieldType::parameterized("NUMERIC", vec!["10", "2"])

// Array (PG only — errors on SQLite)
FieldType::Array(Box::new(FieldType::scalar("TEXT")))

// Vector (pgvector)
FieldType::Vector(1536)
```

Full parameterized type example:

```rust
let mut schema = SchemaDef::new("t");
schema.columns = vec![
    ColumnDef::new("name", FieldType::parameterized("VARCHAR", vec!["255"])),
    ColumnDef::new("amount", FieldType::parameterized("NUMERIC", vec!["10", "2"])),
    ColumnDef::new("tags", FieldType::Array(Box::new(FieldType::scalar("TEXT")))),
    ColumnDef::new("embedding", FieldType::Vector(1536)),
];
let stmt = SchemaMutationStmt::create_table(schema);
```

PostgreSQL:
```sql
CREATE TABLE "t" ("name" VARCHAR(255), "amount" NUMERIC(10, 2), "tags" TEXT[], "embedding" VECTOR(1536))
```

### NOT NULL and DEFAULT

```rust
ColumnDef::new("key", FieldType::scalar("TEXT")).not_null()

ColumnDef::new("status", FieldType::scalar("TEXT"))
    .not_null()
    .default(Expr::Value(Value::Str("draft".into())))
```

PostgreSQL:
```sql
CREATE TABLE "posts" ("id" SERIAL, "status" TEXT NOT NULL DEFAULT 'draft')
```

### IF NOT EXISTS

```rust
let stmt = SchemaMutationStmt::CreateTable {
    schema,
    if_not_exists: true,
    temporary: false,
    unlogged: false,
    tablespace: None,
    partition_by: None,
    inherits: None,
    using_method: None,
    with_options: None,
    on_commit: None,
    table_options: None,
    without_rowid: false,
    strict: false,
};
```

```sql
CREATE TABLE IF NOT EXISTS "users" ("id" INTEGER)
```

### TEMPORARY

```rust
let stmt = SchemaMutationStmt::CreateTable {
    schema,
    temporary: true,
    // ... other fields ...
    ..  // remaining fields as defaults
};
```

SQLite renders `TEMP`:
```sql
CREATE TEMP TABLE "temp_data" ("val" TEXT)
```

PostgreSQL renders `TEMPORARY`:
```sql
CREATE TEMPORARY TABLE "tmp" ("x" INT)
```

### Generated columns

A generated (computed) column uses `GeneratedColumn` with an expression and a `stored` flag:

```rust
ColumnDef {
    name: "total".into(),
    field_type: FieldType::scalar("NUMERIC"),
    not_null: false,
    default: None,
    generated: Some(GeneratedColumn {
        expr: Expr::Raw {
            sql: "price * qty".into(),
            params: vec![],
        },
        stored: true,
    }),
    identity: None,
    collation: None,
    comment: None,
    storage: None,
    compression: None,
}
```

PostgreSQL (only STORED is supported):
```sql
"total" NUMERIC GENERATED ALWAYS AS (price * qty) STORED
```

SQLite supports both STORED and VIRTUAL:
```sql
-- stored: false
"total" REAL GENERATED ALWAYS AS (price * qty) VIRTUAL
-- stored: true
"full_name" TEXT GENERATED ALWAYS AS (first_name || ' ' || last_name) STORED
```

### Identity columns (PostgreSQL only)

Identity columns provide standards-compliant auto-incrementing. SQLite renders an error for identity columns.

```rust
ColumnDef {
    name: "id".into(),
    field_type: FieldType::scalar("BIGINT"),
    not_null: true,
    default: None,
    generated: None,
    identity: Some(IdentityColumn {
        always: true,
        start: Some(1),
        increment: Some(1),
        ..Default::default()
    }),
    collation: None,
    comment: None,
    storage: None,
    compression: None,
}
```

PostgreSQL:
```sql
"id" BIGINT NOT NULL GENERATED ALWAYS AS IDENTITY (START WITH 1 INCREMENT BY 1)
```

### SQLite-specific: WITHOUT ROWID, STRICT, AUTOINCREMENT

```rust
// WITHOUT ROWID
let stmt = SchemaMutationStmt::CreateTable {
    schema,
    without_rowid: true,
    strict: false,
    // ...
};
```

```sql
CREATE TABLE "kv" ("key" TEXT NOT NULL, "value" BLOB, PRIMARY KEY ("key")) WITHOUT ROWID
```

```rust
// STRICT
let stmt = SchemaMutationStmt::CreateTable {
    schema,
    strict: true,
    // ...
};
```

```sql
CREATE TABLE "data" ("id" INTEGER NOT NULL, "name" TEXT) STRICT
```

Both can be combined:

```sql
CREATE TABLE "kv_strict" ("key" TEXT NOT NULL, "val" INTEGER, PRIMARY KEY ("key")) WITHOUT ROWID, STRICT
```

AUTOINCREMENT is set on `ConstraintDef::PrimaryKey`:

```rust
schema.constraints = Some(vec![ConstraintDef::PrimaryKey {
    name: None,
    columns: vec!["id".into()],
    include: None,
    autoincrement: true,
}]);
```

SQLite renders this as an inline column constraint:
```sql
CREATE TABLE "events" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "name" TEXT)
```

### PostgreSQL-specific: UNLOGGED, INHERITS, LIKE, PARTITION BY, tablespace, storage params

**UNLOGGED:**

```rust
let stmt = SchemaMutationStmt::CreateTable {
    schema,
    unlogged: true,
    // ...
};
```

```sql
CREATE TEMPORARY UNLOGGED TABLE "tmp" ("x" INT)
```

Note: on SQLite, `unlogged` is silently ignored.

**INHERITS:**

```rust
let stmt = SchemaMutationStmt::CreateTable {
    schema,
    inherits: Some(vec![SchemaRef::new("parent_table")]),
    // ...
};
```

```sql
CREATE TABLE "child_table" ("extra" TEXT) INHERITS ("parent_table")
```

**LIKE:**

```rust
let mut schema = SchemaDef::new("users_copy");
schema.like_tables = Some(vec![LikeTableDef {
    source_table: SchemaRef::new("users"),
    options: vec![
        LikeOption { kind: LikeOptionKind::All, include: true },
        LikeOption { kind: LikeOptionKind::Indexes, include: false },
    ],
}]);
let stmt = SchemaMutationStmt::create_table(schema);
```

```sql
CREATE TABLE "users_copy" (LIKE "users" INCLUDING ALL EXCLUDING INDEXES)
```

**PARTITION BY:**

```rust
let stmt = SchemaMutationStmt::CreateTable {
    schema,
    partition_by: Some(PartitionByDef {
        strategy: PartitionStrategy::Range,
        columns: vec![PartitionColumnDef {
            expr: IndexExpr::Column("created_at".into()),
            collation: None,
            opclass: None,
        }],
    }),
    // ...
};
```

```sql
CREATE TABLE "logs" ("id" BIGINT NOT NULL, "created_at" TIMESTAMPTZ NOT NULL, "message" TEXT) PARTITION BY RANGE ("created_at")
```

Partition by expression is also supported:

```rust
PartitionColumnDef {
    expr: IndexExpr::Expression(Expr::Func {
        name: "lower".into(),
        args: vec![Expr::Raw { sql: "region".into(), params: vec![] }],
    }),
    collation: None,
    opclass: None,
}
```

```sql
PARTITION BY LIST ((lower(region)))
```

**Tablespace:**

```rust
let stmt = SchemaMutationStmt::CreateTable {
    schema,
    tablespace: Some("fast_storage".into()),
    // ...
};
```

```sql
CREATE TABLE "big" ("id" INT) TABLESPACE "fast_storage"
```

**Storage params (WITH):**

```rust
let stmt = SchemaMutationStmt::CreateTable {
    schema,
    with_options: Some(vec![
        ("fillfactor".into(), "70".into()),
        ("autovacuum_enabled".into(), "true".into()),
    ]),
    // ...
};
```

```sql
CREATE TABLE "hot_data" ("id" INT) WITH (fillfactor = 70, autovacuum_enabled = true)
```

**USING (access method):**

```rust
let stmt = SchemaMutationStmt::CreateTable {
    schema,
    using_method: Some("columnar".into()),
    // ...
};
```

```sql
CREATE TABLE "columnar_data" ("id" INT) USING columnar
```

**Column STORAGE and COMPRESSION (PG):**

```rust
ColumnDef {
    name: "body".into(),
    field_type: FieldType::scalar("TEXT"),
    storage: Some("EXTERNAL".into()),
    compression: Some("lz4".into()),
    // ... other fields ...
}
```

```sql
"body" TEXT STORAGE EXTERNAL COMPRESSION lz4
```

---

## 2. Constraints

Constraints are defined in `SchemaDef.constraints` as a `Vec<ConstraintDef>` and appear inside the `CREATE TABLE` parentheses.

### PRIMARY KEY

```rust
schema.constraints = Some(vec![ConstraintDef::primary_key(vec!["id"])]);
```

Without a name:
```sql
PRIMARY KEY ("id")
```

With a name:
```rust
ConstraintDef::PrimaryKey {
    name: Some("pk_users".into()),
    columns: vec!["id".into()],
    include: None,
    autoincrement: false,
}
```

```sql
CONSTRAINT "pk_users" PRIMARY KEY ("id")
```

### FOREIGN KEY

```rust
ConstraintDef::foreign_key(vec!["user_id"], "users", vec!["id"])
```

Minimal (no actions):
```sql
FOREIGN KEY ("user_id") REFERENCES "users" ("id")
```

With ON DELETE/UPDATE actions, name, and deferrable:

```rust
ConstraintDef::ForeignKey {
    name: Some("fk_posts_user".into()),
    columns: vec!["user_id".into()],
    ref_table: SchemaRef::new("users"),
    ref_columns: vec!["id".into()],
    on_delete: Some(ReferentialAction::Cascade),
    on_update: Some(ReferentialAction::NoAction),
    deferrable: None,
    match_type: None,
}
```

```sql
CONSTRAINT "fk_posts_user" FOREIGN KEY ("user_id") REFERENCES "users" ("id") ON DELETE CASCADE ON UPDATE NO ACTION
```

Available `ReferentialAction` variants: `NoAction`, `Restrict`, `Cascade`, `SetNull(Option<Vec<String>>)`, `SetDefault(Option<Vec<String>>)`.

### UNIQUE

```rust
ConstraintDef::unique(vec!["email"])
```

```sql
UNIQUE ("email")
```

With a name:
```rust
ConstraintDef::Unique {
    name: Some("uq_email".into()),
    columns: vec!["email".into()],
    include: None,
    nulls_distinct: None,
    condition: None,
}
```

```sql
CONSTRAINT "uq_email" UNIQUE ("email")
```

### CHECK

```rust
ConstraintDef::check(Conditions {
    children: vec![ConditionNode::Comparison(Comparison {
        left: Expr::Raw { sql: "\"age\"".into(), params: vec![] },
        op: CompareOp::Gt,
        right: Expr::Value(Value::Int(0)),
        negate: false,
    })],
    connector: Connector::And,
    negated: false,
})
```

```sql
CHECK ("age" > 0)
```

With a name:
```sql
CONSTRAINT "ck_age" CHECK ("age" > 0)
```

### EXCLUSION (PostgreSQL only)

Exclusion constraints are PG-specific. SQLite returns an error.

```rust
ConstraintDef::Exclusion {
    name: None,
    elements: vec![ExclusionElement {
        column: "range_col".into(),
        operator: "&&".into(),
        opclass: None,
    }],
    index_method: "gist".into(),
    condition: None,
}
```

### Deferrable constraints

Both PG and SQLite support deferrable foreign keys:

```rust
ConstraintDef::ForeignKey {
    name: None,
    columns: vec!["ref_id".into()],
    ref_table: SchemaRef::new("other"),
    ref_columns: vec!["id".into()],
    on_delete: None,
    on_update: None,
    deferrable: Some(DeferrableConstraint {
        deferrable: true,
        initially_deferred: true,
    }),
    match_type: None,
}
```

SQLite:
```sql
FOREIGN KEY ("ref_id") REFERENCES "other" ("id") DEFERRABLE INITIALLY DEFERRED
```

PostgreSQL (with MATCH FULL):
```sql
CONSTRAINT "fk_user" FOREIGN KEY ("user_id") REFERENCES "users" ("id") MATCH FULL ON DELETE SET NULL DEFERRABLE INITIALLY DEFERRED
```

---

## 3. ALTER TABLE

### ADD COLUMN

```rust
let stmt = SchemaMutationStmt::add_column(
    "users",
    ColumnDef::new("email", FieldType::scalar("TEXT")),
);
```

SQLite:
```sql
ALTER TABLE "users" ADD COLUMN "email" TEXT
```

PostgreSQL supports `IF NOT EXISTS`:

```rust
let stmt = SchemaMutationStmt::AddColumn {
    schema_ref: SchemaRef::new("users"),
    column: ColumnDef::new("email", FieldType::scalar("TEXT")).not_null(),
    if_not_exists: true,
    position: None,
};
```

```sql
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "email" TEXT NOT NULL
```

### DROP COLUMN

```rust
let stmt = SchemaMutationStmt::drop_column("users", "old_field");
```

```sql
ALTER TABLE "users" DROP COLUMN "old_field"
```

PostgreSQL supports `IF EXISTS` and `CASCADE`:

```rust
let stmt = SchemaMutationStmt::DropColumn {
    schema_ref: SchemaRef::new("users"),
    name: "old_col".into(),
    if_exists: true,
    cascade: true,
};
```

```sql
ALTER TABLE "users" DROP COLUMN IF EXISTS "old_col" CASCADE
```

### RENAME TABLE

```rust
let stmt = SchemaMutationStmt::rename_table("old_name", "new_name");
```

```sql
ALTER TABLE "old_name" RENAME TO "new_name"
```

### RENAME COLUMN

```rust
let stmt = SchemaMutationStmt::rename_column("users", "name", "full_name");
```

```sql
ALTER TABLE "users" RENAME COLUMN "name" TO "full_name"
```

### ALTER COLUMN type/default/nullability (PostgreSQL only)

These operations error on SQLite.

**Change type:**

```rust
let stmt = SchemaMutationStmt::AlterColumnType {
    schema_ref: SchemaRef::new("users"),
    column_name: "age".into(),
    new_type: FieldType::scalar("BIGINT"),
    using_expr: None,
};
```

```sql
ALTER TABLE "users" ALTER COLUMN "age" SET DATA TYPE BIGINT
```

With USING:

```rust
let stmt = SchemaMutationStmt::AlterColumnType {
    schema_ref: SchemaRef::new("t"),
    column_name: "x".into(),
    new_type: FieldType::scalar("INTEGER"),
    using_expr: Some(Expr::Raw { sql: "x::INTEGER".into(), params: vec![] }),
};
```

```sql
ALTER TABLE "t" ALTER COLUMN "x" SET DATA TYPE INTEGER USING x::INTEGER
```

**Set/drop default:**

```rust
// Set default
let stmt = SchemaMutationStmt::AlterColumnDefault {
    schema_ref: SchemaRef::new("users"),
    column_name: "status".into(),
    default: Some(Expr::Value(Value::Str("active".into()))),
};
```

```sql
ALTER TABLE "users" ALTER COLUMN "status" SET DEFAULT 'active'
```

```rust
// Drop default
let stmt = SchemaMutationStmt::AlterColumnDefault {
    schema_ref: SchemaRef::new("users"),
    column_name: "status".into(),
    default: None,
};
```

```sql
ALTER TABLE "users" ALTER COLUMN "status" DROP DEFAULT
```

**Set/drop NOT NULL:**

```rust
let stmt = SchemaMutationStmt::AlterColumnNullability {
    schema_ref: SchemaRef::new("users"),
    column_name: "email".into(),
    not_null: true,  // false for DROP NOT NULL
};
```

```sql
ALTER TABLE "users" ALTER COLUMN "email" SET NOT NULL
-- or with not_null: false:
ALTER TABLE "users" ALTER COLUMN "email" DROP NOT NULL
```

### ADD/DROP CONSTRAINT (PostgreSQL only)

These operations error on SQLite.

**ADD CONSTRAINT:**

```rust
let stmt = SchemaMutationStmt::AddConstraint {
    schema_ref: SchemaRef::new("users"),
    constraint: ConstraintDef::Unique {
        name: Some("uq_email".into()),
        columns: vec!["email".into()],
        include: None,
        nulls_distinct: None,
        condition: None,
    },
    not_valid: false,
};
```

```sql
ALTER TABLE "users" ADD CONSTRAINT "uq_email" UNIQUE ("email")
```

With `NOT VALID` (useful for adding FK constraints without locking):

```rust
let stmt = SchemaMutationStmt::AddConstraint {
    schema_ref: SchemaRef::new("orders"),
    constraint: ConstraintDef::ForeignKey {
        name: Some("fk_user".into()),
        columns: vec!["user_id".into()],
        ref_table: SchemaRef::new("users"),
        ref_columns: vec!["id".into()],
        on_delete: None,
        on_update: None,
        deferrable: None,
        match_type: None,
    },
    not_valid: true,
};
```

```sql
ALTER TABLE "orders" ADD CONSTRAINT "fk_user" FOREIGN KEY ("user_id") REFERENCES "users" ("id") NOT VALID
```

Then validate later:

```rust
let stmt = SchemaMutationStmt::ValidateConstraint {
    schema_ref: SchemaRef::new("orders"),
    constraint_name: "fk_user".into(),
};
```

```sql
ALTER TABLE "orders" VALIDATE CONSTRAINT "fk_user"
```

**DROP CONSTRAINT:**

```rust
let stmt = SchemaMutationStmt::DropConstraint {
    schema_ref: SchemaRef::new("users"),
    constraint_name: "uq_email".into(),
    if_exists: true,
    cascade: false,
};
```

```sql
ALTER TABLE "users" DROP CONSTRAINT IF EXISTS "uq_email"
```

---

## 4. CREATE INDEX

### Simple index

```rust
let stmt = SchemaMutationStmt::create_index(
    "users",
    IndexDef::new("idx_users_email", vec![IndexColumnDef::column("email")]),
);
```

```sql
CREATE INDEX "idx_users_email" ON "users" ("email")
```

### Unique index

```rust
let stmt = SchemaMutationStmt::create_index(
    "users",
    IndexDef::new("idx_email", vec![IndexColumnDef::column("email")]).unique(),
);
```

```sql
CREATE UNIQUE INDEX "idx_email" ON "users" ("email")
```

### Partial index (WHERE)

```rust
let stmt = SchemaMutationStmt::CreateIndex {
    schema_ref: SchemaRef::new("users"),
    index: IndexDef {
        name: "idx_active_email".into(),
        columns: vec![IndexColumnDef::column("email")],
        unique: true,
        index_type: None,
        include: Some(vec!["name".into()]),
        condition: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw { sql: "\"active\"".into(), params: vec![] },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(true)),
                negate: false,
            })],
            connector: Connector::And,
            negated: false,
        }),
        parameters: None,
        tablespace: None,
        nulls_distinct: None,
    },
    if_not_exists: false,
    concurrently: false,
};
```

PostgreSQL (includes INCLUDE clause):
```sql
CREATE UNIQUE INDEX "idx_active_email" ON "users" ("email") INCLUDE ("name") WHERE "active" = TRUE
```

SQLite (INCLUDE silently ignored, booleans rendered as integers):
```sql
CREATE INDEX "idx_active" ON "users" ("email") WHERE "active" = 1
```

### Expression index

```rust
let stmt = SchemaMutationStmt::create_index(
    "users",
    IndexDef::new(
        "idx_lower_email",
        vec![IndexColumnDef::expression(Expr::Func {
            name: "lower".into(),
            args: vec![Expr::Raw { sql: "\"email\"".into(), params: vec![] }],
        })],
    ),
);
```

```sql
CREATE INDEX "idx_lower_email" ON "users" ((lower("email")))
```

### Multi-column with direction and nulls ordering

```rust
let stmt = SchemaMutationStmt::create_index(
    "events",
    IndexDef::new(
        "idx_events_composite",
        vec![
            IndexColumnDef::column("created_at").desc().nulls_last(),
            IndexColumnDef::column("priority").asc(),
        ],
    ),
);
```

PostgreSQL:
```sql
CREATE INDEX "idx_events_composite" ON "events" ("created_at" DESC NULLS LAST, "priority" ASC)
```

### Index type: btree, hash, gin, gist (PostgreSQL)

```rust
let stmt = SchemaMutationStmt::CreateIndex {
    schema_ref: SchemaRef::new("docs"),
    index: IndexDef {
        name: "idx_docs_content".into(),
        columns: vec![IndexColumnDef {
            expr: IndexExpr::Column("content".into()),
            direction: None,
            nulls: None,
            opclass: Some("gin_trgm_ops".into()),
            collation: None,
        }],
        unique: false,
        index_type: Some("GIN".into()),
        include: None,
        condition: None,
        parameters: None,
        tablespace: None,
        nulls_distinct: None,
    },
    if_not_exists: false,
    concurrently: false,
};
```

```sql
CREATE INDEX "idx_docs_content" ON "docs" USING GIN ("content" gin_trgm_ops)
```

On SQLite, `index_type`, `include`, `parameters`, and `tablespace` are silently ignored.

### CONCURRENTLY (PostgreSQL)

```rust
let stmt = SchemaMutationStmt::CreateIndex {
    schema_ref: SchemaRef::new("users"),
    index: IndexDef::new("idx_email", vec![IndexColumnDef::column("email")]).unique(),
    if_not_exists: true,
    concurrently: true,
};
```

```sql
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS "idx_email" ON "users" ("email")
```

On SQLite, `concurrently` is silently ignored.

---

## 5. DROP TABLE / DROP INDEX

### DROP TABLE

```rust
let stmt = SchemaMutationStmt::drop_table("users");
```

```sql
DROP TABLE "users"
```

With IF EXISTS:

```rust
let stmt = SchemaMutationStmt::drop_table_if_exists("users");
```

```sql
DROP TABLE IF EXISTS "users"
```

With CASCADE (PostgreSQL only; silently ignored on SQLite):

```rust
let stmt = SchemaMutationStmt::DropTable {
    schema_ref: SchemaRef::new("users"),
    if_exists: true,
    cascade: true,
};
```

PostgreSQL:
```sql
DROP TABLE IF EXISTS "users" CASCADE
```

SQLite:
```sql
DROP TABLE IF EXISTS "users"
```

### DROP INDEX

```rust
let stmt = SchemaMutationStmt::drop_index("users", "idx_email");
```

```sql
DROP INDEX "idx_email"
```

PostgreSQL supports CONCURRENTLY, IF EXISTS, and CASCADE:

```rust
let stmt = SchemaMutationStmt::DropIndex {
    schema_ref: SchemaRef::new("users"),
    index_name: "idx_email".into(),
    if_exists: true,
    concurrently: true,
    cascade: true,
};
```

PostgreSQL:
```sql
DROP INDEX CONCURRENTLY IF EXISTS "idx_email" CASCADE
```

SQLite (CONCURRENTLY and CASCADE silently ignored):
```sql
DROP INDEX IF EXISTS "idx_email"
```

---

## 6. TRUNCATE

### PostgreSQL

```rust
let stmt = SchemaMutationStmt::truncate("users");
```

```sql
TRUNCATE TABLE "users"
```

With RESTART IDENTITY and CASCADE:

```rust
let stmt = SchemaMutationStmt::TruncateTable {
    schema_ref: SchemaRef::new("orders"),
    restart_identity: true,
    cascade: true,
};
```

```sql
TRUNCATE TABLE "orders" RESTART IDENTITY CASCADE
```

### SQLite

SQLite has no `TRUNCATE` statement. rquery renders it as `DELETE FROM`:

```rust
let stmt = SchemaMutationStmt::truncate("users");
```

```sql
DELETE FROM "users"
```

The `restart_identity` and `cascade` options are silently ignored on SQLite.
