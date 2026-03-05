# DML Syntax Reference (All Dialects)

Full syntax for latest versions: PostgreSQL 17, SQLite 3.45+, MySQL 8.4, Oracle 23c, SQL Server 2022.

---

## 1. INSERT

### 1.1 PostgreSQL 17

```sql
[ WITH [ RECURSIVE ] with_query [, ...] ]
INSERT INTO table_name [ AS alias ] [ ( column_name [, ...] ) ]
    [ OVERRIDING { SYSTEM | USER } VALUE ]
    { DEFAULT VALUES | VALUES ( { expression | DEFAULT } [, ...] ) [, ...] | query }
    [ ON CONFLICT [ conflict_target ] conflict_action ]
    [ RETURNING { * | output_expression [ [ AS ] output_name ] } [, ...] ]
```

**ON CONFLICT (Upsert):**
```sql
-- conflict_target:
( { index_column_name | ( index_expression ) } [ COLLATE collation ] [ opclass ] [, ...] )
    [ WHERE index_predicate ]
ON CONSTRAINT constraint_name

-- conflict_action:
DO NOTHING
DO UPDATE SET { column_name = { expression | DEFAULT } |
                ( column_name [, ...] ) = [ ROW ] ( { expression | DEFAULT } [, ...] ) |
                ( column_name [, ...] ) = ( sub-SELECT )
              } [, ...]
              [ WHERE condition ]
```

- `EXCLUDED` table references the row proposed for insertion.
- `OVERRIDING SYSTEM VALUE` overrides identity column sequence values.
- `OVERRIDING USER VALUE` ignores user-supplied values for identity columns.

### 1.2 SQLite 3.45+

```sql
[ WITH [ RECURSIVE ] common-table-expression [, ...] ]
{ INSERT | REPLACE | INSERT OR ROLLBACK | INSERT OR ABORT | INSERT OR FAIL
  | INSERT OR IGNORE | INSERT OR REPLACE }
INTO [ schema-name . ] table-name [ AS alias ]
    [ ( column-name [, ...] ) ]
    { VALUES ( expr [, ...] ) [, ...] | select-stmt | DEFAULT VALUES }
    [ upsert-clause ]
    [ returning-clause ]
```

**Upsert (ON CONFLICT):**
```sql
ON CONFLICT ( indexed-column [, ...] ) [ WHERE expr ]
    DO NOTHING
ON CONFLICT ( indexed-column [, ...] ) [ WHERE expr ]
    DO UPDATE SET column-name = expr [, ...]
    [ WHERE expr ]
```

- Multiple `ON CONFLICT` clauses allowed; processed in order.
- Last clause may omit conflict target (catch-all).
- `excluded.column` references the proposed row.
- Conflict resolution keywords: `ROLLBACK`, `ABORT`, `FAIL`, `IGNORE`, `REPLACE`.
- `REPLACE` is shorthand for `INSERT OR REPLACE`.

### 1.3 MySQL 8.4

```sql
INSERT [LOW_PRIORITY | HIGH_PRIORITY] [IGNORE]
    [INTO] tbl_name
    [PARTITION (partition_name [, ...])]
    [(col_name [, ...])]
    { VALUES | VALUE } ( { expr | DEFAULT } [, ...] ) [, ...]
    [AS row_alias [(col_alias [, ...])]]
    [ON DUPLICATE KEY UPDATE assignment_list]
```

**INSERT ... SET form:**
```sql
INSERT [INTO] tbl_name SET col_name = { expr | DEFAULT } [, ...]
    [AS row_alias] [ON DUPLICATE KEY UPDATE ...]
```

**INSERT ... SELECT form:**
```sql
INSERT [INTO] tbl_name [(col_name [, ...])]
    { SELECT ... | TABLE table_name }
    [ON DUPLICATE KEY UPDATE ...]
```

- `AS row_alias` replaces deprecated `VALUES()` function in ON DUPLICATE KEY UPDATE.
- `LOW_PRIORITY` / `HIGH_PRIORITY` for lock priority.
- `IGNORE` downgrades errors to warnings.
- `PARTITION` clause for partition targeting.
- No RETURNING clause. Use `LAST_INSERT_ID()`.

### 1.4 Oracle 23c

**Single-table:**
```sql
INSERT [ hint ] INTO [ schema. ] table [ t_alias ]
    [ PARTITION ( partition_name ) ]
    [ ( column [, ...] ) ]
    { VALUES ( { expr | DEFAULT } [, ...] ) | subquery }
    [ error_logging_clause ]
    [ RETURNING expr [, ...] INTO data_item [, ...] ]
```

**Multi-table INSERT (unique to Oracle):**
```sql
INSERT [ hint ] { ALL | FIRST }
    [ WHEN condition THEN ]
        INTO table [ (column_list) ] [ VALUES (value_list) ]
    [ WHEN condition THEN ... ] ...
    [ ELSE INTO table ... ]
subquery;
```

**INSERT ... SET (new in 23c):**
```sql
INSERT INTO table SET col1 = expr1, col2 = expr2;
```

**INSERT BY NAME (new in 23c):**
```sql
INSERT INTO target BY NAME SELECT col_a AS target_col1 FROM source;
```

- No multi-row `VALUES (row1), (row2)`. Use `INSERT ALL` or `UNION ALL`.
- `RETURNING ... INTO` returns into bind variables (not result set).
- `LOG ERRORS INTO error_table REJECT LIMIT n` for error logging.
- `/*+ APPEND */` hint for direct-path insert.
- Upsert via `MERGE` statement (no ON CONFLICT).

### 1.5 SQL Server 2022

```sql
[ WITH <common_table_expression> [, ...] ]
INSERT [ TOP ( expression ) [ PERCENT ] ]
    [ INTO ] <object> [ WITH ( <Table_Hint> [, ...] ) ]
    [ ( column_list ) ]
    [ <OUTPUT Clause> ]
    { VALUES ( { DEFAULT | NULL | expression } [, ...] ) [, ...]
      | derived_table
      | execute_statement
      | DEFAULT VALUES
    }
```

**OUTPUT clause:**
```sql
OUTPUT INSERTED.* [INTO @table_variable | output_table]
OUTPUT INSERTED.column_name [, ...]
```

**INSERT ... EXEC (unique to SQL Server):**
```sql
INSERT INTO t (a, b) EXEC sp_get_data @param1;
INSERT INTO t (a, b) EXEC('SELECT x, y FROM source');
```

- `TOP (n) [PERCENT]` limits rows from SELECT.
- Table hints: `WITH (TABLOCK)`.
- Four-part naming: `server.database.schema.table`.
- `SET IDENTITY_INSERT t ON` for explicit identity values.
- Upsert via `MERGE` statement (no ON CONFLICT).
- Up to 1000 rows in VALUES constructor.

---

## 2. UPDATE

### 2.1 PostgreSQL 17

```sql
[ WITH [ RECURSIVE ] with_query [, ...] ]
UPDATE [ ONLY ] table_name [ * ] [ [ AS ] alias ]
    SET { column_name = { expression | DEFAULT } |
          ( column_name [, ...] ) = [ ROW ] ( { expression | DEFAULT } [, ...] ) |
          ( column_name [, ...] ) = ( sub-SELECT )
        } [, ...]
    [ FROM from_item [, ...] ]
    [ WHERE condition | WHERE CURRENT OF cursor_name ]
    [ RETURNING { * | output_expression [ [ AS ] output_name ] } [, ...] ]
```

- `FROM` clause for joining with other tables.
- `ONLY` excludes child/inherited tables.
- Tuple assignment: `(col1, col2) = ROW(expr1, expr2)`.
- No direct `ORDER BY` or `LIMIT` on UPDATE.

### 2.2 SQLite 3.45+

```sql
[ WITH [ RECURSIVE ] common-table-expression [, ...] ]
UPDATE [ OR { ROLLBACK | ABORT | REPLACE | FAIL | IGNORE } ]
  [ schema_name. ] table_name [ [ AS ] alias ]
  [ INDEXED BY index_name | NOT INDEXED ]
SET column_name = expr | ( column_name [, ...] ) = ( row_value ) [, ...]
[ FROM table_or_subquery [ join_clause ] ]
[ WHERE expr ]
[ RETURNING expr [ [ AS ] column_alias ] [, ...] | * ]
[ ORDER BY ordering_term [, ...] ]
[ LIMIT expr [ OFFSET expr ] ]
```

- `FROM` clause since SQLite 3.33.0.
- `ORDER BY` and `LIMIT` require compile-time flag `SQLITE_ENABLE_UPDATE_DELETE_LIMIT`.
- Conflict resolution: `OR ROLLBACK/ABORT/REPLACE/FAIL/IGNORE`.
- `INDEXED BY` / `NOT INDEXED` hints.

### 2.3 MySQL 8.4

**Single-table:**
```sql
[ WITH cte_name AS (...) [, ...] ]
UPDATE [LOW_PRIORITY] [IGNORE] table_reference
    [ PARTITION (partition_list) ]
SET col_name = { expr | DEFAULT } [, ...]
[ WHERE condition ]
[ ORDER BY ... ]
[ LIMIT row_count ]
```

**Multi-table:**
```sql
UPDATE [LOW_PRIORITY] [IGNORE] table_references
    SET col_name = { expr | DEFAULT } [, ...]
[ WHERE condition ]
```

- Multi-table: can SET columns in multiple tables via JOIN.
- Single-table SET evaluated left-to-right (new value available for next assignment).
- `ORDER BY` and `LIMIT` single-table only.
- No `RETURNING`. No `FROM` clause (uses JOIN syntax).

### 2.4 Oracle 23c

```sql
UPDATE [ /*+ hint */ ] [ schema. ] { table | view | ( subquery ) }
    [ t_alias ]
    [ partition_extension_clause ]
SET { column = { expr | ( subquery ) | DEFAULT } |
      ( column [, ...] ) = ( subquery )
    } [, ...]
[ FROM from_using_clause ]
[ WHERE condition ]
[ RETURNING expr [, ...] INTO data_item [, ...] ]
[ LOG ERRORS ... ]
```

- `FROM` clause new in Oracle 23c.
- `RETURNING ... INTO` returns into variables (not result set).
- `LOG ERRORS` for DML error logging.
- No direct `ORDER BY`, `LIMIT`, or CTE support on UPDATE.

### 2.5 SQL Server 2022

```sql
[ WITH <common_table_expression> [, ...] ]
UPDATE [ TOP ( expression ) [ PERCENT ] ]
    { table_alias | <object> | @table_variable }
    [ WITH ( <Table_Hint> [, ...] ) ]
SET { column_name = { expression | DEFAULT | NULL }
      | column_name { += | -= | *= | /= | %= | &= | ^= | |= } expression
      | @variable = expression
      | @variable = column = expression
    } [, ...]
[ <OUTPUT Clause> ]
[ FROM { <table_source> } [, ...] ]
[ WHERE { condition | CURRENT OF cursor_name } ]
[ OPTION ( <query_hint> [, ...] ) ]
```

- `OUTPUT DELETED.*, INSERTED.*` (both old and new values).
- `TOP (n) [PERCENT]` instead of LIMIT.
- Compound assignment: `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `^=`, `|=`.
- `.WRITE(expr, offset, length)` for partial LOB updates.
- Can update through a CTE directly.
- `FROM` clause with full JOIN support.

---

## 3. DELETE

### 3.1 PostgreSQL 17

```sql
[ WITH [ RECURSIVE ] with_query [, ...] ]
DELETE FROM [ ONLY ] table_name [ * ] [ [ AS ] alias ]
    [ USING from_item [, ...] ]
    [ WHERE condition | WHERE CURRENT OF cursor_name ]
    [ RETURNING { * | output_expression [ [ AS ] output_name ] } [, ...] ]
```

- `USING` clause (PostgreSQL's JOIN syntax for DELETE).
- `ONLY` excludes descendant tables.
- No direct `ORDER BY` or `LIMIT` (use CTE workaround).

### 3.2 SQLite 3.45+

```sql
[ WITH [ RECURSIVE ] common-table-expression [, ...] ]
DELETE FROM [ schema-name. ] table-name [ [ AS ] alias ]
    [ INDEXED BY index-name | NOT INDEXED ]
    [ WHERE expr ]
    [ RETURNING expr [ [ AS ] column-alias ] [, ...] | * ]
    [ ORDER BY ordering-term [, ...] ]
    [ LIMIT expr [ OFFSET expr ] ]
```

- `ORDER BY` and `LIMIT` require compile-time flag.
- `INDEXED BY` / `NOT INDEXED` hints.
- No JOIN/USING support — use subqueries in WHERE.

### 3.3 MySQL 8.4

**Single-table:**
```sql
[ WITH cte_name AS (...) [, ...] ]
DELETE [LOW_PRIORITY] [QUICK] [IGNORE]
    FROM tbl_name [ [AS] tbl_alias ]
    [ PARTITION (partition_name [, ...]) ]
    [ WHERE condition ]
    [ ORDER BY ... ]
    [ LIMIT row_count ]
```

**Multi-table (two forms):**
```sql
-- Form 1: tables to delete listed before FROM
DELETE t1, t2 FROM t1 INNER JOIN t2 ON t1.id = t2.id WHERE ...

-- Form 2: USING keyword
DELETE FROM t1, t2 USING t1 INNER JOIN t2 ON t1.id = t2.id WHERE ...
```

- `LOW_PRIORITY`, `QUICK`, `IGNORE` modifiers.
- `PARTITION` clause.
- Multi-table: can delete from multiple tables in one statement.
- `ORDER BY` + `LIMIT` single-table only.
- No `RETURNING` clause.

### 3.4 Oracle 23c

```sql
DELETE [ /*+ hint */ ]
    FROM [ ONLY ] [ schema. ] { table | view | ( subquery ) }
        [ PARTITION (partition_name) ]
        [ @dblink ] [ t_alias ]
    [ FROM table_source [ JOIN ... ON ... ] ]
    [ WHERE condition ]
    [ RETURNING expr [, ...] INTO data_item [, ...] ]
    [ LOG ERRORS ... ]
```

- Second `FROM` clause for joins (new in 23c).
- `@dblink` for remote table deletion.
- `RETURNING ... INTO` (variables only).
- `LOG ERRORS` for DML error logging.
- No direct `ORDER BY` or `LIMIT` (use ROWNUM/FETCH in subquery).

### 3.5 SQL Server 2022

```sql
[ WITH <common_table_expression> [, ...] ]
DELETE [ TOP ( expression ) [ PERCENT ] ]
    [ FROM ] { table_alias | <object> | @table_variable }
    [ WITH ( <Table_Hint> [, ...] ) ]
    [ <OUTPUT Clause> ]
    [ FROM table_source [, ...] ]
    [ WHERE { condition | CURRENT OF cursor_name } ]
    [ OPTION ( <query_hint> [, ...] ) ]
```

- `OUTPUT DELETED.*` (equivalent to RETURNING).
- `TOP (n) [PERCENT]` for limiting rows.
- Second `FROM` clause for JOINs.
- Can delete through a CTE directly.
- Four-part naming for linked servers.

---

## 4. Cross-Database Comparison

### INSERT

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| Multi-row VALUES | Yes | Yes | Yes | No (INSERT ALL) | Yes (≤1000) |
| INSERT ... SELECT | Yes | Yes | Yes | Yes | Yes |
| INSERT ... SET | No | No | Yes | Yes (23c) | No |
| DEFAULT VALUES | Yes | Yes | `() VALUES ()` | No | Yes |
| ON CONFLICT / Upsert | ON CONFLICT DO NOTHING/UPDATE | ON CONFLICT + OR REPLACE/IGNORE | ON DUPLICATE KEY UPDATE | MERGE | MERGE |
| RETURNING / OUTPUT | RETURNING | RETURNING | No | RETURNING INTO (vars) | OUTPUT INSERTED.* |
| CTE (WITH) | Yes (writable) | Yes | No | Yes | Yes |
| Multi-table INSERT | No | No | No | Yes (ALL/FIRST) | No |
| INSERT ... EXEC | No | No | No | No | Yes |
| Identity override | OVERRIDING SYSTEM/USER | N/A | N/A | N/A | IDENTITY_INSERT ON |
| Conflict resolution modes | DO NOTHING, DO UPDATE | ABORT/ROLLBACK/FAIL/IGNORE/REPLACE | IGNORE, ON DUPLICATE KEY | MERGE | MERGE |
| Excluded row reference | `EXCLUDED` table | `excluded` table | `AS row_alias` | N/A | MERGE source |
| Error logging | No | No | No | LOG ERRORS | No |

### UPDATE

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| FROM clause (JOIN) | Yes | Yes (3.33+) | No (JOIN syntax) | Yes (23c) | Yes |
| RETURNING / OUTPUT | RETURNING | RETURNING | No | RETURNING INTO (vars) | OUTPUT DELETED/INSERTED |
| ORDER BY | No | Yes (compile flag) | Yes (single-table) | No | No |
| LIMIT / TOP | No (CTE workaround) | Yes (compile flag) | Yes (single-table) | No | TOP (n) [PERCENT] |
| CTE (WITH) | Yes | Yes | Yes | No | Yes (updatable) |
| Multi-table SET | No | No | Yes | No | No |
| Cursor-based | WHERE CURRENT OF | No | No | PL/SQL only | WHERE CURRENT OF |
| Conflict handling | No | OR REPLACE/IGNORE/etc | IGNORE | LOG ERRORS | No |
| Compound assignment | No | No | No | No | +=, -=, *=, /=, etc |
| Tuple assignment | Yes (ROW) | Yes | No | Yes (subquery) | No |
| Inheritance control | ONLY / * | No | No | No | No |

### DELETE

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| JOIN in DELETE | USING clause | No (subquery) | Multi-table DELETE | Second FROM (23c) | Second FROM |
| RETURNING / OUTPUT | RETURNING | RETURNING | No | RETURNING INTO (vars) | OUTPUT DELETED.* |
| ORDER BY | No | Yes (compile flag) | Yes (single-table) | No | No |
| LIMIT / TOP | No (CTE workaround) | Yes (compile flag) | Yes (single-table) | No | TOP (n) [PERCENT] |
| CTE (WITH) | Yes | Yes | Yes | Yes | Yes |
| Multi-table delete | No | No | Yes | No | No |
| Cursor-based | WHERE CURRENT OF | No | No | Yes | WHERE CURRENT OF |
| Partition targeting | No | No | PARTITION clause | PARTITION/SUBPARTITION | No |
| Error logging | No | No | IGNORE modifier | LOG ERRORS | No |
| Inheritance control | ONLY / * | No | No | ONLY (views) | No |
