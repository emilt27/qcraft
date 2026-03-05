# Unsupported Features Policy

## Mechanism

When the AST contains a feature not supported by the target dialect, the renderer applies a configurable policy:

```rust
enum UnsupportedPolicy {
    Ignore,  // silently skip, no output
    Warn,    // skip + add warning to RenderResult
    Error,   // return error, stop rendering
}

struct RenderResult {
    sql: String,
    params: Vec<Value>,
    warnings: Vec<Warning>,
}

struct Warning {
    feature: &'static str,
    message: String,
}
```

Each dialect has sensible defaults per feature. Users can override any default:

```rust
let renderer = PostgresRenderer::new();

let renderer = SqliteRenderer::builder()
    .policy(Feature::Deferrable, UnsupportedPolicy::Error)  // override default
    .build();
```

## Classification Criteria

**Ignore** = ignoring makes behavior **stricter** or removes an optimization. Result is correct.

**Warn** = ignoring is usually fine but represents a meaningful user intention that's being lost.

**Error** = ignoring changes **semantics** or breaks **data integrity**. Result may be incorrect.

## Feature Policy Table

### CREATE TABLE

| Feature | PG | SQLite | MySQL | Oracle | SQL Server | Default if unsupported |
|---|---|---|---|---|---|---|
| IF NOT EXISTS | Y | Y | Y | Y | N | **Warn** (workaround exists) |
| TEMPORARY | Y | Y | Y | Y | Y (# prefix) | dialect-specific syntax |
| UNLOGGED | Y | N | N | N | N | **Ignore** (logged = safer) |
| TABLESPACE | Y | N | Y | Y | Y | **Ignore** (uses default) |
| Column COLLATE | Y | Y | Y | Y | Y | **Ignore** (uses default collation) |
| Column COMMENT | Y (ext) | N | Y | N | N | **Ignore** (metadata only) |
| Column STORAGE (PLAIN/EXTERNAL/etc) | Y | N | N | N | N | **Ignore** (uses default) |
| Column COMPRESSION | Y | N | Y | Y | Y | **Ignore** (no compression) |
| GENERATED STORED | Y | Y | Y | Y | Y (PERSISTED) | supported everywhere |
| GENERATED VIRTUAL | N (PG) | Y | Y | Y | Y | **Warn** (PG only supports STORED) |
| IDENTITY / AUTO_INCREMENT | Y | Y (INTEGER PK) | Y | Y | Y | dialect-specific syntax |
| INHERITS | Y | N | N | N | N | **Error** (no equivalent) |
| LIKE | Y | N | Y | N | N | **Error** (no equivalent) |
| PARTITION BY | Y | N | Y | Y | Y | **Warn** (works without, but architectural intent lost) |
| Table access method (USING/ENGINE) | Y | N | Y | Y | Y | **Ignore** (uses default) |
| WITH storage parameters | Y | N | Y | Y | Y | **Ignore** (uses defaults) |
| ON COMMIT (temp tables) | Y | N | N | Y | N | **Ignore** (uses default behavior) |

### Column Constraints

| Feature | PG | SQLite | MySQL | Oracle | SQL Server | Default if unsupported |
|---|---|---|---|---|---|---|
| NOT NULL | Y | Y | Y | Y | Y | supported everywhere |
| DEFAULT | Y | Y | Y | Y | Y | supported everywhere |
| CHECK | Y | Y | Y | Y | Y | supported everywhere |
| UNIQUE | Y | Y | Y | Y | Y | supported everywhere |
| PRIMARY KEY | Y | Y | Y | Y | Y | supported everywhere |
| FOREIGN KEY | Y | Y | Y | Y | Y | supported everywhere |
| DEFERRABLE | Y | Y (FK only) | N | Y | N | **Ignore** (checked immediately = stricter) |
| INITIALLY DEFERRED | Y | Y (FK only) | N | Y | N | **Ignore** (same as DEFERRABLE) |
| NO INHERIT (CHECK) | Y | N | N | N | N | **Ignore** (CHECK applies everywhere = stricter) |
| NULLS [NOT] DISTINCT | Y | N | N | N | N | **Ignore** (uses dialect default behavior) |
| ON CONFLICT clause (SQLite) | N | Y | N | N | N | **Error** (SQLite-specific, no equivalent) |
| CHECK [NOT] ENFORCED (MySQL) | N | N | Y | N | N | **Ignore** (always enforced = stricter) |
| NOT FOR REPLICATION (SQL Server) | N | N | N | N | Y | **Ignore** (constraint always enforced) |

### Foreign Key Actions

| Feature | PG | SQLite | MySQL | Oracle | SQL Server | Default if unsupported |
|---|---|---|---|---|---|---|
| ON DELETE CASCADE | Y | Y | Y | Y | Y | supported everywhere |
| ON DELETE SET NULL | Y | Y | Y | Y | Y | supported everywhere |
| ON DELETE SET DEFAULT | Y | Y | Y | N | Y | **Warn** (Oracle: no SET DEFAULT) |
| ON DELETE RESTRICT | Y | Y | Y | N | Y | **Warn** (Oracle: use NO ACTION) |
| ON UPDATE CASCADE | Y | Y | Y | **N** | Y | **Error** (Oracle: no ON UPDATE at all) |
| ON UPDATE SET NULL | Y | Y | Y | **N** | Y | **Error** (Oracle: no ON UPDATE at all) |
| ON UPDATE SET DEFAULT | Y | Y | Y | **N** | Y | **Error** (Oracle: no ON UPDATE at all) |
| ON UPDATE RESTRICT | Y | Y | Y | **N** | Y | **Error** (Oracle: no ON UPDATE at all) |
| MATCH FULL/PARTIAL/SIMPLE | Y | Y | Y | N | N | **Ignore** (uses default match) |

### Table Constraints

| Feature | PG | SQLite | MySQL | Oracle | SQL Server | Default if unsupported |
|---|---|---|---|---|---|---|
| EXCLUSION | Y | N | N | N | N | **Error** (no equivalent, data integrity at risk) |
| UNIQUE with INCLUDE | Y | N | N | N | Y | **Ignore** (index works, not covering) |
| PK/UNIQUE index parameters | Y | N | Y | Y | Y | **Ignore** (uses defaults) |
| CONSTRAINT name | Y | Y | Y | Y | Y | supported everywhere |
| USING INDEX TABLESPACE | Y | N | N | Y | Y | **Ignore** (uses default tablespace) |

### CREATE INDEX

| Feature | PG | SQLite | MySQL | Oracle | SQL Server | Default if unsupported |
|---|---|---|---|---|---|---|
| IF NOT EXISTS | Y | Y | N | Y | N | **Warn** (workaround exists) |
| UNIQUE | Y | Y | Y | Y | Y | supported everywhere |
| WHERE (partial) | Y | Y | N | N | Y | **Error** (changes semantics, especially with UNIQUE) |
| Expression index | Y | Y | Y | Y | via computed col | **Error** (can't index expression without support) |
| INCLUDE (covering) | Y | N | N | N | Y | **Ignore** (index works, not covering) |
| CONCURRENTLY | Y | N | N | N | N | **Ignore** (builds with lock = slower but correct) |
| ONLINE | N | N | Y | Y | Y | **Ignore** (builds with lock = slower but correct) |
| ASC/DESC per column | Y | Y | Y | Y | Y | supported everywhere |
| NULLS FIRST/LAST | Y | N | N | N | N | **Ignore** (uses default null ordering) |
| Index type (USING method) | Y | N | Y | Y | Y | **Warn** (uses default type, may affect performance) |
| Operator class | Y | N | N | N | N | **Ignore** (uses default operator class) |
| Storage parameters (WITH) | Y | N | N | Y | Y | **Ignore** (uses defaults) |
| TABLESPACE | Y | N | N | Y | Y | **Ignore** (uses default) |
| INVISIBLE | N | N | Y | Y | N | **Ignore** (index is visible = still works) |
| BITMAP | N | N | N | Y | N | **Warn** (will use default index type) |
| CLUSTERED/NONCLUSTERED | N | N | N | N | Y | **Ignore** (uses default) |
| COMPRESSION | N | N | N | Y | Y | **Ignore** (no compression) |
| RESUMABLE | N | N | N | N | Y | **Ignore** (non-resumable) |

### ALTER TABLE

| Feature | PG | SQLite | MySQL | Oracle | SQL Server | Default if unsupported |
|---|---|---|---|---|---|---|
| ADD COLUMN | Y | Y (limited) | Y | Y | Y | supported everywhere |
| DROP COLUMN | Y | Y (limited) | Y | Y | Y | supported everywhere |
| Change column type | Y | **N** | Y | Y | Y | **Error** (can't modify schema) |
| Set/Drop DEFAULT | Y | **N** | Y | Y | Y | **Error** (can't modify schema) |
| Set/Drop NOT NULL | Y | **N** | Y | Y | Y | **Error** (can't modify schema) |
| RENAME COLUMN | Y | Y | Y | Y | Y (sp_rename) | supported everywhere |
| RENAME TABLE | Y | Y | Y | Y | Y (sp_rename) | supported everywhere |
| ADD CONSTRAINT | Y | **N** | Y | Y | Y | **Error** (can't add constraint) |
| DROP CONSTRAINT | Y | **N** | Y | Y | Y | **Error** (can't drop constraint) |
| VALIDATE CONSTRAINT | Y | N | N | Y | Y | **Ignore** (constraint already valid) |
| IF EXISTS on actions | Y | N | N | N | Y (DROP) | **Warn** (may fail if not exists) |
| FIRST/AFTER (column position) | N | N | Y | N | N | **Ignore** (added at end, order rarely matters) |
| NOT VALID (add constraint) | Y | N | N | Y | N | **Ignore** (constraint validated immediately = stricter) |
| CONCURRENTLY (detach partition) | Y | N | N | N | N | **Ignore** (detach with lock) |

### DROP TABLE

| Feature | PG | SQLite | MySQL | Oracle | SQL Server | Default if unsupported |
|---|---|---|---|---|---|---|
| IF EXISTS | Y | Y | Y | Y | Y | supported everywhere |
| CASCADE | Y | N | N (no-op) | Y (CONSTRAINTS) | N | **Warn** (must drop dependents manually) |
| RESTRICT | Y | N | N (no-op) | N | N | **Ignore** (default behavior is similar) |
| PURGE (Oracle) | N | N | N | Y | N | **Ignore** (uses recycle bin if available) |
| Multiple tables | Y | N | Y | N | Y | render as separate statements |

### DROP INDEX

| Feature | PG | SQLite | MySQL | Oracle | SQL Server | Default if unsupported |
|---|---|---|---|---|---|---|
| IF EXISTS | Y | Y | N | Y | Y | **Warn** (may fail if not exists) |
| CONCURRENTLY | Y | N | N | N | N | **Ignore** (drops with lock) |
| ONLINE | N | N | Y | Y | Y | **Ignore** (drops with lock) |
| CASCADE | Y | N | N | N | N | **Warn** (must drop dependents manually) |
| FORCE (Oracle domain) | N | N | N | Y | N | **Ignore** (not relevant outside Oracle) |

## Summary by Default Policy

### Ignore (37 features)
Features that are performance hints, storage optimizations, or make behavior stricter when absent. Safe to silently skip.

### Warn (13 features)
Features where ignoring is usually fine but the user's intention is meaningfully lost, or where a workaround exists and should be applied.

### Error (13 features)
Features where ignoring breaks data integrity, changes query semantics, or makes schema modification impossible.

## Override Example

```rust
// Default: DEFERRABLE is Ignore on SQLite
// User wants it to be an error because their app relies on deferred constraint checking
let renderer = SqliteRenderer::builder()
    .policy(Feature::Deferrable, UnsupportedPolicy::Error)
    .build();

// Default: PARTITION BY is Warn
// User doesn't care about partitioning in dev environment
let renderer = PostgresRenderer::builder()
    .policy(Feature::PartitionBy, UnsupportedPolicy::Ignore)
    .build();

// Make everything strict: any unsupported feature is an error
let renderer = SqliteRenderer::builder()
    .default_policy(UnsupportedPolicy::Error)
    .build();
```
