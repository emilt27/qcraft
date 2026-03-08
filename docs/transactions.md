# Transaction Control (TCL)

rquery models transaction control statements via the `TransactionStmt` enum. All TCL statements are rendered as **inline literals** — they are never parameterized. Values like savepoint names and transaction IDs are emitted directly into the SQL string.

## BEGIN

Start a new transaction with `TransactionStmt::begin()`:

```rust
let stmt = TransactionStmt::begin();
// BEGIN
```

### With isolation level

```rust
let stmt = TransactionStmt::Begin(BeginStmt::with_isolation(IsolationLevel::Serializable));
// BEGIN ISOLATION LEVEL SERIALIZABLE
```

Available isolation levels: `ReadUncommitted`, `ReadCommitted`, `RepeatableRead`, `Serializable`, `Snapshot` (SQL Server only).

### Read-only transaction

```rust
let stmt = TransactionStmt::Begin(BeginStmt::read_only());
// BEGIN READ ONLY
```

### Multiple transaction modes

The `BeginStmt::modes` field accepts a `Vec<TransactionMode>`, so you can combine modes:

```rust
let stmt = TransactionStmt::Begin(BeginStmt {
    modes: Some(vec![
        TransactionMode::IsolationLevel(IsolationLevel::Serializable),
        TransactionMode::ReadOnly,
        TransactionMode::Deferrable,
    ]),
    ..Default::default()
});
// BEGIN ISOLATION LEVEL SERIALIZABLE READ ONLY DEFERRABLE
```

### SQLite lock types

SQLite supports `DEFERRED`, `IMMEDIATE`, and `EXCLUSIVE` transaction types:

```rust
let stmt = TransactionStmt::Begin(BeginStmt::sqlite_deferred());
// BEGIN DEFERRED

let stmt = TransactionStmt::Begin(BeginStmt::sqlite_immediate());
// BEGIN IMMEDIATE

let stmt = TransactionStmt::Begin(BeginStmt::sqlite_exclusive());
// BEGIN EXCLUSIVE
```

These are set via `BeginStmt::lock_type` and rendered only by `SqliteRenderer`. The PostgreSQL renderer ignores the `lock_type` field.

## COMMIT

```rust
let stmt = TransactionStmt::commit();
// COMMIT
```

### AND CHAIN (PostgreSQL)

Start a new transaction immediately after committing:

```rust
let stmt = TransactionStmt::Commit(CommitStmt {
    and_chain: true,
    ..Default::default()
});
// COMMIT AND CHAIN
```

## ROLLBACK

```rust
let stmt = TransactionStmt::rollback();
// ROLLBACK
```

### Rollback to savepoint

```rust
let stmt = TransactionStmt::rollback_to("sp1");
// ROLLBACK TO SAVEPOINT sp1
```

### AND CHAIN

```rust
let stmt = TransactionStmt::Rollback(RollbackStmt {
    and_chain: true,
    ..Default::default()
});
// ROLLBACK AND CHAIN
```

## SAVEPOINT

Create, release, and roll back to savepoints:

```rust
let stmt = TransactionStmt::savepoint("sp1");
// SAVEPOINT sp1

let stmt = TransactionStmt::release("sp1");
// RELEASE SAVEPOINT sp1

let stmt = TransactionStmt::rollback_to("sp1");
// ROLLBACK TO SAVEPOINT sp1
```

## SET TRANSACTION (PostgreSQL)

Set transaction characteristics for the current transaction or session:

```rust
let stmt = TransactionStmt::SetTransaction(SetTransactionStmt {
    modes: vec![
        TransactionMode::IsolationLevel(IsolationLevel::RepeatableRead),
        TransactionMode::ReadOnly,
    ],
    scope: None,
    snapshot_id: None,
    name: None,
});
// SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY
```

Session-level scope:

```rust
let stmt = TransactionStmt::SetTransaction(SetTransactionStmt {
    modes: vec![TransactionMode::IsolationLevel(IsolationLevel::ReadCommitted)],
    scope: Some(TransactionScope::Session),
    snapshot_id: None,
    name: None,
});
// SET SESSION CHARACTERISTICS AS TRANSACTION ISOLATION LEVEL READ COMMITTED
```

PostgreSQL snapshot import:

```rust
let stmt = TransactionStmt::SetTransaction(SetTransactionStmt {
    modes: vec![],
    scope: None,
    snapshot_id: Some("00000003-0000001A-1".to_string()),
    name: None,
});
// SET TRANSACTION SNAPSHOT '00000003-0000001A-1'
```

## LOCK TABLE (PostgreSQL)

```rust
let stmt = TransactionStmt::LockTable(LockTableStmt {
    tables: vec![LockTableDef {
        table: "users".to_string(),
        schema: None,
        mode: LockMode::RowExclusive,
        only: false,
        alias: None,
        wait: None,
        partition: None,
    }],
    nowait: true,
});
// LOCK TABLE "users" IN ROW EXCLUSIVE MODE NOWAIT
```

Available PostgreSQL lock modes: `AccessShare`, `RowShare`, `RowExclusive`, `ShareUpdateExclusive`, `Share`, `ShareRowExclusive`, `Exclusive`, `AccessExclusive`.

## Two-Phase Commit (PostgreSQL)

```rust
let stmt = TransactionStmt::PrepareTransaction(PrepareTransactionStmt {
    transaction_id: "my_txn".to_string(),
});
// PREPARE TRANSACTION 'my_txn'

let stmt = TransactionStmt::CommitPrepared(CommitPreparedStmt {
    transaction_id: "my_txn".to_string(),
});
// COMMIT PREPARED 'my_txn'

let stmt = TransactionStmt::RollbackPrepared(RollbackPreparedStmt {
    transaction_id: "my_txn".to_string(),
});
// ROLLBACK PREPARED 'my_txn'
```

## Rendering

TCL statements are rendered via the `render_transaction` method on the `Renderer` trait:

```rust
let renderer = PostgresRenderer::new();
let stmt = TransactionStmt::begin();
let (sql, params) = renderer.render_transaction_stmt(&stmt)?;
// sql = "BEGIN", params = [] (always empty for TCL)
```

TCL always uses inline literals, never parameter placeholders. The `RenderCtx` is created with `parameterize: false` for all transaction statements.

## Custom Transactions

For vendor-specific TCL not covered by the built-in types, use `TransactionStmt::Custom` with a type implementing the `CustomTransaction` trait. See the [extensibility guide](extensibility.md) for details.
