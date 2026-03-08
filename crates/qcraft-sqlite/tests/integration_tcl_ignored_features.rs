//! Tests that verify SQLite silently ignores unsupported TCL features
//! while still producing valid, executable SQL.

use qcraft_core::ast::tcl::*;
use qcraft_sqlite::SqliteRenderer;
use rusqlite::Connection;

fn conn() -> Connection {
    Connection::open_in_memory().unwrap()
}

fn render(stmt: &TransactionStmt) -> String {
    let renderer = SqliteRenderer::new();
    let (sql, _) = renderer.render_transaction_stmt(stmt).unwrap();
    sql
}

fn setup_table(conn: &Connection) {
    conn.execute(
        "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        [],
    )
    .unwrap();
}

fn count_rows(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))
        .unwrap()
}

/// Helper: begin a transaction, insert a row, then execute the given
/// statement (commit/rollback variant), and return the row count.
fn begin_insert_and_execute(stmt: &TransactionStmt) -> i64 {
    let db = conn();
    setup_table(&db);

    let begin = render(&TransactionStmt::Begin(BeginStmt {
        modes: None,
        lock_type: None,
        name: None,
        with_mark: None,
    }));
    db.execute_batch(&begin).unwrap();

    db.execute("INSERT INTO items (id, name) VALUES (1, 'apple')", [])
        .unwrap();

    let sql = render(stmt);
    db.execute_batch(&sql).unwrap();

    count_rows(&db)
}

// ==========================================================================
// COMMIT — ignored fields
// ==========================================================================

#[test]
fn commit_and_chain_ignored() {
    let rows = begin_insert_and_execute(&TransactionStmt::Commit(CommitStmt {
        and_chain: true,
        release: false,
        name: None,
        comment: None,
        write_mode: None,
        force: None,
    }));
    assert_eq!(rows, 1);
}

#[test]
fn commit_release_ignored() {
    let rows = begin_insert_and_execute(&TransactionStmt::Commit(CommitStmt {
        and_chain: false,
        release: true,
        name: None,
        comment: None,
        write_mode: None,
        force: None,
    }));
    assert_eq!(rows, 1);
}

#[test]
fn commit_name_ignored() {
    let rows = begin_insert_and_execute(&TransactionStmt::Commit(CommitStmt {
        and_chain: false,
        release: false,
        name: Some("tx1".to_string()),
        comment: None,
        write_mode: None,
        force: None,
    }));
    assert_eq!(rows, 1);
}

#[test]
fn commit_all_extras_ignored() {
    let rows = begin_insert_and_execute(&TransactionStmt::Commit(CommitStmt {
        and_chain: true,
        release: true,
        name: Some("tx1".to_string()),
        comment: Some("important commit".to_string()),
        write_mode: Some(OracleWriteMode {
            wait: OracleWriteWait::NoWait,
            flush: OracleWriteFlush::Batch,
        }),
        force: Some("abc123".to_string()),
    }));
    assert_eq!(rows, 1);
}

// ==========================================================================
// ROLLBACK — ignored fields
// ==========================================================================

#[test]
fn rollback_and_chain_ignored() {
    let rows = begin_insert_and_execute(&TransactionStmt::Rollback(RollbackStmt {
        to_savepoint: None,
        and_chain: true,
        release: false,
        name: None,
        force: None,
    }));
    assert_eq!(rows, 0);
}

#[test]
fn rollback_release_ignored() {
    let rows = begin_insert_and_execute(&TransactionStmt::Rollback(RollbackStmt {
        to_savepoint: None,
        and_chain: false,
        release: true,
        name: None,
        force: None,
    }));
    assert_eq!(rows, 0);
}

// ==========================================================================
// BEGIN — ignored fields
// ==========================================================================

#[test]
fn begin_modes_ignored() {
    let db = conn();
    setup_table(&db);

    // modes field is ignored by the SQLite renderer — only lock_type matters
    let begin = render(&TransactionStmt::Begin(BeginStmt {
        modes: Some(vec![
            TransactionMode::IsolationLevel(IsolationLevel::Serializable),
            TransactionMode::ReadOnly,
        ]),
        lock_type: None,
        name: None,
        with_mark: None,
    }));
    db.execute_batch(&begin).unwrap();

    db.execute("INSERT INTO items (id, name) VALUES (1, 'apple')", [])
        .unwrap();

    let commit = render(&TransactionStmt::Commit(CommitStmt {
        and_chain: false,
        release: false,
        name: None,
        comment: None,
        write_mode: None,
        force: None,
    }));
    db.execute_batch(&commit).unwrap();

    assert_eq!(count_rows(&db), 1);
}
