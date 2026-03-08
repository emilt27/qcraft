use qcraft_core::ast::tcl::*;
use qcraft_core::error::RenderError;
use qcraft_sqlite::SqliteRenderer;

fn render(stmt: TransactionStmt) -> String {
    let renderer = SqliteRenderer::new();
    let (sql, _params) = renderer.render_transaction_stmt(&stmt).unwrap();
    sql
}

fn render_err(stmt: TransactionStmt) -> RenderError {
    let renderer = SqliteRenderer::new();
    renderer.render_transaction_stmt(&stmt).unwrap_err()
}

// ---------------------------------------------------------------------------
// BEGIN
// ---------------------------------------------------------------------------

#[test]
fn begin_simple() {
    let sql = render(TransactionStmt::Begin(BeginStmt {
        modes: None,
        lock_type: None,
        name: None,
        with_mark: None,
    }));
    assert_eq!(sql, "BEGIN TRANSACTION");
}

#[test]
fn begin_deferred() {
    let sql = render(TransactionStmt::Begin(BeginStmt {
        modes: None,
        lock_type: Some(SqliteLockType::Deferred),
        name: None,
        with_mark: None,
    }));
    assert_eq!(sql, "BEGIN DEFERRED TRANSACTION");
}

#[test]
fn begin_immediate() {
    let sql = render(TransactionStmt::Begin(BeginStmt {
        modes: None,
        lock_type: Some(SqliteLockType::Immediate),
        name: None,
        with_mark: None,
    }));
    assert_eq!(sql, "BEGIN IMMEDIATE TRANSACTION");
}

#[test]
fn begin_exclusive() {
    let sql = render(TransactionStmt::Begin(BeginStmt {
        modes: None,
        lock_type: Some(SqliteLockType::Exclusive),
        name: None,
        with_mark: None,
    }));
    assert_eq!(sql, "BEGIN EXCLUSIVE TRANSACTION");
}

// ---------------------------------------------------------------------------
// COMMIT
// ---------------------------------------------------------------------------

#[test]
fn commit() {
    let sql = render(TransactionStmt::Commit(CommitStmt {
        and_chain: false,
        release: false,
        name: None,
        comment: None,
        write_mode: None,
        force: None,
    }));
    assert_eq!(sql, "COMMIT");
}

// ---------------------------------------------------------------------------
// ROLLBACK
// ---------------------------------------------------------------------------

#[test]
fn rollback_simple() {
    let sql = render(TransactionStmt::Rollback(RollbackStmt {
        to_savepoint: None,
        and_chain: false,
        release: false,
        name: None,
        force: None,
    }));
    assert_eq!(sql, "ROLLBACK");
}

#[test]
fn rollback_to_savepoint() {
    let sql = render(TransactionStmt::Rollback(RollbackStmt {
        to_savepoint: Some("sp1".into()),
        and_chain: false,
        release: false,
        name: None,
        force: None,
    }));
    assert_eq!(sql, "ROLLBACK TO SAVEPOINT \"sp1\"");
}

// ---------------------------------------------------------------------------
// SAVEPOINT
// ---------------------------------------------------------------------------

#[test]
fn savepoint() {
    let sql = render(TransactionStmt::Savepoint(SavepointStmt {
        name: "my_sp".into(),
    }));
    assert_eq!(sql, "SAVEPOINT \"my_sp\"");
}

// ---------------------------------------------------------------------------
// RELEASE SAVEPOINT
// ---------------------------------------------------------------------------

#[test]
fn release_savepoint() {
    let sql = render(TransactionStmt::ReleaseSavepoint(ReleaseSavepointStmt {
        name: "my_sp".into(),
    }));
    assert_eq!(sql, "RELEASE SAVEPOINT \"my_sp\"");
}

// ---------------------------------------------------------------------------
// Unsupported features
// ---------------------------------------------------------------------------

#[test]
fn set_transaction_unsupported() {
    let err = render_err(TransactionStmt::SetTransaction(SetTransactionStmt {
        modes: vec![TransactionMode::IsolationLevel(
            IsolationLevel::Serializable,
        )],
        scope: None,
        snapshot_id: None,
        name: None,
    }));
    assert!(err.to_string().contains("SET TRANSACTION"));
}

#[test]
fn lock_table_unsupported() {
    let err = render_err(TransactionStmt::LockTable(LockTableStmt {
        tables: vec![LockTableDef {
            table: "t".into(),
            schema: None,
            mode: LockMode::Exclusive,
            only: false,
            alias: None,
            wait: None,
            partition: None,
        }],
        nowait: false,
    }));
    assert!(err.to_string().contains("LOCK TABLE"));
}

#[test]
fn prepare_transaction_unsupported() {
    let err = render_err(TransactionStmt::PrepareTransaction(
        PrepareTransactionStmt {
            transaction_id: "foo".into(),
        },
    ));
    assert!(err.to_string().contains("PREPARE TRANSACTION"));
}

#[test]
fn commit_prepared_unsupported() {
    let err = render_err(TransactionStmt::CommitPrepared(CommitPreparedStmt {
        transaction_id: "foo".into(),
    }));
    assert!(err.to_string().contains("COMMIT PREPARED"));
}

#[test]
fn rollback_prepared_unsupported() {
    let err = render_err(TransactionStmt::RollbackPrepared(RollbackPreparedStmt {
        transaction_id: "foo".into(),
    }));
    assert!(err.to_string().contains("ROLLBACK PREPARED"));
}
