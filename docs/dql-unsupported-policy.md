# DQL Unsupported Feature Policy

How each renderer handles features not natively supported by its target database.

Three strategies: **ERROR** (return `RenderError::Unsupported`), **IGNORE** (silently skip), **WORKAROUND** (transform to equivalent syntax).

---

## ERROR — No equivalent, would produce wrong results or invalid SQL

| Feature | Supported by | Reason |
|---|---|---|
| `DISTINCT ON (expr, ...)` | PG | No equivalent without rewriting as window function (different query) |
| `TABLESAMPLE` / `SAMPLE` | PG, Oracle, SQL Server | SQLite, MySQL — no mechanism for random sampling |
| `GROUPING SETS (...)` | PG, Oracle, SQL Server | SQLite, MySQL — not supported |
| `CUBE (...)` | PG, Oracle, SQL Server | SQLite, MySQL — not supported |
| `ROLLUP (...)` | PG, Oracle, SQL Server, MySQL | SQLite — not supported |
| `PIVOT` / `UNPIVOT` | Oracle, SQL Server | PG, SQLite, MySQL — no native syntax |
| `FOR NO KEY UPDATE` | PG | Granularity unavailable elsewhere |
| `FOR KEY SHARE` | PG | Granularity unavailable elsewhere |
| `SKIP LOCKED` | PG, MySQL, Oracle | SQLite — no row locking; SQL Server `READPAST` has different semantics |
| `FETCH FIRST ... PERCENT` | Oracle | No trivial equivalent (would require COUNT subquery) |
| `FOR UPDATE WAIT N` | Oracle | Others either NOWAIT or wait forever — different semantics |
| `INTERSECT ALL` | PG, MySQL 8.0.31+ | SQLite, Oracle, SQL Server — not supported |
| `EXCEPT ALL` | PG, MySQL 8.0.31+ | SQLite, Oracle, SQL Server — not supported |
| `NATURAL JOIN` | PG, SQLite, MySQL, Oracle | SQL Server — not supported; workaround requires schema knowledge we don't have |
| `USING (col, ...)` in JOIN | PG, SQLite, MySQL, Oracle | SQL Server — not supported; workaround requires tracking table aliases, too fragile |

---

## IGNORE — Optimizer hints, safe to skip without changing results

| Feature | Supported by | Notes |
|---|---|---|
| CTE `MATERIALIZED` / `NOT MATERIALIZED` | PG | Optimizer hint, query produces same result without it |
| `INDEXED BY` / `NOT INDEXED` | SQLite | Index hint, no effect on results |
| `USE INDEX` / `IGNORE INDEX` / `FORCE INDEX` | MySQL | Index hint |
| `WITH (table_hint)` — performance hints | SQL Server | `NOLOCK`, `ROWLOCK`, `PAGELOCK`, `TABLOCK`, `FORCESEEK`, `FORCESCAN`, `INDEX(...)` etc. |
| `HIGH_PRIORITY` | MySQL | Scheduling hint |
| `STRAIGHT_JOIN` | MySQL | Join order hint |
| `SQL_SMALL_RESULT` / `SQL_BIG_RESULT` / `SQL_BUFFER_RESULT` | MySQL | Temp table hints |
| `SQL_NO_CACHE` / `SQL_CALC_FOUND_ROWS` | MySQL | Cache/counting hints |
| `OPTION (query_hint, ...)` | SQL Server | `RECOMPILE`, `MAXDOP`, `HASH JOIN` etc. |

---

## WORKAROUND — Renderer transforms to equivalent syntax automatically

| Feature | Transformation | Details |
|---|---|---|
| `LIMIT n` | PG/SQLite/MySQL: `LIMIT n`; Oracle: `FETCH FIRST n ROWS ONLY`; SS: `TOP(n)` or `FETCH FIRST` | AST has one `LimitDef`, renderer picks syntax |
| `FETCH FIRST n ROWS ONLY` | Oracle/SS: native; PG/SQLite/MySQL: `LIMIT n` | Reverse of above |
| `TOP(n)` | SS: native; others: `LIMIT n` or `FETCH FIRST` | Reverse of above |
| `FETCH FIRST ... WITH TIES` | PG/Oracle/SS: native; MySQL/SQLite: ERROR (no equivalent) | WITH TIES changes result set — can't silently drop |
| `EXCEPT` ↔ `MINUS` | Oracle: render as `MINUS`; others: render as `EXCEPT` | Same semantics, different keyword |
| `LATERAL (subquery)` ↔ `CROSS/OUTER APPLY` | PG/MySQL/Oracle: `LATERAL`; SS: `CROSS APPLY`/`OUTER APPLY`; SQLite: ERROR | Same semantics, different syntax |
| `WINDOW w AS (...)` → Oracle | Inline window definition into each `OVER(...)` reference | Oracle lacks named windows but supports inline window specs |
| `NULLS FIRST` / `NULLS LAST` | PG/Oracle/SQLite: native; MySQL/SS: `ORDER BY (CASE WHEN col IS NULL THEN 0 ELSE 1 END), col` | Django-style workaround; widely expected to work |
| `FOR UPDATE` → SS | `WITH (UPDLOCK, ROWLOCK)` as table hint | Different syntax, same lock semantics |
| `FOR SHARE` → SS | `WITH (HOLDLOCK)` as table hint | Different syntax, same semantics |
| `FOR UPDATE OF table` → SS | `FROM t1 WITH (UPDLOCK) JOIN t2` — apply hint to specific table | Per-table targeting via hints |

---

## Decision Matrix by Database

### PostgreSQL renderer

| Feature | Strategy |
|---|---|
| Everything in core SELECT | Native support |
| MySQL/Oracle/SS-specific hints | IGNORE |

### SQLite renderer

| Feature | Strategy |
|---|---|
| `DISTINCT ON` | ERROR |
| `TABLESAMPLE` | ERROR |
| `ROLLUP` / `CUBE` / `GROUPING SETS` | ERROR |
| `PIVOT` / `UNPIVOT` | ERROR |
| `FOR UPDATE` / `FOR SHARE` / any row locking | ERROR (no row-level locking) |
| `SKIP LOCKED` / `NOWAIT` | ERROR |
| `LATERAL` | ERROR |
| `FETCH FIRST ... WITH TIES` | ERROR |
| `FETCH FIRST ... PERCENT` | ERROR |
| `INTERSECT ALL` / `EXCEPT ALL` | ERROR |
| `NULLS FIRST` / `NULLS LAST` | Native (3.30+) |
| `LIMIT` / `OFFSET` | Native |
| `WINDOW` clause | Native (3.28+) |
| `CTE MATERIALIZED` | IGNORE |
| MySQL/Oracle/SS hints | IGNORE |

### MySQL renderer (future)

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

### Oracle renderer (future)

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

### SQL Server renderer (future)

| Feature | Strategy |
|---|---|
| `DISTINCT ON` | ERROR |
| `LIMIT n` | WORKAROUND → `TOP(n)` or `FETCH FIRST` |
| `NULLS FIRST` / `NULLS LAST` | WORKAROUND → CASE expression |
| `NATURAL JOIN` | ERROR |
| `USING` in JOIN | ERROR |
| `LATERAL` | WORKAROUND → `CROSS/OUTER APPLY` |
| `FOR UPDATE` / `FOR SHARE` | WORKAROUND → table hints |
| `SKIP LOCKED` | WORKAROUND → `READPAST` hint (note: slightly different semantics) |
| `EXCEPT` / `INTERSECT` | Native |
| `INTERSECT ALL` / `EXCEPT ALL` | ERROR |
| `WINDOW` clause | Native (2022) |
| `CTE MATERIALIZED` | IGNORE |
| `FETCH FIRST ... PERCENT` | ERROR |
