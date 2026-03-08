# Multi-Dialect Support

rquery uses a single AST to represent SQL queries, then renders dialect-specific SQL via separate renderer implementations. The same `QueryStmt`, `MutationStmt`, or `SchemaMutationStmt` can be passed to `PostgresRenderer` or `SqliteRenderer` to produce valid SQL for each database.

## Three categories of cross-dialect behavior

When a feature exists in the AST but the target dialect does not support it natively, the renderer responds in one of three ways:

### 1. Supported

The feature works natively in the target dialect and renders as expected.

### 2. Ignored (silently skipped)

The feature is safely omitted without changing query semantics. The renderer skips it with no error. Examples:

- CTE `MATERIALIZED` / `NOT MATERIALIZED` hints on SQLite (SQLite has no planner control for CTEs)
- `ONLY` keyword on SQLite (SQLite has no table inheritance)
- Optimizer hints and PG-specific decorations that have no SQLite equivalent

### 3. Error (returns `RenderError::Unsupported`)

The feature would change query semantics if omitted, so the renderer returns an error rather than producing incorrect SQL. Examples:

- `DISTINCT ON` on SQLite -- would return different rows than intended
- `FOR UPDATE` / `FOR SHARE` on SQLite -- no row-level locking
- `LATERAL` joins on SQLite -- changes join evaluation semantics
- `GROUPING SETS` / `CUBE` / `ROLLUP` on SQLite -- no advanced grouping
- `TABLESAMPLE` on SQLite -- no sampling support

## Feature status table

| Feature | PostgreSQL | SQLite |
|---|---|---|
| DISTINCT | Supported | Supported |
| DISTINCT ON | Supported | Error |
| CTE (WITH) | Supported | Supported |
| CTE MATERIALIZED | Supported | Ignored |
| Recursive CTE | Supported | Supported |
| LIMIT / OFFSET | Supported | Supported |
| FETCH FIRST ... ROWS | Supported | Workaround (converts to LIMIT) |
| TOP(n) | Workaround (converts to LIMIT) | Workaround (converts to LIMIT) |
| FOR UPDATE / FOR SHARE | Supported | Error |
| TABLESAMPLE | Supported | Error |
| LATERAL | Supported | Error |
| GROUPING SETS | Supported | Error |
| CUBE | Supported | Error |
| ROLLUP | Supported | Error |
| Window functions | Supported | Supported |
| INDEXED BY | Ignored | Supported |
| NOT INDEXED | Ignored | Supported |
| WITHOUT ROWID | Ignored | Supported |
| STRICT | Ignored | Supported |
| OR REPLACE / OR IGNORE | Not applicable | Supported |
| RETURNING | Supported | Supported |
| ON CONFLICT (upsert) | Supported | Supported |
| ARRAY subquery | Supported | Error |
| Exclusion constraints | Supported | Error |
| CREATE EXTENSION | Supported | Error |
| JSONB operators | Supported | Error |
| Trigram operators | Supported | Error |
| Full-text search match | Supported | Error |
| Range operators | Supported | Error |
| ILIKE | Supported | Error |
| Regex match (~, ~*) | Supported | Error |

## Workarounds

Some features are automatically transformed to equivalent syntax:

- **TOP(n)** -- Both renderers convert `LimitKind::Top` to a `LIMIT` clause. This allows SQL Server-style AST nodes to render on PostgreSQL and SQLite.
- **FETCH FIRST n ROWS ONLY** -- SQLite converts `LimitKind::FetchFirst` to `LIMIT n`. PostgreSQL renders it natively.

## Writing portable queries

To write queries that work on both PostgreSQL and SQLite, stick to the common subset:

- Use `LIMIT` / `OFFSET` instead of `FETCH FIRST` or `TOP`
- Avoid `DISTINCT ON` -- use `GROUP BY` or window functions instead
- Avoid `FOR UPDATE` -- handle concurrency at the application level on SQLite
- Avoid `LATERAL` joins -- restructure as subqueries in SELECT or use CTEs
- Avoid `GROUPING SETS`, `CUBE`, `ROLLUP` -- use multiple queries or UNION ALL
- Use standard comparison operators (`=`, `>`, `<`, `LIKE`) -- avoid PG-specific operators (`ILIKE`, `~`, `@>`)
- Avoid `TABLESAMPLE` -- filter with `random()` or application-level sampling

## SQLite-specific features

Features unique to or specific to SQLite:

- **INDEXED BY / NOT INDEXED** -- Index hints in FROM clauses via `FromItem::index_hint`
- **WITHOUT ROWID** -- Set `without_rowid: true` on `SchemaMutationStmt::CreateTable`
- **STRICT** -- Set `strict: true` on `SchemaMutationStmt::CreateTable`
- **Conflict resolution** -- `INSERT OR REPLACE`, `INSERT OR IGNORE`, etc. via `InsertStmt::conflict_resolution` and `UpdateStmt::conflict_resolution` with the `ConflictResolution` enum (`Rollback`, `Abort`, `Fail`, `Ignore`, `Replace`)
- **DEFERRED / IMMEDIATE / EXCLUSIVE transactions** -- Via `BeginStmt::sqlite_deferred()`, `BeginStmt::sqlite_immediate()`, `BeginStmt::sqlite_exclusive()`

## PostgreSQL-specific features

Features unique to or specific to PostgreSQL:

- **DISTINCT ON** -- `DistinctDef::DistinctOn(vec![...])` in `QueryStmt::distinct`
- **TABLESAMPLE** -- `TableSampleDef` with `SampleMethod::Bernoulli` or `SampleMethod::System` on `FromItem::sample`
- **GROUPING SETS / CUBE / ROLLUP** -- `GroupByItem::GroupingSets`, `GroupByItem::Cube`, `GroupByItem::Rollup`
- **FOR UPDATE / FOR SHARE / FOR NO KEY UPDATE / FOR KEY SHARE** -- `SelectLockDef` with `LockStrength` variants, plus `nowait` and `skip_locked`
- **LATERAL** -- `TableSource::Lateral(Box<FromItem>)` or `FromItem::lateral(inner)`
- **CTE MATERIALIZED / NOT MATERIALIZED** -- `CteDef::materialized()`, `CteDef::not_materialized()`
- **ARRAY(subquery)** -- `Expr::ArraySubQuery`
- **JSONB operators** -- `CompareOp::JsonbContains`, `JsonbContainedBy`, `JsonbHasKey`, `JsonbHasAnyKey`, `JsonbHasAllKeys`
- **Trigram operators** -- `CompareOp::TrigramSimilar`, `TrigramWordSimilar`, `TrigramStrictWordSimilar`
- **Full-text search** -- `CompareOp::FtsMatch`
- **Range operators** -- `CompareOp::RangeContains`, `RangeContainedBy`, `RangeOverlap`
- **Two-phase commit** -- `PrepareTransaction`, `CommitPrepared`, `RollbackPrepared`
- **LOCK TABLE** -- `LockTableStmt` with PostgreSQL lock modes
- **Exclusion constraints** -- `ConstraintDef::Exclusion`
- **CREATE / DROP EXTENSION** -- `SchemaMutationStmt::CreateExtension`, `DropExtension`
- **Partition tables** -- `PartitionByDef` on `CreateTable`
- **OVERRIDING SYSTEM/USER VALUE** -- `InsertStmt::overriding`

---

## Planned dialects (MySQL, Oracle, SQL Server)

> The tables below describe the **planned** behavior for renderers that are not yet implemented. They are included as a roadmap — the AST already models most of these features, but the corresponding renderers do not exist yet.

### DQL feature matrix (planned)

#### Error — no equivalent, would produce wrong results

| Feature | Supported by | Reason |
|---|---|---|
| `DISTINCT ON (expr, ...)` | PG | No equivalent without rewriting as window function |
| `TABLESAMPLE` / `SAMPLE` | PG, Oracle, SQL Server | MySQL — no mechanism for random sampling |
| `GROUPING SETS (...)` | PG, Oracle, SQL Server | MySQL — not supported |
| `CUBE (...)` | PG, Oracle, SQL Server | MySQL — not supported |
| `ROLLUP (...)` | PG, Oracle, SQL Server, MySQL | SQLite — not supported |
| `PIVOT` / `UNPIVOT` | Oracle, SQL Server | PG, SQLite, MySQL — no native syntax |
| `FOR NO KEY UPDATE` | PG | Granularity unavailable elsewhere |
| `FOR KEY SHARE` | PG | Granularity unavailable elsewhere |
| `SKIP LOCKED` | PG, MySQL, Oracle | SQLite — no row locking; SQL Server `READPAST` has different semantics |
| `FETCH FIRST ... PERCENT` | Oracle | No trivial equivalent |
| `FOR UPDATE WAIT N` | Oracle | Others either NOWAIT or wait forever |
| `INTERSECT ALL` | PG, MySQL 8.0.31+ | SQLite, Oracle, SQL Server — not supported |
| `EXCEPT ALL` | PG, MySQL 8.0.31+ | SQLite, Oracle, SQL Server — not supported |
| `NATURAL JOIN` | PG, SQLite, MySQL, Oracle | SQL Server — not supported |
| `USING (col, ...)` in JOIN | PG, SQLite, MySQL, Oracle | SQL Server — not supported |

#### Ignore — optimizer hints, safe to skip

| Feature | Supported by | Notes |
|---|---|---|
| CTE `MATERIALIZED` / `NOT MATERIALIZED` | PG | Optimizer hint, same result without it |
| `INDEXED BY` / `NOT INDEXED` | SQLite | Index hint |
| `USE INDEX` / `IGNORE INDEX` / `FORCE INDEX` | MySQL | Index hint |
| `WITH (table_hint)` — performance hints | SQL Server | `NOLOCK`, `ROWLOCK`, `TABLOCK`, etc. |
| `HIGH_PRIORITY` | MySQL | Scheduling hint |
| `STRAIGHT_JOIN` | MySQL | Join order hint |
| `SQL_SMALL_RESULT` / `SQL_BIG_RESULT` / `SQL_BUFFER_RESULT` | MySQL | Temp table hints |
| `SQL_NO_CACHE` / `SQL_CALC_FOUND_ROWS` | MySQL | Cache/counting hints |
| `OPTION (query_hint, ...)` | SQL Server | `RECOMPILE`, `MAXDOP`, etc. |

#### Workaround — automatic syntax transformation

| Feature | Transformation |
|---|---|
| `LIMIT n` | Oracle: `FETCH FIRST n ROWS ONLY`; SQL Server: `TOP(n)` or `FETCH FIRST` |
| `FETCH FIRST n ROWS ONLY` | MySQL: `LIMIT n` |
| `TOP(n)` | Non-SS: `LIMIT n` or `FETCH FIRST` |
| `FETCH FIRST ... WITH TIES` | MySQL/SQLite: ERROR (no equivalent) |
| `EXCEPT` ↔ `MINUS` | Oracle: `MINUS`; others: `EXCEPT` |
| `LATERAL` ↔ `CROSS/OUTER APPLY` | PG/MySQL/Oracle: `LATERAL`; SQL Server: `CROSS APPLY`/`OUTER APPLY`; SQLite: ERROR |
| `WINDOW w AS (...)` → Oracle | Inline window definition into each `OVER(...)` |
| `NULLS FIRST` / `NULLS LAST` | MySQL/SQL Server: `ORDER BY (CASE WHEN col IS NULL ...)` |
| `FOR UPDATE` → SQL Server | `WITH (UPDLOCK, ROWLOCK)` table hint |
| `FOR SHARE` → SQL Server | `WITH (HOLDLOCK)` table hint |

### Per-renderer decision matrix (planned)

#### MySQL renderer

| Feature | Strategy |
|---|---|
| `DISTINCT ON` | ERROR |
| `TABLESAMPLE` | ERROR |
| `CUBE` / `GROUPING SETS` | ERROR |
| `ROLLUP` | WORKAROUND → `WITH ROLLUP` syntax |
| `PIVOT` / `UNPIVOT` | ERROR |
| `FOR NO KEY UPDATE` / `KEY SHARE` | ERROR |
| `NULLS FIRST` / `NULLS LAST` | WORKAROUND → CASE expression |
| `LATERAL` | Native (8.0.14+) |
| `WINDOW` clause | Native (8.0+) |
| `FETCH FIRST ... WITH TIES` | ERROR |
| `FETCH FIRST ... PERCENT` | ERROR |
| `NATURAL JOIN` / `USING` | Native |
| `CTE MATERIALIZED` | IGNORE |

#### Oracle renderer

| Feature | Strategy |
|---|---|
| `DISTINCT ON` | ERROR |
| `WINDOW` clause (named windows) | WORKAROUND → inline into OVER() |
| `LIMIT n` | WORKAROUND → `FETCH FIRST n ROWS ONLY` |
| `INTERSECT ALL` / `EXCEPT ALL` | ERROR |
| `EXCEPT` | WORKAROUND → `MINUS` |
| `LATERAL` | Native |
| `FOR UPDATE` / `SKIP LOCKED` / `NOWAIT` | Native |
| `NULLS FIRST` / `NULLS LAST` | Native |
| `CTE MATERIALIZED` | IGNORE |

#### SQL Server renderer

| Feature | Strategy |
|---|---|
| `DISTINCT ON` | ERROR |
| `LIMIT n` | WORKAROUND → `TOP(n)` or `FETCH FIRST` |
| `NULLS FIRST` / `NULLS LAST` | WORKAROUND → CASE expression |
| `NATURAL JOIN` | ERROR |
| `USING` in JOIN | ERROR |
| `LATERAL` | WORKAROUND → `CROSS/OUTER APPLY` |
| `FOR UPDATE` / `FOR SHARE` | WORKAROUND → table hints |
| `SKIP LOCKED` | WORKAROUND → `READPAST` hint (slightly different semantics) |
| `EXCEPT` / `INTERSECT` | Native |
| `INTERSECT ALL` / `EXCEPT ALL` | ERROR |
| `WINDOW` clause | Native (2022) |
| `CTE MATERIALIZED` | IGNORE |
| `FETCH FIRST ... PERCENT` | ERROR |

### DDL feature matrix (planned)

#### CREATE TABLE

| Feature | PG | SQLite | MySQL | Oracle | SQL Server | Default if unsupported |
|---|---|---|---|---|---|---|
| IF NOT EXISTS | Y | Y | Y | Y | N | Warn |
| TEMPORARY | Y | Y | Y | Y | Y (# prefix) | dialect-specific |
| UNLOGGED | Y | N | N | N | N | Ignore |
| TABLESPACE | Y | N | Y | Y | Y | Ignore |
| Column COLLATE | Y | Y | Y | Y | Y | Ignore |
| Column COMMENT | Y (ext) | N | Y | N | N | Ignore |
| Column STORAGE | Y | N | N | N | N | Ignore |
| Column COMPRESSION | Y | N | Y | Y | Y | Ignore |
| GENERATED STORED | Y | Y | Y | Y | Y (PERSISTED) | supported |
| GENERATED VIRTUAL | N (PG) | Y | Y | Y | Y | Warn |
| IDENTITY / AUTO_INCREMENT | Y | Y (INTEGER PK) | Y | Y | Y | dialect-specific |
| INHERITS | Y | N | N | N | N | Error |
| LIKE | Y | N | Y | N | N | Error |
| PARTITION BY | Y | N | Y | Y | Y | Warn |
| Table access method (USING/ENGINE) | Y | N | Y | Y | Y | Ignore |
| WITH storage parameters | Y | N | Y | Y | Y | Ignore |
| ON COMMIT (temp tables) | Y | N | N | Y | N | Ignore |

#### Column Constraints

| Feature | PG | SQLite | MySQL | Oracle | SQL Server | Default if unsupported |
|---|---|---|---|---|---|---|
| NOT NULL / DEFAULT / CHECK / UNIQUE / PK / FK | Y | Y | Y | Y | Y | supported |
| DEFERRABLE | Y | Y (FK) | N | Y | N | Ignore |
| INITIALLY DEFERRED | Y | Y (FK) | N | Y | N | Ignore |
| NO INHERIT (CHECK) | Y | N | N | N | N | Ignore |
| NULLS [NOT] DISTINCT | Y | N | N | N | N | Ignore |
| ON CONFLICT clause (SQLite) | N | Y | N | N | N | Error |
| CHECK [NOT] ENFORCED (MySQL) | N | N | Y | N | N | Ignore |
| NOT FOR REPLICATION (SQL Server) | N | N | N | N | Y | Ignore |

#### Foreign Key Actions

| Feature | PG | SQLite | MySQL | Oracle | SQL Server | Default if unsupported |
|---|---|---|---|---|---|---|
| ON DELETE CASCADE / SET NULL | Y | Y | Y | Y | Y | supported |
| ON DELETE SET DEFAULT | Y | Y | Y | N | Y | Warn |
| ON DELETE RESTRICT | Y | Y | Y | N | Y | Warn |
| ON UPDATE CASCADE / SET NULL / SET DEFAULT / RESTRICT | Y | Y | Y | **N** | Y | Error (Oracle) |
| MATCH FULL / PARTIAL / SIMPLE | Y | Y | Y | N | N | Ignore |

#### CREATE INDEX

| Feature | PG | SQLite | MySQL | Oracle | SQL Server | Default if unsupported |
|---|---|---|---|---|---|---|
| IF NOT EXISTS | Y | Y | N | Y | N | Warn |
| UNIQUE | Y | Y | Y | Y | Y | supported |
| WHERE (partial) | Y | Y | N | N | Y | Error |
| Expression index | Y | Y | Y | Y | via computed col | Error |
| INCLUDE (covering) | Y | N | N | N | Y | Ignore |
| CONCURRENTLY | Y | N | N | N | N | Ignore |
| ONLINE | N | N | Y | Y | Y | Ignore |
| ASC/DESC per column | Y | Y | Y | Y | Y | supported |
| NULLS FIRST/LAST | Y | N | N | N | N | Ignore |
| Index type (USING method) | Y | N | Y | Y | Y | Warn |
| Operator class | Y | N | N | N | N | Ignore |

#### ALTER TABLE

| Feature | PG | SQLite | MySQL | Oracle | SQL Server | Default if unsupported |
|---|---|---|---|---|---|---|
| ADD COLUMN | Y | Y (limited) | Y | Y | Y | supported |
| DROP COLUMN | Y | Y (limited) | Y | Y | Y | supported |
| Change column type | Y | **N** | Y | Y | Y | Error |
| Set/Drop DEFAULT | Y | **N** | Y | Y | Y | Error |
| Set/Drop NOT NULL | Y | **N** | Y | Y | Y | Error |
| RENAME COLUMN | Y | Y | Y | Y | Y (sp_rename) | supported |
| RENAME TABLE | Y | Y | Y | Y | Y (sp_rename) | supported |
| ADD / DROP CONSTRAINT | Y | **N** | Y | Y | Y | Error |
| VALIDATE CONSTRAINT | Y | N | N | Y | Y | Ignore |
| NOT VALID (add constraint) | Y | N | N | Y | N | Ignore |

#### DROP TABLE / DROP INDEX

| Feature | PG | SQLite | MySQL | Oracle | SQL Server | Default if unsupported |
|---|---|---|---|---|---|---|
| IF EXISTS | Y | Y | Y | Y | Y | supported |
| CASCADE (DROP TABLE) | Y | N | N | Y | N | Warn |
| RESTRICT (DROP TABLE) | Y | N | N | N | N | Ignore |
| CONCURRENTLY (DROP INDEX) | Y | N | N | N | N | Ignore |
| CASCADE (DROP INDEX) | Y | N | N | N | N | Warn |
