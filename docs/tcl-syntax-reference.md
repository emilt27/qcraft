# TCL Syntax Reference (All Dialects)

Full syntax for latest versions: PostgreSQL 17, SQLite 3.45+, MySQL 8.4, Oracle 23c, SQL Server 2022.

---

## 1. BEGIN / START TRANSACTION

### 1.1 PostgreSQL 17

```sql
BEGIN [ WORK | TRANSACTION ] [ transaction_mode [, ...] ]

START TRANSACTION [ transaction_mode [, ...] ]

-- transaction_mode:
ISOLATION LEVEL { SERIALIZABLE | REPEATABLE READ | READ COMMITTED | READ UNCOMMITTED }
READ WRITE | READ ONLY
[ NOT ] DEFERRABLE
```

- `BEGIN` and `START TRANSACTION` are equivalent.
- `WORK` and `TRANSACTION` are noise words, optional.
- `READ UNCOMMITTED` maps to `READ COMMITTED` in practice.
- `DEFERRABLE` only meaningful with `SERIALIZABLE READ ONLY` — defers until safe snapshot, avoids serialization failures.

### 1.2 SQLite 3.45+

```sql
BEGIN [ DEFERRED | IMMEDIATE | EXCLUSIVE ] [ TRANSACTION ]
```

- `DEFERRED` (default): no lock until first read/write.
- `IMMEDIATE`: acquires RESERVED lock immediately (allows other reads, blocks other writes).
- `EXCLUSIVE`: acquires EXCLUSIVE lock immediately (blocks all other connections).
- No isolation level syntax — SQLite is always SERIALIZABLE.
- WAL mode allows concurrent readers with a single writer.

### 1.3 MySQL 8.4

```sql
START TRANSACTION
    [ WITH CONSISTENT SNAPSHOT ]
    [, READ WRITE | READ ONLY ]

BEGIN [ WORK ]
```

- `START TRANSACTION` is preferred; `BEGIN` also works but doesn't support modifiers.
- `WITH CONSISTENT SNAPSHOT` starts a consistent read with InnoDB.
- Isolation level is set separately via `SET TRANSACTION`.

### 1.4 Oracle 23c

```sql
SET TRANSACTION
    { READ ONLY | READ WRITE }
    [ ISOLATION LEVEL { SERIALIZABLE | READ COMMITTED } ]
    [ USE ROLLBACK SEGMENT rollback_segment ]
    [ NAME 'transaction_name' ]
```

- Oracle has **no explicit BEGIN** — a transaction starts implicitly with the first DML.
- `SET TRANSACTION` must be the first statement in a transaction.
- Only `READ COMMITTED` (default) and `SERIALIZABLE` isolation levels.
- `NAME` assigns a label to the transaction (for monitoring/diagnostics).
- `USE ROLLBACK SEGMENT` is a legacy hint for undo segment placement.

### 1.5 SQL Server 2022

```sql
BEGIN { TRAN | TRANSACTION }
    [ { transaction_name | @tran_name_variable }
      [ WITH MARK [ 'description' ] ]
    ]
```

- `TRAN` and `TRANSACTION` are interchangeable.
- Named transactions: `BEGIN TRAN my_tx`.
- `WITH MARK` writes to the transaction log for point-in-time recovery.
- Isolation level is set separately via `SET TRANSACTION ISOLATION LEVEL`.

---

## 2. COMMIT

### 2.1 PostgreSQL 17

```sql
COMMIT [ WORK | TRANSACTION ] [ AND [ NO ] CHAIN ]
END [ WORK | TRANSACTION ] [ AND [ NO ] CHAIN ]
```

- `COMMIT` and `END` are equivalent.
- `AND CHAIN` immediately starts a new transaction with same isolation/access mode.
- `AND NO CHAIN` is the default.

### 2.2 SQLite 3.45+

```sql
COMMIT [ TRANSACTION ]
END [ TRANSACTION ]
```

- `COMMIT` and `END` are equivalent.
- No `AND CHAIN` support.

### 2.3 MySQL 8.4

```sql
COMMIT [ WORK ] [ AND [ NO ] CHAIN ] [ [ NO ] RELEASE ]
```

- `AND CHAIN` starts a new transaction immediately.
- `RELEASE` disconnects the session after commit.
- `NO RELEASE` is the default.

### 2.4 Oracle 23c

```sql
COMMIT [ WORK ]
    [ COMMENT 'comment_text' ]
    [ WRITE [ WAIT | NOWAIT ] [ IMMEDIATE | BATCH ] ]
    [ FORCE 'transaction_id' [, system_change_number] ]
```

- `COMMENT` associates a comment with the transaction in data dictionary.
- `WRITE WAIT IMMEDIATE` (default) — synchronous, immediately flushes redo.
- `WRITE NOWAIT BATCH` — asynchronous, batches redo (for non-critical data, better performance).
- `FORCE` manually commits an in-doubt distributed transaction by transaction ID.

### 2.5 SQL Server 2022

```sql
COMMIT { TRAN | TRANSACTION }
    [ transaction_name | @tran_name_variable ]
```

- Transaction name is optional; if provided, must match the `BEGIN TRAN` name (informational only).
- Nested transactions: only the outermost `COMMIT` actually commits.

---

## 3. ROLLBACK

### 3.1 PostgreSQL 17

```sql
ROLLBACK [ WORK | TRANSACTION ] [ AND [ NO ] CHAIN ]
ABORT [ WORK | TRANSACTION ] [ AND [ NO ] CHAIN ]
```

- `ROLLBACK` and `ABORT` are equivalent.
- `AND CHAIN` starts a new transaction immediately after rollback.

### 3.2 SQLite 3.45+

```sql
ROLLBACK [ TRANSACTION ] [ TO [ SAVEPOINT ] savepoint_name ]
```

- Without `TO`: rolls back entire transaction.
- With `TO SAVEPOINT`: rolls back to the named savepoint (does NOT release it).

### 3.3 MySQL 8.4

```sql
ROLLBACK [ WORK ] [ AND [ NO ] CHAIN ] [ [ NO ] RELEASE ]
```

- Same modifiers as COMMIT (`AND CHAIN`, `RELEASE`).
- For savepoint: `ROLLBACK TO [ SAVEPOINT ] savepoint_name` (separate syntax).

### 3.4 Oracle 23c

```sql
ROLLBACK [ WORK ]
    [ TO [ SAVEPOINT ] savepoint_name ]
    [ FORCE 'transaction_id' ]
```

- `FORCE` manually rolls back an in-doubt distributed transaction.
- `TO SAVEPOINT` rolls back to the named savepoint.

### 3.5 SQL Server 2022

```sql
ROLLBACK { TRAN | TRANSACTION }
    [ transaction_name | @tran_name_variable | savepoint_name | @savepoint_variable ]
```

- Can rollback to a savepoint name created with `SAVE TRAN`.
- Named rollback to a transaction name rolls back the entire transaction.

---

## 4. SAVEPOINT

### 4.1 PostgreSQL 17

```sql
SAVEPOINT savepoint_name
```

- Reusing a name destroys the old savepoint (replaced by the new one).

### 4.2 SQLite 3.45+

```sql
SAVEPOINT savepoint_name
```

- SQLite savepoints can also act as transactions (can be used without a prior BEGIN).
- Savepoints are nested — creating a savepoint within a savepoint creates a nested scope.

### 4.3 MySQL 8.4

```sql
SAVEPOINT savepoint_name
```

- Standard syntax, no special modifiers.

### 4.4 Oracle 23c

```sql
SAVEPOINT savepoint_name
```

- Standard syntax, no special modifiers.

### 4.5 SQL Server 2022

```sql
SAVE { TRAN | TRANSACTION } { savepoint_name | @savepoint_variable }
```

- Different keyword: `SAVE TRAN` instead of `SAVEPOINT`.
- Savepoint name is required (not optional).

---

## 5. RELEASE SAVEPOINT

### 5.1 PostgreSQL 17

```sql
RELEASE [ SAVEPOINT ] savepoint_name
```

- Destroys the savepoint and all savepoints created after it.
- Does NOT commit — work is still part of the surrounding transaction.

### 5.2 SQLite 3.45+

```sql
RELEASE [ SAVEPOINT ] savepoint_name
```

- If the savepoint is the outermost, it acts as a COMMIT for the entire transaction.
- Otherwise, merges the savepoint's work into the parent savepoint.

### 5.3 MySQL 8.4

```sql
RELEASE SAVEPOINT savepoint_name
```

- Removes the savepoint. No commit effect.

### 5.4 Oracle 23c

- **Not supported.** Oracle does not have `RELEASE SAVEPOINT`.
- Savepoints are automatically released on commit/rollback.

### 5.5 SQL Server 2022

- **Not supported.** SQL Server does not have `RELEASE SAVEPOINT`.
- Savepoints remain until the transaction completes.

---

## 6. ROLLBACK TO SAVEPOINT

### 6.1 PostgreSQL 17

```sql
ROLLBACK [ WORK | TRANSACTION ] TO [ SAVEPOINT ] savepoint_name
```

- Rolls back all commands after the savepoint.
- The savepoint remains valid and can be rolled back to again.
- All savepoints created after the target are destroyed.

### 6.2 SQLite 3.45+

```sql
ROLLBACK [ TRANSACTION ] TO [ SAVEPOINT ] savepoint_name
```

- Does NOT release the savepoint (unlike RELEASE after ROLLBACK TO in PG).

### 6.3 MySQL 8.4

```sql
ROLLBACK [ WORK ] TO [ SAVEPOINT ] savepoint_name
```

- Savepoints created after the target are removed.

### 6.4 Oracle 23c

```sql
ROLLBACK [ WORK ] TO [ SAVEPOINT ] savepoint_name
```

- Standard behavior. Savepoint remains valid.

### 6.5 SQL Server 2022

```sql
ROLLBACK { TRAN | TRANSACTION } savepoint_name
```

- Uses `ROLLBACK TRAN savepoint_name` — no separate `TO SAVEPOINT` syntax.

---

## 7. SET TRANSACTION (Isolation Level)

### 7.1 PostgreSQL 17

```sql
SET TRANSACTION transaction_mode [, ...]
SET TRANSACTION SNAPSHOT snapshot_id

SET SESSION CHARACTERISTICS AS TRANSACTION transaction_mode [, ...]

-- transaction_mode:
ISOLATION LEVEL { SERIALIZABLE | REPEATABLE READ | READ COMMITTED | READ UNCOMMITTED }
READ WRITE | READ ONLY
[ NOT ] DEFERRABLE
```

- `SET TRANSACTION` affects the current transaction only.
- `SET SESSION CHARACTERISTICS AS TRANSACTION` sets defaults for all future transactions.
- `SET TRANSACTION SNAPSHOT` imports a snapshot from another session (for parallel consistent reads).

### 7.2 SQLite 3.45+

- No `SET TRANSACTION` syntax.
- Isolation is controlled via locking mode at `BEGIN` (`DEFERRED`/`IMMEDIATE`/`EXCLUSIVE`).
- `PRAGMA journal_mode` and `PRAGMA locking_mode` control behavior globally.

### 7.3 MySQL 8.4

```sql
SET [ GLOBAL | SESSION ] TRANSACTION
    { ISOLATION LEVEL level | access_mode } [, ...]

-- level:
{ REPEATABLE READ | READ COMMITTED | READ UNCOMMITTED | SERIALIZABLE }

-- access_mode:
{ READ WRITE | READ ONLY }
```

- Without `GLOBAL`/`SESSION`: applies to the next transaction only.
- `SESSION`: applies to all transactions in the current session.
- `GLOBAL`: applies to all future sessions (requires `SUPER` privilege).

### 7.4 Oracle 23c

```sql
SET TRANSACTION
    { ISOLATION LEVEL { SERIALIZABLE | READ COMMITTED } }
    { READ ONLY | READ WRITE }
    [ NAME 'transaction_name' ]
```

- Must be the first statement in a transaction.
- Only `READ COMMITTED` and `SERIALIZABLE`.

### 7.5 SQL Server 2022

```sql
SET TRANSACTION ISOLATION LEVEL
    { READ UNCOMMITTED | READ COMMITTED | REPEATABLE READ | SNAPSHOT | SERIALIZABLE }
```

- Applies to all statements in the current session until changed.
- `SNAPSHOT` isolation is unique to SQL Server — uses row versioning.
- `READ COMMITTED` has two variants depending on `READ_COMMITTED_SNAPSHOT` database option.

---

## 8. LOCK TABLE

### 8.1 PostgreSQL 17

```sql
LOCK [ TABLE ] [ ONLY ] name [ * ] [, ...] [ IN lockmode MODE ] [ NOWAIT ]

-- lockmode:
ACCESS SHARE | ROW SHARE | ROW EXCLUSIVE | SHARE UPDATE EXCLUSIVE
| SHARE | SHARE ROW EXCLUSIVE | EXCLUSIVE | ACCESS EXCLUSIVE
```

- 8 lock modes with a well-defined conflict matrix.
- `ONLY` prevents locking inherited/child tables.
- `NOWAIT` returns error immediately if lock cannot be acquired (no waiting).
- Locks are released at transaction end only.

### 8.2 SQLite 3.45+

- **No explicit LOCK TABLE.**
- Locking is controlled via `BEGIN IMMEDIATE`/`BEGIN EXCLUSIVE`.
- `PRAGMA locking_mode = EXCLUSIVE` locks at connection level.

### 8.3 MySQL 8.4

```sql
LOCK TABLES
    tbl_name [ [AS] alias ] { READ [LOCAL] | [LOW_PRIORITY] WRITE }
    [, tbl_name [ [AS] alias ] { READ [LOCAL] | [LOW_PRIORITY] WRITE } ] ...

UNLOCK TABLES
```

- `READ`: shared lock (other sessions can read, none can write).
- `READ LOCAL`: allows concurrent inserts by other sessions.
- `WRITE`: exclusive lock (no other sessions can read or write).
- `LOW_PRIORITY WRITE`: deprecated, waits for all read locks first.
- Must lock all tables you plan to use in the session.
- `UNLOCK TABLES` explicitly releases all locks.

### 8.4 Oracle 23c

```sql
LOCK TABLE [ schema. ] { table | view } [, ...] [ partition_clause ]
    IN lockmode MODE
    [ WAIT integer | NOWAIT ]

-- lockmode:
ROW SHARE | ROW EXCLUSIVE | SHARE | SHARE ROW EXCLUSIVE | EXCLUSIVE
-- aliases:
SHARE UPDATE = ROW SHARE
```

- `WAIT integer`: wait up to N seconds for the lock.
- `NOWAIT`: return error immediately if lock unavailable.
- Can lock individual partitions/subpartitions.

### 8.5 SQL Server 2022

- **No explicit LOCK TABLE statement.**
- Locking is controlled via table hints in queries:

```sql
SELECT * FROM table_name WITH (TABLOCK, HOLDLOCK)
UPDATE table_name WITH (TABLOCKX) SET ...
```

- Common hints: `NOLOCK`, `ROWLOCK`, `PAGELOCK`, `TABLOCK`, `TABLOCKX`, `HOLDLOCK`, `UPDLOCK`, `XLOCK`.
- `sp_lock` and DMVs for monitoring locks.

---

## 9. Two-Phase Commit (2PC) / Distributed Transactions

### 9.1 PostgreSQL 17

```sql
PREPARE TRANSACTION 'transaction_id'

COMMIT PREPARED 'transaction_id'

ROLLBACK PREPARED 'transaction_id'
```

- **PREPARE TRANSACTION**: prepares the current transaction for 2PC. After this, the transaction is dissociated from the session and stored on disk.
- The prepared transaction persists across server restarts and crashes.
- `transaction_id` is a string literal (max 200 bytes), must be globally unique.
- After `PREPARE TRANSACTION`, the session cannot use `COMMIT`/`ROLLBACK` — must use `COMMIT PREPARED` or `ROLLBACK PREPARED`.
- Requires `max_prepared_transactions > 0` in postgresql.conf (default is 0 = disabled).
- `pg_prepared_xacts` system view lists all currently prepared transactions.
- Primarily used by transaction managers (e.g., in XA protocol, microservices coordination).

### 9.2 SQLite 3.45+

- **Not supported.** SQLite is an embedded database with no networked multi-server transactions.

### 9.3 MySQL 8.4

```sql
XA START 'xid' [ JOIN | RESUME ]
XA END 'xid' [ SUSPEND [ FOR MIGRATE ] ]
XA PREPARE 'xid'
XA COMMIT 'xid' [ ONE PHASE ]
XA ROLLBACK 'xid'
XA RECOVER [ CONVERT XID ]

-- xid format:
'gtrid' [, 'bqual' [, formatID]]
```

- MySQL implements the X/Open XA specification.
- `XA START` begins a distributed transaction branch.
- `XA END` marks the end of work for a branch.
- `XA PREPARE` prepares the branch.
- `XA COMMIT ONE PHASE` for single-branch optimization.
- `XA RECOVER` lists prepared transactions.
- `xid` has three parts: global transaction ID, branch qualifier, format ID.

### 9.4 Oracle 23c

- Oracle uses **DBMS_XA** PL/SQL package for XA protocol:

```sql
-- Not direct SQL, but PL/SQL API:
DBMS_XA.XA_START(xid, ...)
DBMS_XA.XA_END(xid, ...)
DBMS_XA.XA_PREPARE(xid)
DBMS_XA.XA_COMMIT(xid, ...)
DBMS_XA.XA_ROLLBACK(xid)
DBMS_XA.XA_RECOVER(...)
```

- Also supports **database links** with implicit 2PC:

```sql
COMMIT FORCE 'transaction_id' [, system_change_number]
ROLLBACK FORCE 'transaction_id'
```

- `COMMIT FORCE` / `ROLLBACK FORCE` resolve in-doubt distributed transactions.
- `DBA_2PC_PENDING` view shows in-doubt transactions.

### 9.5 SQL Server 2022

- SQL Server uses **MS DTC** (Microsoft Distributed Transaction Coordinator):

```sql
BEGIN DISTRIBUTED TRANSACTION [ transaction_name | @tran_name_variable ]
```

- `BEGIN DISTRIBUTED TRAN` enlists the transaction in MS DTC.
- Individual resource managers are coordinated by DTC (not SQL-level prepare/commit).
- Can also participate in XA transactions through the MSDTC XA interface.
- No SQL-level `PREPARE` / `COMMIT PREPARED` syntax.

---

## Comparison Table

| Feature | PostgreSQL | SQLite | MySQL | Oracle | SQL Server |
|---|---|---|---|---|---|
| **BEGIN** | `BEGIN` / `START TRANSACTION` | `BEGIN [DEFERRED\|IMMEDIATE\|EXCLUSIVE]` | `START TRANSACTION` / `BEGIN` | Implicit (no BEGIN) | `BEGIN TRAN [name]` |
| **COMMIT** | `COMMIT [AND CHAIN]` | `COMMIT` / `END` | `COMMIT [AND CHAIN] [RELEASE]` | `COMMIT [WRITE opts] [FORCE]` | `COMMIT TRAN [name]` |
| **ROLLBACK** | `ROLLBACK [AND CHAIN]` | `ROLLBACK` | `ROLLBACK [AND CHAIN] [RELEASE]` | `ROLLBACK [FORCE]` | `ROLLBACK TRAN [name]` |
| **SAVEPOINT** | `SAVEPOINT name` | `SAVEPOINT name` | `SAVEPOINT name` | `SAVEPOINT name` | `SAVE TRAN name` |
| **RELEASE SAVEPOINT** | `RELEASE [SAVEPOINT] name` | `RELEASE [SAVEPOINT] name` | `RELEASE SAVEPOINT name` | Not supported | Not supported |
| **ROLLBACK TO** | `ROLLBACK TO [SAVEPOINT] name` | `ROLLBACK TO [SAVEPOINT] name` | `ROLLBACK TO [SAVEPOINT] name` | `ROLLBACK TO [SAVEPOINT] name` | `ROLLBACK TRAN name` |
| **Isolation Levels** | RC, RR, S, RU + DEFERRABLE | Implicit SERIALIZABLE | RU, RC, RR, S | RC, S | RU, RC, RR, S, SNAPSHOT |
| **SET TRANSACTION** | Per-txn + per-session | N/A (PRAGMA) | Per-txn / session / global | Must be first stmt | Per-session |
| **LOCK TABLE** | 8 lock modes + NOWAIT | N/A (BEGIN EXCLUSIVE) | READ / WRITE + UNLOCK | 5 lock modes + WAIT N | Table hints only |
| **2PC** | `PREPARE TRANSACTION` | Not supported | XA protocol | DBMS_XA / COMMIT FORCE | MS DTC / BEGIN DISTRIBUTED |
| **AND CHAIN** | Yes | No | Yes | No | No |

**Isolation level abbreviations:** RU = READ UNCOMMITTED, RC = READ COMMITTED, RR = REPEATABLE READ, S = SERIALIZABLE.

---

## Key Notes

1. **Oracle has no explicit BEGIN** — transactions start implicitly with the first DML statement.
2. **SQL Server uses SAVE TRAN** instead of standard `SAVEPOINT`.
3. **RELEASE SAVEPOINT** is only supported by PostgreSQL, SQLite, and MySQL.
4. **PostgreSQL's PREPARE TRANSACTION** is the only database with native SQL-level 2PC. Others use external protocols (XA, MS DTC) or PL/SQL APIs.
5. **SQLite's SAVEPOINT** can act as a transaction boundary when used outside a BEGIN block.
6. **AND CHAIN** (start new transaction after commit/rollback) is only in PostgreSQL and MySQL.
7. **DEFERRABLE** is PostgreSQL-specific, only meaningful with SERIALIZABLE READ ONLY.
8. **SNAPSHOT isolation** is SQL Server-specific (row versioning, no locks for reads).
9. **Oracle's COMMIT WRITE NOWAIT BATCH** is unique for async commit optimization.
10. **MySQL's COMMIT RELEASE** disconnects the session after commit — unique feature.
