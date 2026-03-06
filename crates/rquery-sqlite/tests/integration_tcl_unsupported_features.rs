//! Tests that verify SQLite renderer correctly rejects unsupported TCL features
//! with meaningful errors (not silent failures).

use rquery_core::ast::tcl::*;
use rquery_sqlite::SqliteRenderer;

fn render_err(stmt: &TransactionStmt) -> String {
    let renderer = SqliteRenderer::new();
    renderer.render_transaction_stmt(stmt).unwrap_err().to_string()
}

// ==========================================================================
// Unsupported TCL statements
// ==========================================================================

#[test]
fn set_transaction_unsupported() {
    let err = render_err(&TransactionStmt::SetTransaction(SetTransactionStmt {
        modes: vec![TransactionMode::IsolationLevel(IsolationLevel::Serializable)],
        scope: None,
        snapshot_id: None,
        name: None,
    }));
    assert!(err.contains("SET TRANSACTION"), "error should mention SET TRANSACTION: {err}");
}

#[test]
fn lock_table_unsupported() {
    let err = render_err(&TransactionStmt::LockTable(LockTableStmt {
        tables: vec![LockTableDef {
            table: "items".to_string(),
            schema: None,
            mode: LockMode::Exclusive,
            only: false,
            alias: None,
            wait: None,
            partition: None,
        }],
        nowait: false,
    }));
    assert!(err.contains("LOCK TABLE"), "error should mention LOCK TABLE: {err}");
}

#[test]
fn prepare_transaction_unsupported() {
    let err = render_err(&TransactionStmt::PrepareTransaction(PrepareTransactionStmt {
        transaction_id: "tx1".to_string(),
    }));
    assert!(
        err.contains("PREPARE TRANSACTION"),
        "error should mention PREPARE TRANSACTION: {err}"
    );
}

#[test]
fn commit_prepared_unsupported() {
    let err = render_err(&TransactionStmt::CommitPrepared(CommitPreparedStmt {
        transaction_id: "tx1".to_string(),
    }));
    assert!(
        err.contains("COMMIT PREPARED"),
        "error should mention COMMIT PREPARED: {err}"
    );
}

#[test]
fn rollback_prepared_unsupported() {
    let err = render_err(&TransactionStmt::RollbackPrepared(RollbackPreparedStmt {
        transaction_id: "tx1".to_string(),
    }));
    assert!(
        err.contains("ROLLBACK PREPARED"),
        "error should mention ROLLBACK PREPARED: {err}"
    );
}
