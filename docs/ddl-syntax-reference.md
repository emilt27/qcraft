# DDL Syntax Reference (All Dialects)

Full syntax for latest versions: PostgreSQL 17, SQLite 3.45+, MySQL 8.4, Oracle 23c, SQL Server 2022.

---

## 1. CREATE TABLE

### 1.1 PostgreSQL 17

```sql
CREATE [ [ GLOBAL | LOCAL ] { TEMPORARY | TEMP } | UNLOGGED ] TABLE [ IF NOT EXISTS ] table_name (
  { column_name data_type
    [ STORAGE { PLAIN | EXTERNAL | EXTENDED | MAIN | DEFAULT } ]
    [ COMPRESSION compression_method ]
    [ COLLATE collation ]
    [ column_constraint [ ... ] ]
    | table_constraint
    | LIKE source_table [ like_option ... ]
  } [, ... ]
)
[ INHERITS ( parent_table [, ... ] ) ]
[ PARTITION BY { RANGE | LIST | HASH } ( { column_name | ( expression ) }
  [ COLLATE collation ] [ opclass ] [, ... ] ) ]
[ USING method ]
[ WITH ( storage_parameter [= value] [, ... ] ) | WITHOUT OIDS ]
[ ON COMMIT { PRESERVE ROWS | DELETE ROWS | DROP } ]
[ TABLESPACE tablespace_name ]
```

**Column Constraints:**
```sql
[ CONSTRAINT constraint_name ]
{ NOT NULL | NULL
  | CHECK ( expression ) [ NO INHERIT ]
  | DEFAULT default_expr
  | GENERATED ALWAYS AS ( generation_expr ) STORED
  | GENERATED { ALWAYS | BY DEFAULT } AS IDENTITY [ ( sequence_options ) ]
  | UNIQUE [ NULLS [ NOT ] DISTINCT ] index_parameters
  | PRIMARY KEY index_parameters
  | REFERENCES reftable [ ( refcolumn ) ]
    [ MATCH FULL | MATCH PARTIAL | MATCH SIMPLE ]
    [ ON DELETE referential_action ] [ ON UPDATE referential_action ]
}
[ DEFERRABLE | NOT DEFERRABLE ] [ INITIALLY DEFERRED | INITIALLY IMMEDIATE ]
```

**Table Constraints:**
```sql
[ CONSTRAINT constraint_name ]
{ CHECK ( expression ) [ NO INHERIT ]
  | UNIQUE [ NULLS [ NOT ] DISTINCT ] ( column_name [, ... ] ) index_parameters
  | PRIMARY KEY ( column_name [, ... ] ) index_parameters
  | EXCLUDE [ USING index_method ] ( exclude_element WITH operator [, ... ] )
    index_parameters [ WHERE ( predicate ) ]
  | FOREIGN KEY ( column_name [, ... ] ) REFERENCES reftable [ ( refcolumn [, ... ] ) ]
    [ MATCH FULL | MATCH PARTIAL | MATCH SIMPLE ]
    [ ON DELETE referential_action ] [ ON UPDATE referential_action ]
}
[ DEFERRABLE | NOT DEFERRABLE ] [ INITIALLY DEFERRED | INITIALLY IMMEDIATE ]
```

**Index Parameters** (for UNIQUE, PRIMARY KEY, EXCLUDE):
```sql
[ INCLUDE ( column_name [, ... ] ) ]
[ WITH ( storage_parameter [= value] [, ... ] ) ]
[ USING INDEX TABLESPACE tablespace_name ]
```

**Referential Actions:** `NO ACTION | RESTRICT | CASCADE | SET NULL [ (columns) ] | SET DEFAULT [ (columns) ]`

**LIKE Options:** `{ INCLUDING | EXCLUDING } { COMMENTS | COMPRESSION | CONSTRAINTS | DEFAULTS | GENERATED | IDENTITY | INDEXES | STATISTICS | STORAGE | ALL }`

**Partition Bound:** `IN (values) | FROM (values) TO (values) | WITH ( MODULUS n, REMAINDER n )`

**CTAS:**
```sql
CREATE TABLE table_name [ ( column_name [, ...] ) ]
  [ USING method ] [ WITH (...) ] [ ON COMMIT ... ] [ TABLESPACE ... ]
  AS query [ WITH [ NO ] DATA ]
```

### 1.2 SQLite 3.45+

```sql
CREATE [ TEMP | TEMPORARY ] TABLE [ IF NOT EXISTS ] [ schema-name. ] table-name (
  column-def [, column-def] ...
  [, table-constraint [, table-constraint] ...]
) [ table-options ]
```

**Column Definition:**
```sql
column-name [ type-name ] [ column-constraint ... ]
```

**Column Constraints:**
```sql
[ CONSTRAINT name ]
{ PRIMARY KEY [ ASC | DESC ] [ conflict-clause ] [ AUTOINCREMENT ]
  | NOT NULL [ conflict-clause ]
  | UNIQUE [ conflict-clause ]
  | CHECK ( expr )
  | DEFAULT { literal-value | ( expr ) | CURRENT_TIME | CURRENT_DATE | CURRENT_TIMESTAMP }
  | COLLATE collation-name
  | REFERENCES foreign-table [ ( column-name [, ...] ) ]
    [ ON DELETE { SET NULL | SET DEFAULT | CASCADE | RESTRICT | NO ACTION } ]
    [ ON UPDATE { SET NULL | SET DEFAULT | CASCADE | RESTRICT | NO ACTION } ]
    [ MATCH name ]
    [ [ NOT ] DEFERRABLE [ INITIALLY { DEFERRED | IMMEDIATE } ] ]
  | GENERATED ALWAYS AS ( expr ) [ VIRTUAL | STORED ]
}
```

**Table Constraints:**
```sql
[ CONSTRAINT name ]
{ PRIMARY KEY ( indexed-column [, ...] ) [ conflict-clause ]
  | UNIQUE ( indexed-column [, ...] ) [ conflict-clause ]
  | CHECK ( expr )
  | FOREIGN KEY ( column-name [, ...] ) REFERENCES foreign-table [ ( column-name [, ...] ) ]
    [ ON DELETE action ] [ ON UPDATE action ] [ MATCH name ]
    [ [ NOT ] DEFERRABLE [ INITIALLY { DEFERRED | IMMEDIATE } ] ]
}
```

**Conflict Clause (SQLite-specific):** `ON CONFLICT { ROLLBACK | ABORT | FAIL | IGNORE | REPLACE }`

**Table Options:** `WITHOUT ROWID` | `STRICT` (can be combined)

**CTAS:** `CREATE TABLE table-name AS select-stmt` (no constraints created)

### 1.3 MySQL 8.4

```sql
CREATE [ TEMPORARY ] TABLE [ IF NOT EXISTS ] tbl_name (
  create_definition, ...
) [ table_options ] [ partition_options ]
```

**Column Definition:**
```sql
col_name data_type
  [ NOT NULL | NULL ]
  [ DEFAULT { literal | ( expr ) } ]
  [ VISIBLE | INVISIBLE ]
  [ AUTO_INCREMENT ]
  [ UNIQUE [ KEY ] ]
  [ [ PRIMARY ] KEY ]
  [ COMMENT 'string' ]
  [ COLLATE collation_name ]
  [ COLUMN_FORMAT { FIXED | DYNAMIC | DEFAULT } ]
  [ STORAGE { DISK | MEMORY } ]
  [ reference_definition ]
  [ check_constraint_definition ]
```

**Generated Column:**
```sql
col_name data_type [ COLLATE collation_name ]
  [ GENERATED ALWAYS ] AS ( expr ) [ VIRTUAL | STORED ]
  [ NOT NULL | NULL ] [ VISIBLE | INVISIBLE ] [ UNIQUE [ KEY ] ] [ [ PRIMARY ] KEY ]
  [ COMMENT 'string' ] [ reference_definition ] [ check_constraint_definition ]
```

**Index/Key Definitions:**
```sql
{ INDEX | KEY } [ index_name ] [ USING { BTREE | HASH } ] ( key_part, ... ) [ index_option ... ]
{ FULLTEXT | SPATIAL } [ INDEX | KEY ] [ index_name ] ( key_part, ... ) [ index_option ... ]
[ CONSTRAINT [ symbol ] ] PRIMARY KEY [ USING index_type ] ( key_part, ... ) [ index_option ... ]
[ CONSTRAINT [ symbol ] ] UNIQUE [ INDEX | KEY ] [ index_name ] [ USING index_type ] ( key_part, ... ) [ index_option ... ]
[ CONSTRAINT [ symbol ] ] FOREIGN KEY [ index_name ] ( col_name, ... ) reference_definition
[ CONSTRAINT [ symbol ] ] CHECK ( expr ) [ [ NOT ] ENFORCED ]
```

**Key Part:** `{ col_name [ ( length ) ] | ( expr ) } [ ASC | DESC ]`

**Reference Definition:**
```sql
REFERENCES tbl_name ( key_part, ... )
  [ MATCH FULL | MATCH PARTIAL | MATCH SIMPLE ]
  [ ON DELETE { RESTRICT | CASCADE | SET NULL | NO ACTION | SET DEFAULT } ]
  [ ON UPDATE { RESTRICT | CASCADE | SET NULL | NO ACTION | SET DEFAULT } ]
```

**Table Options (selected):**
```sql
ENGINE [=] engine_name
| AUTO_INCREMENT [=] value
| [ DEFAULT ] CHARACTER SET [=] charset_name
| [ DEFAULT ] COLLATE [=] collation_name
| COMMENT [=] 'string'
| COMPRESSION [=] { 'ZLIB' | 'LZ4' | 'NONE' }
| ENCRYPTION [=] { 'Y' | 'N' }
| ROW_FORMAT [=] { DEFAULT | DYNAMIC | FIXED | COMPRESSED | REDUNDANT | COMPACT }
| TABLESPACE tablespace_name [ STORAGE { DISK | MEMORY } ]
| KEY_BLOCK_SIZE [=] value
| DATA DIRECTORY [=] 'path'
| INDEX DIRECTORY [=] 'path'
```

**Partition Options:**
```sql
PARTITION BY { [ LINEAR ] HASH ( expr ) | [ LINEAR ] KEY [ ALGORITHM={1|2} ] ( column_list )
  | RANGE { ( expr ) | COLUMNS ( column_list ) }
  | LIST { ( expr ) | COLUMNS ( column_list ) } }
[ PARTITIONS num ]
[ SUBPARTITION BY { [ LINEAR ] HASH ( expr ) | [ LINEAR ] KEY [ ALGORITHM={1|2} ] ( column_list ) }
  [ SUBPARTITIONS num ] ]
[ ( partition_definition [, ...] ) ]
```

**LIKE:** `CREATE TABLE new_tbl LIKE orig_tbl` (copies columns + indexes, not data/FKs)

**CTAS:** `CREATE TABLE tbl [ ( create_definition, ... ) ] [ table_options ] [ IGNORE | REPLACE ] [ AS ] query`

### 1.4 Oracle 23c

```sql
CREATE [ GLOBAL TEMPORARY | PRIVATE TEMPORARY ] TABLE [ IF NOT EXISTS ] [ schema. ] table
  [ SHARING = { METADATA | DATA | EXTENDED DATA | NONE } ]
  [ IMMUTABLE [ NO DROP | NO DELETE ] ]
  [ BLOCKCHAIN ]
  ( column_definitions_and_constraints )
  [ physical_properties ]
  [ table_properties ]
```

**Column Definition:**
```sql
column { datatype | DOMAIN domain_name }
  [ DEFAULT { expr | ON NULL expr } ]
  [ GENERATED { ALWAYS | BY DEFAULT [ ON NULL ] } AS IDENTITY [ ( sequence_options ) ] ]
  [ VISIBLE | INVISIBLE ]
  [ COLLATE collation_name ]
  [ ENCRYPT [ USING 'algorithm' ] [ IDENTIFIED BY 'password' ] [ NO SALT | SALT ] ]
  [ inline_constraint ... ]
```

**Virtual Column:**
```sql
column [ datatype ] [ GENERATED ALWAYS ] AS ( expression ) [ VISIBLE | INVISIBLE ]
  [ inline_constraint ... ]
```

**Inline Constraints:**
```sql
[ CONSTRAINT constraint_name ]
{ NOT NULL | UNIQUE | PRIMARY KEY
  | CHECK ( condition )
  | REFERENCES [ schema. ] table [ ( column ) ] [ ON DELETE { CASCADE | SET NULL } ]
}
[ DEFERRABLE | NOT DEFERRABLE ] [ INITIALLY DEFERRED | INITIALLY IMMEDIATE ]
```

**Out-of-Line Constraints:**
```sql
[ CONSTRAINT constraint_name ]
{ UNIQUE ( column [, ...] )
  | PRIMARY KEY ( column [, ...] )
  | FOREIGN KEY ( column [, ...] ) REFERENCES table [ ( column [, ...] ) ]
    [ ON DELETE { CASCADE | SET NULL } ]
  | CHECK ( condition )
}
[ DEFERRABLE | NOT DEFERRABLE ] [ INITIALLY DEFERRED | INITIALLY IMMEDIATE ]
```

Note: Oracle does NOT support `ON UPDATE` for foreign keys.

**Table Organization:**
```sql
ORGANIZATION { HEAP | INDEX | EXTERNAL }
```

**Partitioning:** RANGE, LIST, HASH, REFERENCE, SYSTEM, INTERVAL, composite (all combinations)

**Temp Tables:**
```sql
CREATE GLOBAL TEMPORARY TABLE ... ON COMMIT { DELETE ROWS | PRESERVE ROWS }
CREATE PRIVATE TEMPORARY TABLE ... ON COMMIT { DROP DEFINITION | PRESERVE DEFINITION }
```

**CTAS:** `CREATE TABLE table [ ( column [, ...] ) ] [ properties ] AS subquery [ WITH [ NO ] DATA ]`

### 1.5 SQL Server 2022

```sql
CREATE TABLE { database.schema.table | schema.table | table }
  [ AS FileTable | AS { NODE | EDGE } ]
  ( { column_definition | computed_column_definition | column_set_definition
      | table_constraint | table_index } [, ... ]
    [ PERIOD FOR SYSTEM_TIME ( start_col, end_col ) ]
  )
  [ ON { partition_scheme ( column ) | filegroup | "default" } ]
  [ TEXTIMAGE_ON { filegroup | "default" } ]
  [ FILESTREAM_ON { partition_scheme | filegroup | "default" } ]
  [ WITH ( table_option [, ... ] ) ]
```

**Column Definition:**
```sql
column_name data_type
  [ FILESTREAM ]
  [ COLLATE collation_name ]
  [ SPARSE ]
  [ MASKED WITH ( FUNCTION = 'mask_function' ) ]
  [ [ CONSTRAINT name ] DEFAULT constant_expression ]
  [ IDENTITY [ ( seed, increment ) ] [ NOT FOR REPLICATION ] ]
  [ GENERATED ALWAYS AS { ROW | TRANSACTION_ID | SEQUENCE_NUMBER } { START | END } [ HIDDEN ] ]
  [ NULL | NOT NULL ]
  [ ROWGUIDCOL ]
  [ ENCRYPTED WITH ( COLUMN_ENCRYPTION_KEY = key, ENCRYPTION_TYPE = { DETERMINISTIC | RANDOMIZED },
    ALGORITHM = 'AEAD_AES_256_CBC_HMAC_SHA_256' ) ]
  [ column_constraint [, ... ] ]
```

**Computed Column:** `column_name AS expression [ PERSISTED [ NOT NULL ] ] [ column_constraint ]`

**Table Constraints:**
```sql
[ CONSTRAINT constraint_name ]
{ { PRIMARY KEY | UNIQUE } [ CLUSTERED | NONCLUSTERED ]
    ( column [ ASC | DESC ] [, ... ] )
    [ WITH FILLFACTOR = n | WITH ( index_option [, ... ] ) ]
    [ ON { partition_scheme ( column ) | filegroup | "default" } ]
  | FOREIGN KEY ( column [, ... ] ) REFERENCES table [ ( column [, ... ] ) ]
    [ ON DELETE { NO ACTION | CASCADE | SET NULL | SET DEFAULT } ]
    [ ON UPDATE { NO ACTION | CASCADE | SET NULL | SET DEFAULT } ]
    [ NOT FOR REPLICATION ]
  | CHECK [ NOT FOR REPLICATION ] ( logical_expression )
  | CONNECTION ( { node_table TO node_table } [, ...] )  -- graph tables
}
```

**Table Options (WITH clause, selected):**
```sql
DATA_COMPRESSION = { NONE | ROW | PAGE } [ ON PARTITIONS (...) ]
| SYSTEM_VERSIONING = ON [ ( HISTORY_TABLE = schema.table [, HISTORY_RETENTION_PERIOD = ...] ) ]
| LEDGER = ON [ ( LEDGER_VIEW = schema.view [...] [, APPEND_ONLY = ON | OFF ] ) ]
| MEMORY_OPTIMIZED = ON
| DURABILITY = { SCHEMA_ONLY | SCHEMA_AND_DATA }
```

**Temp Tables:** `#local_temp` (session) or `##global_temp` (all sessions) via name prefix.

**IF NOT EXISTS:** Not supported in CREATE TABLE. Workaround: `IF NOT EXISTS (SELECT ...) CREATE TABLE ...`

**CTAS equivalent:** `SELECT ... INTO new_table FROM source` (no constraints/indexes copied)

---

## 2. ALTER TABLE

### 2.1 PostgreSQL 17

```sql
ALTER TABLE [ IF EXISTS ] [ ONLY ] name [ * ]
    action [, ... ]
```

**Column Actions:**
```sql
ADD [ COLUMN ] [ IF NOT EXISTS ] column_name data_type
  [ COLLATE collation ] [ column_constraint [ ... ] ]
DROP [ COLUMN ] [ IF EXISTS ] column_name [ RESTRICT | CASCADE ]
ALTER [ COLUMN ] column_name [ SET DATA ] TYPE data_type [ COLLATE collation ] [ USING expression ]
ALTER [ COLUMN ] column_name SET DEFAULT expression
ALTER [ COLUMN ] column_name DROP DEFAULT
ALTER [ COLUMN ] column_name { SET | DROP } NOT NULL
ALTER [ COLUMN ] column_name SET EXPRESSION AS ( expression )
ALTER [ COLUMN ] column_name DROP EXPRESSION [ IF EXISTS ]
ALTER [ COLUMN ] column_name ADD GENERATED { ALWAYS | BY DEFAULT } AS IDENTITY [ ( sequence_options ) ]
ALTER [ COLUMN ] column_name { SET GENERATED { ALWAYS | BY DEFAULT } | SET sequence_option | RESTART } [...]
ALTER [ COLUMN ] column_name DROP IDENTITY [ IF EXISTS ]
ALTER [ COLUMN ] column_name SET STATISTICS { integer | DEFAULT }
ALTER [ COLUMN ] column_name SET ( attribute_option = value [, ... ] )
ALTER [ COLUMN ] column_name RESET ( attribute_option [, ... ] )
ALTER [ COLUMN ] column_name SET STORAGE { PLAIN | EXTERNAL | EXTENDED | MAIN | DEFAULT }
ALTER [ COLUMN ] column_name SET COMPRESSION compression_method
```

**Constraint Actions:**
```sql
ADD table_constraint [ NOT VALID ]
ADD table_constraint_using_index
DROP CONSTRAINT [ IF EXISTS ] constraint_name [ RESTRICT | CASCADE ]
ALTER CONSTRAINT constraint_name [ DEFERRABLE | NOT DEFERRABLE ]
  [ INITIALLY DEFERRED | INITIALLY IMMEDIATE ]
VALIDATE CONSTRAINT constraint_name
RENAME CONSTRAINT constraint_name TO new_name
```

**Rename Actions:**
```sql
RENAME [ COLUMN ] column_name TO new_column_name
RENAME TO new_name
```

**Other Actions:**
```sql
SET SCHEMA new_schema
OWNER TO { new_owner | CURRENT_ROLE | CURRENT_USER | SESSION_USER }
SET TABLESPACE new_tablespace
SET { LOGGED | UNLOGGED }
SET ( storage_parameter [= value] [, ... ] )
RESET ( storage_parameter [, ... ] )
SET ACCESS METHOD { new_access_method | DEFAULT }
CLUSTER ON index_name
SET WITHOUT CLUSTER
INHERIT parent_table
NO INHERIT parent_table
OF type_name
NOT OF
REPLICA IDENTITY { DEFAULT | USING INDEX index_name | FULL | NOTHING }
ENABLE / DISABLE TRIGGER [ trigger_name | ALL | USER ]
ENABLE / DISABLE RULE rewrite_rule_name
{ ENABLE | DISABLE | FORCE | NO FORCE } ROW LEVEL SECURITY
ATTACH PARTITION partition_name { FOR VALUES partition_bound_spec | DEFAULT }
DETACH PARTITION partition_name [ CONCURRENTLY | FINALIZE ]
```

### 2.2 SQLite 3.45+

SQLite supports only **four** ALTER TABLE operations:

```sql
ALTER TABLE table-name RENAME TO new-table-name
ALTER TABLE table-name RENAME COLUMN column-name TO new-column-name
ALTER TABLE table-name ADD COLUMN column-def
ALTER TABLE table-name DROP COLUMN column-name
```

**NOT supported:** Modify column type, set/drop DEFAULT, set/drop NOT NULL, ADD/DROP CONSTRAINT, any other ALTER.

**Workaround:** 12-step procedure: create new table, copy data, drop old, rename new.

### 2.3 MySQL 8.4

```sql
ALTER TABLE tbl_name [ alter_option [, alter_option] ... ] [ partition_options ]
```

**Column Actions:**
```sql
ADD [ COLUMN ] col_name column_definition [ FIRST | AFTER col_name ]
ADD [ COLUMN ] ( col_name column_definition, ... )
DROP [ COLUMN ] col_name
MODIFY [ COLUMN ] col_name column_definition [ FIRST | AFTER col_name ]
CHANGE [ COLUMN ] old_col_name new_col_name column_definition [ FIRST | AFTER col_name ]
RENAME COLUMN old_col_name TO new_col_name
ALTER [ COLUMN ] col_name SET DEFAULT { literal | ( expr ) }
ALTER [ COLUMN ] col_name DROP DEFAULT
ALTER [ COLUMN ] col_name SET { VISIBLE | INVISIBLE }
```

**Constraint Actions:**
```sql
ADD [ CONSTRAINT [ symbol ] ] PRIMARY KEY [ index_type ] ( key_part, ... ) [ index_option ... ]
ADD [ CONSTRAINT [ symbol ] ] UNIQUE [ INDEX | KEY ] [ index_name ] [ index_type ] ( key_part, ... )
ADD [ CONSTRAINT [ symbol ] ] FOREIGN KEY [ index_name ] ( col_name, ... ) reference_definition
ADD [ CONSTRAINT [ symbol ] ] CHECK ( expr ) [ [ NOT ] ENFORCED ]
ADD { INDEX | KEY } [ index_name ] [ index_type ] ( key_part, ... ) [ index_option ... ]
ADD { FULLTEXT | SPATIAL } [ INDEX | KEY ] [ index_name ] ( key_part, ... )
DROP PRIMARY KEY
DROP { INDEX | KEY } index_name
DROP FOREIGN KEY fk_symbol
DROP { CHECK | CONSTRAINT } symbol
ALTER { CHECK | CONSTRAINT } symbol [ NOT ] ENFORCED
ALTER INDEX index_name { VISIBLE | INVISIBLE }
RENAME { INDEX | KEY } old_index_name TO new_index_name
```

**Rename:** `RENAME [ TO | AS ] new_tbl_name`

**MySQL-specific:**
```sql
ALGORITHM [=] { DEFAULT | INSTANT | INPLACE | COPY }
LOCK [=] { DEFAULT | NONE | SHARED | EXCLUSIVE }
CONVERT TO CHARACTER SET charset_name [ COLLATE collation_name ]
{ DISABLE | ENABLE } KEYS
FORCE
ORDER BY col_name [, ...]
{ WITHOUT | WITH } VALIDATION
-- Plus all table options (ENGINE, AUTO_INCREMENT, ROW_FORMAT, etc.)
-- Plus extensive partition operations
```

### 2.4 Oracle 23c

```sql
ALTER TABLE [ schema. ] table_name { column_clauses | constraint_clauses | ... }
```

**Column Actions:**
```sql
ADD ( column datatype [ DEFAULT [ ON NULL ] expr ] [ identity_clause ] [ inline_constraint ... ] )
DROP COLUMN column_name [ CASCADE CONSTRAINTS ] [ INVALIDATE ]
DROP ( column1, column2, ... ) [ CASCADE CONSTRAINTS ] [ INVALIDATE ]
SET UNUSED COLUMN column_name  /  DROP UNUSED COLUMNS [ CHECKPOINT integer ]
MODIFY column_name { new_datatype | DEFAULT expr | NOT NULL | NULL | VISIBLE | INVISIBLE }
MODIFY column_name { GENERATED { ALWAYS | BY DEFAULT [ON NULL] } AS IDENTITY | DROP IDENTITY }
RENAME COLUMN old_name TO new_name
```

**Constraint Actions:**
```sql
ADD [ CONSTRAINT name ] { PRIMARY KEY | UNIQUE | FOREIGN KEY | CHECK } ...
  [ DEFERRABLE ] [ INITIALLY { DEFERRED | IMMEDIATE } ]
  [ ENABLE | DISABLE ] [ VALIDATE | NOVALIDATE ] [ RELY | NORELY ]
DROP { PRIMARY KEY | UNIQUE ( columns ) | CONSTRAINT name } [ CASCADE ]
ENABLE CONSTRAINT name [ USING INDEX (...) ] [ VALIDATE | NOVALIDATE ]
DISABLE CONSTRAINT name [ CASCADE ]
RENAME CONSTRAINT old_name TO new_name
```

**Rename:** `RENAME TO new_name`

Note: Oracle does NOT support ON UPDATE for foreign keys.

**Oracle-specific:**
```sql
READ ONLY / READ WRITE
MOVE [ TABLESPACE ts ] [ COMPRESS ... ] [ UPDATE GLOBAL INDEXES ]
SHRINK SPACE [ COMPACT ] [ CASCADE ]
{ ENABLE | DISABLE } ROW MOVEMENT
INMEMORY / NO INMEMORY
FLASHBACK ARCHIVE archive_name / OFF
PARALLEL [ n ] / NOPARALLEL
-- Extensive partition operations (ADD, DROP, TRUNCATE, SPLIT, MERGE, EXCHANGE, etc.)
```

### 2.5 SQL Server 2022

```sql
ALTER TABLE { database.schema.table | schema.table | table } { alter_action }
```

**Column Actions:**
```sql
ADD column_name data_type [ COLLATE ] [ SPARSE ] [ MASKED WITH (...) ]
  [ DEFAULT expr [ WITH VALUES ] ] [ IDENTITY ] [ NULL | NOT NULL ] [ column_constraint ... ]
ADD column_name AS expression [ PERSISTED [ NOT NULL ] ]
DROP COLUMN [ IF EXISTS ] column_name [, ...]
ALTER COLUMN column_name { type [ COLLATE ] [ NULL | NOT NULL ] [ SPARSE ]
  | { ADD | DROP } { ROWGUIDCOL | PERSISTED | NOT FOR REPLICATION | SPARSE | HIDDEN }
  | { ADD | DROP } MASKED [ WITH (...) ] }
  [ WITH ( ONLINE = ON | OFF ) ]
```

**Constraint Actions:**
```sql
ADD [ CONSTRAINT name ] { PRIMARY KEY | UNIQUE } [ CLUSTERED | NONCLUSTERED ]
  ( column [ ASC | DESC ] [, ...] ) [ WITH (...) ] [ ON ... ]
ADD [ CONSTRAINT name ] FOREIGN KEY ( columns ) REFERENCES table [ ( columns ) ]
  [ ON DELETE action ] [ ON UPDATE action ] [ NOT FOR REPLICATION ]
ADD [ CONSTRAINT name ] DEFAULT expr FOR column [ WITH VALUES ]
ADD [ CONSTRAINT name ] CHECK [ NOT FOR REPLICATION ] ( expr )
DROP CONSTRAINT [ IF EXISTS ] constraint_name [ WITH ( MAXDOP | ONLINE | MOVE TO ) ]
[ WITH { CHECK | NOCHECK } ] { CHECK | NOCHECK } CONSTRAINT { ALL | name [, ...] }
```

**Rename:** via `EXEC sp_rename 'old', 'new'` (not ALTER TABLE)
**Rename column:** via `EXEC sp_rename 'table.old_col', 'new_col', 'COLUMN'`

**SQL Server-specific:**
```sql
{ ENABLE | DISABLE } TRIGGER { ALL | trigger_name [, ...] }
{ ENABLE | DISABLE } CHANGE_TRACKING [ WITH (...) ]
SWITCH [ PARTITION n ] TO target [ PARTITION n ] [ WITH (...) ]
SET ( SYSTEM_VERSIONING = ON | OFF [(...)] )
ADD / DROP PERIOD FOR SYSTEM_TIME
REBUILD [ PARTITION = ALL | n ] [ WITH ( DATA_COMPRESSION = ... [, ONLINE = ...] ) ]
SET ( LOCK_ESCALATION = { AUTO | TABLE | DISABLE } )
```

---

## 3. DROP TABLE

### 3.1 PostgreSQL 17

```sql
DROP TABLE [ IF EXISTS ] name [, ...] [ CASCADE | RESTRICT ]
```

### 3.2 SQLite 3.45+

```sql
DROP TABLE [ IF EXISTS ] [ schema-name. ] table-name
```

No CASCADE/RESTRICT. No multiple tables.

### 3.3 MySQL 8.4

```sql
DROP [ TEMPORARY ] TABLE [ IF EXISTS ] tbl_name [, ...] [ RESTRICT | CASCADE ]
```

RESTRICT/CASCADE accepted but are **no-ops** (do nothing).

### 3.4 Oracle 23c

```sql
DROP TABLE [ IF EXISTS ] [ schema. ] table [ CASCADE CONSTRAINTS ] [ PURGE ]
```

PURGE = skip recycle bin. No RESTRICT.

### 3.5 SQL Server 2022

```sql
DROP TABLE [ IF EXISTS ] { database.schema.table | schema.table | table } [, ...]
```

No CASCADE. Must manually drop FK constraints first.

---

## 4. CREATE INDEX

### 4.1 PostgreSQL 17

```sql
CREATE [ UNIQUE ] INDEX [ CONCURRENTLY ] [ [ IF NOT EXISTS ] name ]
  ON [ ONLY ] table_name [ USING method ]
  ( { column_name | ( expression ) }
    [ COLLATE collation ] [ opclass [ ( opclass_parameter = value [, ... ] ) ] ]
    [ ASC | DESC ] [ NULLS { FIRST | LAST } ]
    [, ...] )
  [ INCLUDE ( column_name [, ...] ) ]
  [ NULLS [ NOT ] DISTINCT ]
  [ WITH ( storage_parameter [= value] [, ... ] ) ]
  [ TABLESPACE tablespace_name ]
  [ WHERE predicate ]
```

**Index methods:** btree (default), hash, gist, spgist, gin, brin, user-defined.

**Storage params:** fillfactor, deduplicate_items (btree), buffering (gist), fastupdate/gin_pending_list_limit (gin), pages_per_range/autosummarize (brin).

### 4.2 SQLite 3.45+

```sql
CREATE [ UNIQUE ] INDEX [ IF NOT EXISTS ] [ schema-name. ] index-name
  ON table-name ( { column-name | expr } [ COLLATE collation-name ] [ ASC | DESC ] [, ...] )
  [ WHERE expr ]
```

B-tree only. No INCLUDE, no NULLS FIRST/LAST, no CONCURRENTLY, no index types.

### 4.3 MySQL 8.4

```sql
CREATE [ UNIQUE | FULLTEXT | SPATIAL ] INDEX index_name
  [ USING { BTREE | HASH } ]
  ON tbl_name ( { col_name [ ( length ) ] | ( expr ) } [ ASC | DESC ] [, ...] )
  [ KEY_BLOCK_SIZE [=] value ]
  [ USING { BTREE | HASH } ]
  [ WITH PARSER parser_name ]
  [ COMMENT 'string' ]
  [ { VISIBLE | INVISIBLE } ]
  [ ALGORITHM [=] { DEFAULT | INPLACE | COPY } ]
  [ LOCK [=] { DEFAULT | NONE | SHARED | EXCLUSIVE } ]
```

No IF NOT EXISTS. No WHERE (partial indexes). No INCLUDE. Prefix indexes supported.

### 4.4 Oracle 23c

```sql
CREATE [ UNIQUE ] [ BITMAP ] [ MULTIVALUE ] INDEX [ IF NOT EXISTS ] [ schema. ] index_name
  ON [ schema. ] table ( { column | column_expression } [ ASC | DESC ] [, ...] )
  [ physical_attributes ]
  [ TABLESPACE tablespace_name ]
  [ COMPRESS [ integer ] | COMPRESS ADVANCED [ LOW | HIGH ] | NOCOMPRESS ]
  [ { LOGGING | NOLOGGING } ]
  [ { VISIBLE | INVISIBLE } ]
  [ { USABLE | UNUSABLE } ]
  [ REVERSE ]
  [ PARALLEL [ n ] | NOPARALLEL ]
  [ ONLINE ]
  [ { DEFERRED | IMMEDIATE } INVALIDATION ]
  -- Partitioned: LOCAL | GLOBAL PARTITION BY { RANGE | HASH } ...
```

No WHERE (partial indexes). No INCLUDE.

### 4.5 SQL Server 2022

```sql
CREATE [ UNIQUE ] [ CLUSTERED | NONCLUSTERED ] INDEX index_name
  ON { database.schema.table | schema.table | table }
  ( column [ ASC | DESC ] [, ... ] )
  [ INCLUDE ( column [, ... ] ) ]
  [ WHERE filter_predicate ]
  [ WITH ( relational_index_option [, ... ] ) ]
  [ ON { partition_scheme ( column ) | filegroup | "default" } ]
```

**Index options:** PAD_INDEX, FILLFACTOR, SORT_IN_TEMPDB, IGNORE_DUP_KEY, STATISTICS_NORECOMPUTE, DROP_EXISTING, ONLINE, RESUMABLE, MAX_DURATION, ALLOW_ROW_LOCKS, ALLOW_PAGE_LOCKS, OPTIMIZE_FOR_SEQUENTIAL_KEY, MAXDOP, DATA_COMPRESSION, XML_COMPRESSION.

No IF NOT EXISTS (use workaround). Separate statements for XML, SPATIAL, COLUMNSTORE indexes.

---

## 5. DROP INDEX

### 5.1 PostgreSQL 17

```sql
DROP INDEX [ CONCURRENTLY ] [ IF EXISTS ] name [, ...] [ CASCADE | RESTRICT ]
```

### 5.2 SQLite 3.45+

```sql
DROP INDEX [ IF EXISTS ] [ schema-name. ] index-name
```

### 5.3 MySQL 8.4

```sql
DROP INDEX index_name ON tbl_name
  [ ALGORITHM [=] { DEFAULT | INPLACE | COPY } ]
  [ LOCK [=] { DEFAULT | NONE | SHARED | EXCLUSIVE } ]
```

Requires table name. No IF EXISTS (use ALTER TABLE instead).

### 5.4 Oracle 23c

```sql
DROP INDEX [ IF EXISTS ] [ schema. ] index_name
  [ ONLINE ] [ FORCE ]
  [ { DEFERRED | IMMEDIATE } INVALIDATION ]
```

### 5.5 SQL Server 2022

```sql
DROP INDEX [ IF EXISTS ] index_name ON { database.schema.table | schema.table | table }
  [ WITH ( MAXDOP = n | ONLINE = { ON | OFF } | MOVE TO ... ) ]
```

Requires table name.

---

## 6. Cross-Database Comparison

### CREATE TABLE

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| IF NOT EXISTS | Yes | Yes | Yes | Yes | No |
| Temporary tables | TEMP keyword | TEMP keyword | TEMPORARY keyword | GLOBAL/PRIVATE TEMP | #/## prefix |
| Generated columns | STORED only | VIRTUAL/STORED | VIRTUAL/STORED | Virtual + STORED | AS expr [PERSISTED] |
| Identity/Auto-inc | GENERATED AS IDENTITY | INTEGER PK AUTOINCREMENT | AUTO_INCREMENT | GENERATED AS IDENTITY | IDENTITY(seed,inc) |
| LIKE/copy structure | LIKE (with options) | No | LIKE (basic) | No | SELECT INTO (no constraints) |
| INHERITS | Yes | No | No | No | No |
| Partitioning | RANGE/LIST/HASH | No | RANGE/LIST/HASH/KEY | RANGE/LIST/HASH/REF/INTERVAL + composite | Via partition schemes |
| EXCLUSION constraint | Yes | No | No | No | No |
| Deferrable constraints | Yes | FK only | No | Yes | No |
| CHECK enforcement | Always | ON CONFLICT | [NOT] ENFORCED | ENABLE/DISABLE | NOT FOR REPLICATION |
| ON UPDATE (FK) | Yes | Yes | Yes | **No** | Yes |
| UNLOGGED | Yes | No | No | NOLOGGING | No |
| Table access method | USING method | No | ENGINE = | ORGANIZATION | Filegroups |

### ALTER TABLE

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| ADD COLUMN | Yes | Yes (limited) | Yes (FIRST/AFTER) | Yes | Yes |
| DROP COLUMN | Yes | Yes (limited) | Yes | Yes + SET UNUSED | Yes |
| Change column type | SET DATA TYPE + USING | **No** | MODIFY/CHANGE | MODIFY | ALTER COLUMN |
| Set/Drop DEFAULT | Yes | **No** | ALTER COLUMN | MODIFY | ADD/DROP constraint |
| Set/Drop NOT NULL | Yes | **No** | Via MODIFY | MODIFY | ALTER COLUMN |
| RENAME COLUMN | Yes | Yes | Yes | Yes | sp_rename only |
| RENAME TABLE | Yes | Yes | Yes | Yes | sp_rename only |
| ADD CONSTRAINT | Yes | **No** | Yes | Yes | Yes |
| DROP CONSTRAINT | Yes | **No** | Yes | Yes | Yes |
| VALIDATE CONSTRAINT | Yes | No | No | ENABLE VALIDATE | WITH CHECK |
| IF EXISTS on actions | Yes | No | No | No | DROP only |
| Column positioning | No | No | FIRST/AFTER | No | No |
| Online DDL | Lock-level | N/A | ALGORITHM/LOCK | MOVE ONLINE | ONLINE option |
| CONCURRENTLY | DETACH PARTITION | No | No | No | No |

### CREATE INDEX

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| IF NOT EXISTS | Yes | Yes | No | Yes | No |
| UNIQUE | Yes | Yes | Yes | Yes | Yes |
| Partial (WHERE) | Yes | Yes | No | No | Yes (filtered) |
| Expression indexes | Yes | Yes | Yes | Yes (function-based) | Via computed columns |
| INCLUDE (covering) | Yes | No | No | No | Yes |
| CONCURRENTLY | Yes | No | No | No | No |
| ONLINE | No | No | ALGORITHM/LOCK | Yes | Yes |
| NULLS FIRST/LAST | Yes | No | No | No | No |
| Index types | btree,hash,gin,gist,spgist,brin | btree only | btree,hash | btree,bitmap,reverse | clustered,nonclustered |
| FULLTEXT | Via GIN | FTS5 extension | Native | Domain index | Separate statement |
| SPATIAL | Via GiST | R*Tree | Native | Domain index | Separate statement |
| INVISIBLE | No | No | Yes | Yes | No |
| Compression | No | No | No | Yes | Yes |
| RESUMABLE | No | No | No | No | Yes |
| Operator classes | Yes | No | No | No | No |

### DROP INDEX

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| IF EXISTS | Yes | Yes | No | Yes | Yes |
| CONCURRENTLY | Yes | No | No | No | No |
| ONLINE | No | No | ALGORITHM/LOCK | Yes | Yes (clustered) |
| CASCADE | Yes | No | No | No | No |
| Requires table name | No | No | Yes | No | Yes |
| Multiple indexes | Yes | No | No | No | Yes |
