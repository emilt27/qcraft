# DQL Syntax Reference (All Dialects)

Full syntax for latest versions: PostgreSQL 17, SQLite 3.45+, MySQL 8.4, Oracle 23c, SQL Server 2022.

---

## 1. SELECT (Core Structure)

### 1.1 PostgreSQL 17

```sql
[ WITH [ RECURSIVE ] with_query [, ...] ]
SELECT [ ALL | DISTINCT [ ON ( expression [, ...] ) ] ]
    [ * | expression [ [ AS ] output_name ] [, ...] ]
    [ FROM from_item [, ...] ]
    [ WHERE condition ]
    [ GROUP BY [ ALL | DISTINCT ] grouping_element [, ...] ]
    [ HAVING condition ]
    [ WINDOW window_name AS ( window_definition ) [, ...] ]
    [ { UNION | INTERSECT | EXCEPT } [ ALL | DISTINCT ] select ]
    [ ORDER BY expression [ ASC | DESC | USING operator ] [ NULLS { FIRST | LAST } ] [, ...] ]
    [ LIMIT { count | ALL } ]
    [ OFFSET start [ ROW | ROWS ] ]
    [ FETCH { FIRST | NEXT } [ count ] { ROW | ROWS } { ONLY | WITH TIES } ]
    [ FOR { UPDATE | NO KEY UPDATE | SHARE | KEY SHARE } [ OF table_name [, ...] ] [ NOWAIT | SKIP LOCKED ] [, ...] ]
```

**Key PG-specific features:**
- `DISTINCT ON (expr, ...)` — select first row per distinct group (unique to PG).
- `NULLS FIRST | LAST` in ORDER BY.
- `FETCH FIRST ... WITH TIES` — includes tied rows.
- `FOR NO KEY UPDATE` / `FOR KEY SHARE` — fine-grained row locking (beyond UPDATE/SHARE).
- `SKIP LOCKED` — skip locked rows instead of waiting.
- `WINDOW` clause — named window definitions reusable across multiple window functions.
- `GROUPING SETS`, `ROLLUP`, `CUBE` in GROUP BY.
- `GROUP BY ALL | DISTINCT` — ALL keeps duplicates in grouping sets, DISTINCT removes them.

### 1.2 SQLite 3.45+

```sql
[ WITH [ RECURSIVE ] common-table-expression [, ...] ]
SELECT [ ALL | DISTINCT ] result-column [, ...]
    [ FROM { table-or-subquery | join-clause } ]
    [ WHERE expr ]
    [ GROUP BY expr [, ...] [ HAVING expr ] ]
    [ WINDOW window-name AS window-defn [, ...] ]
    [ VALUES (expr [, ...]) [, (expr [, ...]) ...] ]
    [ compound-operator select ]
    [ ORDER BY ordering-term [, ...] ]
    [ LIMIT expr [ ( OFFSET | , ) expr ] ]

-- compound-operator: UNION | UNION ALL | INTERSECT | EXCEPT
```

**Key SQLite-specific features:**
- No `DISTINCT ON`.
- No `FETCH FIRST ... ROWS`.
- No `FOR UPDATE` / row locking (SQLite uses file-level locks).
- No `NULLS FIRST | LAST` (until 3.30.0, now supported).
- `LIMIT expr, expr` alternative syntax (second is offset, reversed from MySQL!).
- No `FULL OUTER JOIN` (until 3.39.0, now supported).
- No `RIGHT JOIN` (until 3.39.0, now supported).
- No `LATERAL` joins.
- No `GROUPING SETS` / `ROLLUP` / `CUBE`.
- `WINDOW` clause supported since 3.28.0.
- No `TABLESAMPLE`.

### 1.3 MySQL 8.4

```sql
[ WITH [ RECURSIVE ] cte_name [ ( col_name [, ...] ) ] AS ( subquery ) [, ...] ]
SELECT
    [ ALL | DISTINCT | DISTINCTROW ]
    [ HIGH_PRIORITY ]
    [ STRAIGHT_JOIN ]
    [ SQL_SMALL_RESULT | SQL_BIG_RESULT ] [ SQL_BUFFER_RESULT ]
    [ SQL_NO_CACHE ] [ SQL_CALC_FOUND_ROWS ]
    select_expr [, ...]
    [ FROM table_references [ PARTITION ( partition_list ) ] ]
    [ WHERE where_condition ]
    [ GROUP BY { col_name | expr | position } [ ASC | DESC ] [, ...] [ WITH ROLLUP ] ]
    [ HAVING where_condition ]
    [ WINDOW window_name AS ( window_spec ) [, ...] ]
    [ ORDER BY { col_name | expr | position } [ ASC | DESC ] [, ...] [ WITH ROLLUP ] ]
    [ LIMIT { [offset,] row_count | row_count OFFSET offset } ]
    [ INTO OUTFILE 'file_name' | INTO DUMPFILE 'file_name' | INTO @var_name [, ...] ]
    [ { UNION | INTERSECT | EXCEPT } [ ALL | DISTINCT ] select ]
    [ FOR { UPDATE | SHARE } [ OF tbl_name [, ...] ] [ NOWAIT | SKIP LOCKED ] ]

-- table_references:
table_factor [ join_type table_factor join_condition ] [, ...]
-- includes: INNER JOIN, CROSS JOIN, LEFT [OUTER] JOIN, RIGHT [OUTER] JOIN, NATURAL JOIN, STRAIGHT_JOIN
```

**Key MySQL-specific features:**
- `DISTINCTROW` — alias for `DISTINCT`.
- `HIGH_PRIORITY` — gives SELECT higher priority than UPDATE.
- `STRAIGHT_JOIN` — forces left-to-right join order.
- `SQL_CALC_FOUND_ROWS` — deprecated, calculates total rows for pagination.
- `SQL_NO_CACHE` — skips query cache.
- `SQL_SMALL_RESULT` / `SQL_BIG_RESULT` — optimizer hints for temp tables.
- `SQL_BUFFER_RESULT` — forces result into temp table.
- `GROUP BY ... WITH ROLLUP` (no `CUBE` or `GROUPING SETS`).
- `GROUP BY ... ASC | DESC` — ordering within group by (nonstandard).
- `LIMIT offset, count` — note: offset comes FIRST (opposite of SQL standard).
- `INTO OUTFILE` / `INTO DUMPFILE` / `INTO @var` — export results.
- `PARTITION (p1, p2)` — query specific partitions.
- No `FETCH FIRST ... ROWS` (until 8.0.37+, now supported).
- No `FULL OUTER JOIN`.
- No `LATERAL` (supported since 8.0.14).
- No `DISTINCT ON`.
- `FOR SHARE` replaces old `LOCK IN SHARE MODE`.

### 1.4 Oracle 23c

```sql
[ WITH
    [ RECURSIVE ] query_name [ ( col_alias [, ...] ) ] AS ( subquery ) [ SEARCH clause ] [ CYCLE clause ]
    [, ...]
]
SELECT [ hint ] [ ALL | DISTINCT | UNIQUE ]
    select_list
    FROM { table_reference | join_clause | ( subquery ) | LATERAL ( subquery ) | table_collection_expression } [, ...]
    [ WHERE condition ]
    [ GROUP BY { expr | rollup_cube_clause | grouping_sets_clause } [, ...] [ HAVING condition ] ]
    [ MODEL clause ]
    [ { UNION [ALL] | INTERSECT | MINUS } select ]
    [ ORDER BY { expr | position | c_alias } [ ASC | DESC ] [ NULLS { FIRST | LAST } ] [, ...] ]
    [ OFFSET offset { ROW | ROWS } ]
    [ FETCH { FIRST | NEXT } [ count | percent PERCENT ] { ROW | ROWS } { ONLY | WITH TIES } ]
    [ FOR UPDATE [ OF column [, ...] ] [ NOWAIT | WAIT integer | SKIP LOCKED ] ]

-- rollup_cube_clause:
{ ROLLUP | CUBE } ( grouping_expression_list )

-- grouping_sets_clause:
GROUPING SETS ( { rollup_cube_clause | grouping_expression_list } [, ...] )
```

**Key Oracle-specific features:**
- `UNIQUE` — alias for `DISTINCT`.
- `MINUS` — Oracle's name for `EXCEPT`.
- `/* hint */` — optimizer hints (`/*+ FULL(t) */`, `/*+ INDEX(t idx) */`, etc.).
- `NULLS FIRST | LAST` in ORDER BY.
- `FETCH FIRST ... PERCENT ROWS` — limit by percentage.
- `FETCH FIRST ... WITH TIES` — includes tied rows.
- `FOR UPDATE ... WAIT integer` — wait N seconds for lock.
- `ROLLUP`, `CUBE`, `GROUPING SETS` — all supported.
- `MODEL` clause — spreadsheet-like calculations (unique to Oracle).
- `CONNECT BY` / `START WITH` — hierarchical queries (legacy, replaced by recursive CTE).
- `LATERAL` inline views.
- `PIVOT` / `UNPIVOT` operators.
- `MATCH_RECOGNIZE` — pattern matching on rows.
- `SAMPLE` clause — random sampling.
- No `LIMIT` keyword (uses `FETCH FIRST` or legacy `ROWNUM`).
- `SEARCH DEPTH FIRST | BREADTH FIRST` for recursive CTEs.
- `CYCLE` clause for recursive CTE cycle detection.

### 1.5 SQL Server 2022

```sql
[ WITH cte_name [ ( column_name [, ...] ) ] AS ( cte_query_definition ) [, ...] ]
SELECT [ ALL | DISTINCT ]
    [ TOP ( expression ) [ PERCENT ] [ WITH TIES ] ]
    select_list
    [ INTO new_table ]
    [ FROM { table_source } [, ...] ]
    [ WHERE search_condition ]
    [ GROUP BY { ALL group_by_expression [, ...] | ROLLUP ( ... ) | CUBE ( ... ) | GROUPING SETS ( ... ) | () } ]
    [ HAVING search_condition ]
    [ WINDOW window_name AS ( window_spec ) [, ...] ]
    [ ORDER BY order_by_expression [ ASC | DESC ] [, ...] ]
    [ OFFSET { integer_constant | offset_row_count_expression } { ROW | ROWS } ]
    [ FETCH { FIRST | NEXT } { integer_constant | fetch_row_count_expression } { ROW | ROWS } { ONLY | WITH TIES } ]
    [ OPTION ( query_hint [, ...] ) ]

-- FOR clause (XML/JSON output):
[ FOR { XML | JSON } ... ]

-- table_source includes table hints:
table_name [ WITH ( table_hint [, ...] ) ] [ AS alias ]
```

**Key SQL Server-specific features:**
- `TOP (N) [PERCENT] [WITH TIES]` — limit rows (alternative to FETCH FIRST).
- `SELECT ... INTO new_table` — create table from query result.
- `OPTION (query_hint, ...)` — query hints (`RECOMPILE`, `HASH JOIN`, `MAXDOP N`, etc.).
- `FOR XML` / `FOR JSON` — convert result to XML/JSON.
- `WITH (table_hint)` — table-level hints (`NOLOCK`, `HOLDLOCK`, `TABLOCK`, etc.).
- `ROLLUP`, `CUBE`, `GROUPING SETS` — all supported.
- `GROUP BY ALL` — deprecated, includes groups with count 0.
- `OFFSET ... FETCH` — SQL standard pagination (requires ORDER BY).
- No `LIMIT` keyword.
- No `DISTINCT ON`.
- `CROSS APPLY` / `OUTER APPLY` — SQL Server's equivalent of `LATERAL JOIN`.
- `PIVOT` / `UNPIVOT` operators.
- `WINDOW` clause supported since SQL Server 2022.
- `NULLS FIRST | LAST` — NOT supported (nulls always sort as smallest values).

---

## 2. FROM Clause & Joins

### 2.1 PostgreSQL 17

```sql
FROM from_item [, ...]

-- from_item:
[ ONLY ] table_name [ * ] [ [ AS ] alias [ ( column_alias [, ...] ) ] ]
    [ TABLESAMPLE sampling_method ( argument [, ...] ) [ REPEATABLE ( seed ) ] ]
| LATERAL ( subquery ) [ AS ] alias [ ( column_alias [, ...] ) ]
| with_query_name [ [ AS ] alias [ ( column_alias [, ...] ) ] ]
| LATERAL function_name ( ... ) [ WITH ORDINALITY ] [ [ AS ] alias [ ( column_alias [, ...] ) ] ]
| ROWS FROM ( function_name ( ... ) [ AS ( column_definition [, ...] ) ] [, ...] ) [ WITH ORDINALITY ]
| from_item [ NATURAL ] join_type from_item [ ON join_condition | USING ( join_column [, ...] ) ]

-- join_type:
[ INNER ] JOIN | LEFT [ OUTER ] JOIN | RIGHT [ OUTER ] JOIN | FULL [ OUTER ] JOIN | CROSS JOIN
```

**PG FROM features:**
- `ONLY table` — exclude inherited/child tables.
- `table *` — include all descendant tables (default).
- `LATERAL` subqueries and functions — can reference earlier FROM items.
- `TABLESAMPLE method (pct) REPEATABLE (seed)` — `BERNOULLI` or `SYSTEM`.
- `WITH ORDINALITY` — adds row number column to function results.
- `ROWS FROM(...)` — combine multiple set-returning functions.
- `USING` in joins (also `USING (col1, col2)`).
- All join types: INNER, LEFT, RIGHT, FULL, CROSS, NATURAL.

### 2.2 SQLite 3.45+

```sql
FROM { table-or-subquery | join-clause }

-- table-or-subquery:
[ schema-name . ] table-name [ [ AS ] alias ] [ INDEXED BY index-name | NOT INDEXED ]
| table-function-name ( expr [, ...] ) [ [ AS ] alias ]
| ( subquery ) [ [ AS ] alias ]
| ( join-clause )

-- join-clause:
table-or-subquery [ join-operator table-or-subquery join-constraint ] [, ...]

-- join-operator:
, | [ NATURAL ] { [ LEFT | RIGHT | FULL ] [ OUTER ] | [ INNER ] | CROSS } JOIN

-- join-constraint:
ON expr | USING ( column-name [, ...] )
```

**SQLite FROM features:**
- `INDEXED BY index-name` / `NOT INDEXED` — force or prevent index usage.
- All join types since 3.39.0 (LEFT, RIGHT, FULL, CROSS, NATURAL).
- No `LATERAL`.
- No `TABLESAMPLE`.
- Table-valued functions supported (`json_each()`, `generate_series()`, etc.).

### 2.3 MySQL 8.4

```sql
FROM table_references

-- table_references:
table_reference [, ...]

-- table_reference:
table_factor | joined_table

-- table_factor:
tbl_name [ PARTITION ( partition_list ) ] [ [ AS ] alias ] [ index_hint_list ]
| ( subquery ) [ AS ] alias [ ( col_list ) ]
| LATERAL ( subquery ) [ AS ] alias [ ( col_list ) ]
| { OJ table_reference LEFT OUTER JOIN table_reference ON join_condition }
| JSON_TABLE ( ... )

-- joined_table:
table_reference { [ INNER | CROSS ] JOIN | STRAIGHT_JOIN } table_reference [ ON | USING ]
| table_reference { LEFT | RIGHT } [ OUTER ] JOIN table_reference { ON | USING }
| table_reference NATURAL [ { LEFT | RIGHT } [ OUTER ] ] JOIN table_reference

-- index_hint:
{ USE | IGNORE | FORCE } { INDEX | KEY } [ FOR { JOIN | ORDER BY | GROUP BY } ] ( index_list )
```

**MySQL FROM features:**
- `PARTITION (p1, p2)` — query specific partitions.
- `USE INDEX / IGNORE INDEX / FORCE INDEX` hints.
- `STRAIGHT_JOIN` — forces join order.
- `{ OJ ... }` — ODBC-style outer join syntax (legacy).
- `LATERAL` subqueries (since 8.0.14).
- `JSON_TABLE()` — convert JSON to relational.
- No `FULL OUTER JOIN`.
- No `TABLESAMPLE`.

### 2.4 Oracle 23c

```sql
FROM { table_reference | join_clause | (subquery) } [, ...]

-- table_reference:
[ schema. ] { table | view } [ @ dblink ] [ PARTITION ( partition ) | SUBPARTITION ( subpartition ) ]
    [ SAMPLE [ BLOCK ] ( sample_percent ) [ SEED ( seed_value ) ] ]
    [ [ AS ] alias ] [ pivot_clause | unpivot_clause | lateral_clause ]
| ( subquery ) [ [ AS ] alias ]
| LATERAL ( subquery ) [ [ AS ] alias ]
| TABLE ( collection_expression ) [ (+) ]
| XMLTABLE ( ... )
| JSON_TABLE ( ... )

-- join_clause:
table_reference { [ INNER ] | { LEFT | RIGHT | FULL | CROSS } [ OUTER ] | NATURAL [...] } JOIN table_reference
    { ON condition | USING ( column [, ...] ) }

-- pivot_clause:
PIVOT [ XML ] ( aggregate_function FOR column IN ( ... ) )

-- unpivot_clause:
UNPIVOT [ { INCLUDE | EXCLUDE } NULLS ] ( value_column FOR pivot_column IN ( ... ) )
```

**Oracle FROM features:**
- `@ dblink` — query remote database via database link.
- `SAMPLE [BLOCK] (pct) SEED (n)` — random sampling.
- `PARTITION (p1)` / `SUBPARTITION (p1)` — partition targeting.
- `PIVOT` / `UNPIVOT` — column rotation.
- `LATERAL` inline views.
- `TABLE(collection)` — unnest collection types.
- `JSON_TABLE()`, `XMLTABLE()`.
- `(+)` — Oracle's old outer join syntax (legacy, use ANSI joins).
- All join types: INNER, LEFT, RIGHT, FULL, CROSS, NATURAL.

### 2.5 SQL Server 2022

```sql
FROM { table_source } [, ...]

-- table_source:
table_or_view_name [ WITH ( table_hint [, ...] ) ] [ [ AS ] alias ]
| ( subquery ) [ AS ] alias [ ( column_alias [, ...] ) ]
| derived_table [ AS ] alias [ ( column_alias [, ...] ) ]
| table_valued_function [ [ AS ] alias [ ( column_alias [, ...] ) ] ]
| OPENROWSET ( ... )
| OPENDATASOURCE ( ... )
| table_source { CROSS | OUTER } APPLY table_source
| table_source PIVOT ( agg FOR col IN ( value_list ) ) AS alias
| table_source UNPIVOT ( value_col FOR pivot_col IN ( col_list ) ) AS alias

-- join_type:
[ INNER ] JOIN | { LEFT | RIGHT | FULL } [ OUTER ] JOIN | CROSS JOIN

-- table_hint:
NOLOCK | HOLDLOCK | UPDLOCK | TABLOCK | TABLOCKX | ROWLOCK | PAGELOCK | XLOCK
| READUNCOMMITTED | READCOMMITTED | REPEATABLEREAD | SERIALIZABLE | SNAPSHOT
| READPAST | NOWAIT | INDEX ( index_val [, ...] ) | FORCESEEK | FORCESCAN
| SPATIAL_WINDOW_MAX_CELLS
```

**SQL Server FROM features:**
- `WITH (table_hint)` — granular locking and access hints.
- `CROSS APPLY` / `OUTER APPLY` — equivalent to `LATERAL JOIN` / `LEFT LATERAL JOIN`.
- `PIVOT` / `UNPIVOT` — column rotation.
- `OPENROWSET` / `OPENDATASOURCE` — ad-hoc remote queries.
- `TABLESAMPLE SYSTEM (pct PERCENT) REPEATABLE (seed)` — random sampling.
- All standard join types except `NATURAL JOIN`.
- No `USING` clause in joins (only `ON`).

---

## 3. WHERE Clause

Standard across all databases. Differences in operators and functions:

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| `IN` / `NOT IN` | Yes | Yes | Yes | Yes | Yes |
| `BETWEEN` | Yes | Yes | Yes | Yes | Yes |
| `LIKE` / `NOT LIKE` | Yes | Yes | Yes | Yes | Yes |
| `ILIKE` (case-insensitive) | Yes | No (use COLLATE) | No | No | No |
| `SIMILAR TO` | Yes | No | No | No | No |
| `~` regex match | Yes | No | No | No | No |
| `REGEXP` | No | No (ext) | Yes | Yes (REGEXP_LIKE) | No |
| `EXISTS` / `NOT EXISTS` | Yes | Yes | Yes | Yes | Yes |
| `ANY` / `ALL` / `SOME` | Yes | No | Yes | Yes | Yes |
| `IS [NOT] DISTINCT FROM` | Yes | `IS [NOT]` | Yes (8.0.16+) | No | `IS [NOT] DISTINCT FROM` (2022) |
| `OVERLAPS` | Yes | No | No | No | No |
| `@>`, `<@` (containment) | Yes (arrays, JSON) | No | No | No | No |
| `&&` (overlap) | Yes (arrays, ranges) | No | No | No | No |

---

## 4. GROUP BY

### Comparison

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| Basic GROUP BY | Yes | Yes | Yes | Yes | Yes |
| GROUP BY position | Yes | No | Yes | Yes | Yes |
| `ROLLUP` | `ROLLUP(a,b)` | No | `WITH ROLLUP` | `ROLLUP(a,b)` | `ROLLUP(a,b)` |
| `CUBE` | `CUBE(a,b)` | No | No | `CUBE(a,b)` | `CUBE(a,b)` |
| `GROUPING SETS` | Yes | No | No | Yes | Yes |
| `GROUPING()` function | Yes | No | Yes (8.0.1+) | Yes | Yes |
| `GROUP BY ALL` | PG 16+ (auto-infer) | No | Deprecated | No | Deprecated |
| `GROUP BY DISTINCT` | PG 16+ | No | No | No | No |
| Empty grouping set `()` | Yes | No | No | Yes | Yes |

### MySQL-specific syntax

```sql
GROUP BY col1 [ASC|DESC], col2 [ASC|DESC] WITH ROLLUP
```

- MySQL allows `ASC`/`DESC` in GROUP BY (nonstandard).
- Uses `WITH ROLLUP` syntax (not `ROLLUP()`).
- No `CUBE`, no `GROUPING SETS`.

---

## 5. HAVING

Standard across all databases. Applied after GROUP BY.

---

## 6. WINDOW Clause (Named Windows)

### Comparison

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| Named windows (`WINDOW w AS (...)`) | Yes | Yes (3.28+) | Yes (8.0+) | No | Yes (2022) |
| Window inheritance | Yes | Yes | Yes | No | Yes |

```sql
-- PG/SQLite/MySQL/SQL Server:
WINDOW w AS (PARTITION BY x ORDER BY y)
SELECT ..., SUM(a) OVER w ...

-- Window inheritance:
WINDOW w1 AS (PARTITION BY x), w2 AS (w1 ORDER BY y)
```

Oracle does not support the `WINDOW` clause — window specs must be inline in `OVER()`.

---

## 7. ORDER BY

### Comparison

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| `ASC` / `DESC` | Yes | Yes | Yes | Yes | Yes |
| By position (`ORDER BY 1, 2`) | Yes | Yes | Yes | Yes | Yes |
| By alias | Yes | Yes | Yes | Yes | Yes |
| `NULLS FIRST` / `NULLS LAST` | Yes | Yes (3.30+) | No | Yes | No |
| `USING operator` | Yes | No | No | No | No |
| `COLLATE` | Yes | Yes | Yes | Yes | Yes |

**Default nulls ordering:**
- PostgreSQL: NULLS LAST for ASC, NULLS FIRST for DESC.
- Oracle: NULLS LAST for ASC, NULLS FIRST for DESC (same as PG).
- MySQL: NULLS FIRST for ASC (nulls are smallest).
- SQL Server: NULLS FIRST for ASC (nulls are smallest).
- SQLite: NULLS FIRST for ASC (nulls are smallest).

---

## 8. LIMIT / OFFSET / Pagination

### Comparison

| Syntax | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| `LIMIT n` | Yes | Yes | Yes | No | No |
| `LIMIT n OFFSET m` | Yes | Yes | Yes | No | No |
| `LIMIT m, n` (offset, count) | No | Yes | Yes | No | No |
| `OFFSET m ROWS` | Yes | No | Yes (8.0.37+) | Yes (12c+) | Yes |
| `FETCH FIRST n ROWS ONLY` | Yes | No | Yes (8.0.37+) | Yes (12c+) | Yes |
| `FETCH ... WITH TIES` | Yes | No | No | Yes | Yes |
| `FETCH ... PERCENT` | No | No | No | Yes | No |
| `TOP n` | No | No | No | No | Yes |
| `TOP n PERCENT` | No | No | No | No | Yes |
| `TOP n WITH TIES` | No | No | No | No | Yes |
| `ROWNUM` (legacy) | No | No | No | Yes | No |

### Notes

- PostgreSQL supports both `LIMIT/OFFSET` and `FETCH FIRST`.
- `FETCH FIRST ... WITH TIES` requires `ORDER BY`.
- SQL Server `TOP` can be used without `ORDER BY` (arbitrary rows).
- Oracle `FETCH FIRST n PERCENT ROWS` is unique.
- MySQL `LIMIT offset, count` — offset is first (confusing vs `LIMIT count OFFSET offset`).

---

## 9. Set Operations (UNION / INTERSECT / EXCEPT)

### Comparison

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| `UNION` | Yes | Yes | Yes | Yes | Yes |
| `UNION ALL` | Yes | Yes | Yes | Yes | Yes |
| `INTERSECT` | Yes | Yes | Yes (8.0.31+) | Yes | Yes |
| `INTERSECT ALL` | Yes | No | Yes (8.0.31+) | No | No |
| `EXCEPT` | Yes | Yes | Yes (8.0.31+) | No (`MINUS`) | Yes |
| `EXCEPT ALL` | Yes | No | Yes (8.0.31+) | No | No |
| `MINUS` | No | No | No | Yes | No |
| Operator precedence | `INTERSECT` > `UNION`/`EXCEPT` | Left to right | `INTERSECT` > `UNION`/`EXCEPT` | Left to right | Left to right |
| Parenthesized queries | Yes | Yes | Yes (8.0.31+) | Yes | Yes |

---

## 10. Common Table Expressions (WITH / CTE)

### Comparison

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| Basic CTE | Yes | Yes | Yes (8.0+) | Yes | Yes |
| Recursive CTE | Yes | Yes | Yes (8.0+) | Yes | Yes |
| `MATERIALIZED` / `NOT MATERIALIZED` | Yes (PG 12+) | No | No | No | No |
| Column list in CTE def | Yes | Yes | Yes | Yes | Yes |
| `SEARCH DEPTH FIRST` | No | No | No | Yes | No |
| `SEARCH BREADTH FIRST` | No | No | No | Yes | No |
| `CYCLE` detection clause | No | No | No | Yes | No |
| CTE in DML (INSERT/UPDATE/DELETE) | Yes | Yes | No | Yes (subquery factoring) | Yes |

### PostgreSQL CTE specifics

```sql
WITH [ RECURSIVE ] cte_name [ ( column_name [, ...] ) ] AS [ [ NOT ] MATERIALIZED ] (
    query
) [, ...]
```

- `MATERIALIZED` — force CTE to be evaluated once and stored.
- `NOT MATERIALIZED` — inline CTE into each reference (optimizer may do this by default since PG 12).

### Oracle CTE specifics

```sql
WITH [ RECURSIVE ] cte_name [ ( column_alias [, ...] ) ] AS (
    anchor_query
    UNION ALL
    recursive_query
)
SEARCH { DEPTH | BREADTH } FIRST BY col SET ordering_column
CYCLE col SET cycle_mark TO 'Y' DEFAULT 'N'
```

- `SEARCH DEPTH FIRST` / `BREADTH FIRST` — control traversal order.
- `CYCLE` clause — automatic cycle detection and marking.

---

## 11. Subqueries

### Types (all databases)

```sql
-- Scalar subquery (returns single value):
SELECT (SELECT MAX(id) FROM t) AS max_id

-- Table subquery (in FROM):
SELECT * FROM (SELECT ...) AS sub

-- Correlated subquery (references outer):
SELECT * FROM t1 WHERE t1.x = (SELECT MAX(t2.x) FROM t2 WHERE t2.id = t1.id)

-- EXISTS:
SELECT * FROM t1 WHERE EXISTS (SELECT 1 FROM t2 WHERE t2.fk = t1.id)

-- IN with subquery:
SELECT * FROM t1 WHERE id IN (SELECT fk FROM t2)

-- ANY / ALL / SOME:
SELECT * FROM t1 WHERE x > ANY (SELECT y FROM t2)
SELECT * FROM t1 WHERE x > ALL (SELECT y FROM t2)
```

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| Scalar subquery | Yes | Yes | Yes | Yes | Yes |
| Table subquery | Yes | Yes | Yes | Yes | Yes |
| Correlated | Yes | Yes | Yes | Yes | Yes |
| EXISTS | Yes | Yes | Yes | Yes | Yes |
| IN (subquery) | Yes | Yes | Yes | Yes | Yes |
| ANY / ALL / SOME | Yes | No | Yes | Yes | Yes |
| `LATERAL` subquery | Yes | No | Yes (8.0.14+) | Yes | `CROSS/OUTER APPLY` |
| `ARRAY(subquery)` | Yes | No | No | No | No |

---

## 12. Row Locking (FOR UPDATE / FOR SHARE)

### Comparison

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| `FOR UPDATE` | Yes | No | Yes | Yes | Table hints |
| `FOR SHARE` | Yes | No | Yes | No | Table hints |
| `FOR NO KEY UPDATE` | Yes | No | No | No | No |
| `FOR KEY SHARE` | Yes | No | No | No | No |
| `OF table [, ...]` | Yes | No | Yes | Yes | N/A |
| `NOWAIT` | Yes | No | Yes | Yes | Table hint |
| `SKIP LOCKED` | Yes | No | Yes (8.0+) | Yes (11g+) | Table hint (`READPAST`) |
| `WAIT n` (seconds) | No | No | No | Yes | No |

### PostgreSQL

```sql
FOR { UPDATE | NO KEY UPDATE | SHARE | KEY SHARE }
    [ OF table_name [, ...] ]
    [ NOWAIT | SKIP LOCKED ]
```

- Multiple lock clauses can target different tables.
- `FOR KEY SHARE` — weakest, allows non-key updates.
- `FOR NO KEY UPDATE` — allows key share by other transactions.

### SQL Server (via table hints)

```sql
SELECT * FROM t WITH (UPDLOCK, ROWLOCK, NOWAIT)
SELECT * FROM t WITH (HOLDLOCK)     -- equivalent to SERIALIZABLE read
SELECT * FROM t WITH (READPAST)     -- equivalent to SKIP LOCKED
```

---

## 13. TABLESAMPLE

### Comparison

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| `TABLESAMPLE` | Yes | No | No | `SAMPLE` | Yes |
| Methods | BERNOULLI, SYSTEM | — | — | BLOCK, ROW | SYSTEM |
| `REPEATABLE (seed)` | Yes | — | — | `SEED (n)` | Yes |

### PostgreSQL

```sql
FROM table_name TABLESAMPLE { BERNOULLI | SYSTEM } ( percentage ) [ REPEATABLE ( seed ) ]
```

### Oracle

```sql
FROM table_name SAMPLE [ BLOCK ] ( percentage ) [ SEED ( seed_value ) ]
```

### SQL Server

```sql
FROM table_name TABLESAMPLE SYSTEM ( percentage PERCENT ) [ REPEATABLE ( seed ) ]
```

---

## 14. PIVOT / UNPIVOT

Supported by: **Oracle**, **SQL Server**. Not natively by PG, SQLite, MySQL.

### Oracle

```sql
SELECT * FROM sales
PIVOT (
    SUM(amount) FOR product IN ('A' AS a, 'B' AS b, 'C' AS c)
)

SELECT * FROM pivoted
UNPIVOT [ { INCLUDE | EXCLUDE } NULLS ] (
    amount FOR product IN (a, b, c)
)
```

### SQL Server

```sql
SELECT * FROM sales
PIVOT (
    SUM(amount) FOR product IN ([A], [B], [C])
) AS pvt

SELECT * FROM pvt
UNPIVOT (
    amount FOR product IN ([A], [B], [C])
) AS unpvt
```

---

## 15. LATERAL

### Comparison

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| `LATERAL` subquery | Yes | No | Yes (8.0.14+) | Yes | No |
| `CROSS APPLY` | No | No | No | Yes (12c+) | Yes |
| `OUTER APPLY` | No | No | No | Yes (12c+) | Yes |

```sql
-- PG / MySQL / Oracle:
SELECT * FROM t1, LATERAL (SELECT * FROM t2 WHERE t2.fk = t1.id) sub

-- SQL Server / Oracle:
SELECT * FROM t1 CROSS APPLY (SELECT * FROM t2 WHERE t2.fk = t1.id) sub
SELECT * FROM t1 OUTER APPLY (SELECT * FROM t2 WHERE t2.fk = t1.id) sub
```

---

## 16. VALUES as Standalone Query

### Comparison

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| `VALUES (1,2), (3,4)` | Yes | Yes | Yes | No | Yes (in FROM) |

```sql
-- PG / SQLite / MySQL:
VALUES (1, 'a'), (2, 'b'), (3, 'c')

-- Can be used in CTEs, set operations, etc.
```

---

## 17. Database-Specific Features

### 17.1 PostgreSQL-only

- `DISTINCT ON (expr, ...)` — first row per group.
- `ARRAY(subquery)` — subquery result as array.
- `ROWS FROM(func1(...), func2(...))` — combine SRFs side-by-side.
- `WITH ORDINALITY` — adds row number to SRF output.
- `FOR NO KEY UPDATE` / `FOR KEY SHARE` — fine-grained locking.
- `GROUP BY ALL` / `GROUP BY DISTINCT` (PG 16+).
- CTE `MATERIALIZED` / `NOT MATERIALIZED`.
- `USING operator` in ORDER BY.

### 17.2 Oracle-only

- `MINUS` (instead of EXCEPT).
- `MODEL` clause.
- `CONNECT BY` / `START WITH` (hierarchical queries).
- `MATCH_RECOGNIZE` (row pattern matching).
- `FETCH FIRST n PERCENT ROWS`.
- `SEARCH DEPTH/BREADTH FIRST` / `CYCLE` in recursive CTEs.
- `SAMPLE [BLOCK]` with `SEED`.
- `@ dblink` remote table access.
- `(+)` old outer join syntax.
- `PIVOT XML`.
- `UNIQUE` as alias for `DISTINCT`.
- `FOR UPDATE WAIT n`.

### 17.3 MySQL-only

- `HIGH_PRIORITY`, `STRAIGHT_JOIN`, `SQL_*` hints.
- `INTO OUTFILE` / `INTO DUMPFILE` / `INTO @var`.
- `GROUP BY ... WITH ROLLUP`.
- `GROUP BY col ASC|DESC`.
- `LIMIT offset, count` (offset-first syntax).
- `PARTITION (p1, ...)` in FROM.
- Index hints: `USE INDEX`, `IGNORE INDEX`, `FORCE INDEX`.
- `{ OJ ... }` ODBC outer join.
- `DISTINCTROW`.
- `FOR SHARE` (replaces `LOCK IN SHARE MODE`).
- `JSON_TABLE()`.

### 17.4 SQL Server-only

- `TOP n [PERCENT] [WITH TIES]`.
- `SELECT ... INTO new_table`.
- `OPTION (query_hint, ...)`.
- `FOR XML` / `FOR JSON`.
- `WITH (table_hint)`.
- `CROSS APPLY` / `OUTER APPLY`.
- `PIVOT` / `UNPIVOT`.
- `OPENROWSET` / `OPENDATASOURCE`.
- No `NATURAL JOIN`.
- No `USING` in joins.
- No `NULLS FIRST | LAST`.

### 17.5 SQLite-only

- `INDEXED BY index-name` / `NOT INDEXED`.
- `LIMIT expr, expr` (offset as second arg, reversed from MySQL).
- Most limited feature set of the five.

---

## Comparison Table: Key SELECT Features

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| **CTE** | Yes | Yes | Yes | Yes | Yes |
| **Recursive CTE** | Yes | Yes | Yes | Yes | Yes |
| **DISTINCT ON** | Yes | No | No | No | No |
| **LIMIT/OFFSET** | Yes | Yes | Yes | No | No |
| **FETCH FIRST** | Yes | No | Yes (8.0.37+) | Yes | Yes |
| **TOP** | No | No | No | No | Yes |
| **UNION/INTERSECT/EXCEPT** | All + ALL | UNION, INTERSECT, EXCEPT | All + ALL (8.0.31+) | UNION, INTERSECT, MINUS | UNION, INTERSECT, EXCEPT |
| **ROLLUP** | Yes | No | WITH ROLLUP | Yes | Yes |
| **CUBE** | Yes | No | No | Yes | Yes |
| **GROUPING SETS** | Yes | No | No | Yes | Yes |
| **WINDOW clause** | Yes | Yes | Yes | No | Yes (2022) |
| **LATERAL** | Yes | No | Yes | Yes | APPLY |
| **TABLESAMPLE** | Yes | No | No | SAMPLE | Yes |
| **PIVOT/UNPIVOT** | No | No | No | Yes | Yes |
| **FOR UPDATE** | Yes | No | Yes | Yes | Hints |
| **SKIP LOCKED** | Yes | No | Yes | Yes | READPAST |
| **NULLS FIRST/LAST** | Yes | Yes | No | Yes | No |
| **Named windows** | Yes | Yes | Yes | No | Yes |
| **JSON_TABLE** | Yes (17+) | No | Yes | Yes | Yes |

---

## Notes for AST Design

1. **Core SELECT structure** is consistent: SELECT columns FROM source WHERE cond GROUP BY HAVING ORDER BY.
2. **Pagination** has 3 families: LIMIT/OFFSET (PG, SQLite, MySQL), FETCH FIRST (PG, Oracle, SQL Server), TOP (SQL Server only).
3. **Set operations** are standard except Oracle uses `MINUS` instead of `EXCEPT`.
4. **LATERAL** vs `APPLY` — same semantics, different syntax.
5. **Row locking** varies greatly: PG has 4 lock strengths, Oracle has WAIT N, SQL Server uses table hints.
6. **GROUP BY extensions** (ROLLUP/CUBE/GROUPING SETS) — MySQL uses different syntax (`WITH ROLLUP`).
7. **DISTINCT ON** is PG-only — very useful, should be in AST.
8. **TABLESAMPLE/SAMPLE** — same concept, different syntax per DB.
9. **PIVOT/UNPIVOT** — Oracle and SQL Server only, significant syntax.
10. **Optimizer hints** — each DB has its own system (PG: none in SQL, Oracle: `/*+ */`, MySQL: various, SQL Server: `OPTION()`). Consider keeping as raw strings.
11. **FOR XML/JSON** (SQL Server) and **INTO OUTFILE** (MySQL) are output format features — rare in ORM usage, candidates for Custom extension.
