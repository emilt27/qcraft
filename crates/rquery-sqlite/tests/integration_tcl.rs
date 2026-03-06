//! Integration tests for SQLite TCL (transaction control) statements
//! that run against a real in-memory SQLite database.

use rusqlite::Connection;
use rquery_core::ast::tcl::*;
use rquery_sqlite::SqliteRenderer;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn conn() -> Connection {
    Connection::open_in_memory().unwrap()
}

fn render(stmt: &TransactionStmt) -> String {
    let renderer = SqliteRenderer::new();
    let (sql, _) = renderer.render_transaction_stmt(stmt).unwrap();
    sql
}

fn setup_table(conn: &Connection) {
    conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL)", [])
        .unwrap();
}

fn count_rows(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))
        .unwrap()
}

fn row_exists(conn: &Connection, id: i64) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM items WHERE id = ?",
        [id],
        |row| row.get::<_, i64>(0),
    )
    .unwrap()
        > 0
}

// ==========================================================================
// BEGIN / COMMIT / ROLLBACK — core flows
// ==========================================================================

#[test]
fn begin_commit_persists_data() {
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

#[test]
fn begin_rollback_discards_data() {
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

    let rollback = render(&TransactionStmt::Rollback(RollbackStmt {
        to_savepoint: None,
        and_chain: false,
        release: false,
        name: None,
        force: None,
    }));
    db.execute_batch(&rollback).unwrap();

    assert_eq!(count_rows(&db), 0);
}

// ==========================================================================
// BEGIN lock types
// ==========================================================================

#[test]
fn begin_deferred() {
    let db = conn();
    setup_table(&db);

    let begin = render(&TransactionStmt::Begin(BeginStmt {
        modes: None,
        lock_type: Some(SqliteLockType::Deferred),
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

#[test]
fn begin_immediate() {
    let db = conn();
    setup_table(&db);

    let begin = render(&TransactionStmt::Begin(BeginStmt {
        modes: None,
        lock_type: Some(SqliteLockType::Immediate),
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

#[test]
fn begin_exclusive() {
    let db = conn();
    setup_table(&db);

    let begin = render(&TransactionStmt::Begin(BeginStmt {
        modes: None,
        lock_type: Some(SqliteLockType::Exclusive),
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

// ==========================================================================
// SAVEPOINT / RELEASE SAVEPOINT / ROLLBACK TO SAVEPOINT
// ==========================================================================

#[test]
fn savepoint_release() {
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

    let savepoint = render(&TransactionStmt::Savepoint(SavepointStmt {
        name: "sp1".to_string(),
    }));
    db.execute_batch(&savepoint).unwrap();

    db.execute("INSERT INTO items (id, name) VALUES (2, 'banana')", [])
        .unwrap();

    let release = render(&TransactionStmt::ReleaseSavepoint(ReleaseSavepointStmt {
        name: "sp1".to_string(),
    }));
    db.execute_batch(&release).unwrap();

    let commit = render(&TransactionStmt::Commit(CommitStmt {
        and_chain: false,
        release: false,
        name: None,
        comment: None,
        write_mode: None,
        force: None,
    }));
    db.execute_batch(&commit).unwrap();

    assert_eq!(count_rows(&db), 2);
    assert!(row_exists(&db, 1));
    assert!(row_exists(&db, 2));
}

#[test]
fn savepoint_rollback_to() {
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

    let savepoint = render(&TransactionStmt::Savepoint(SavepointStmt {
        name: "sp1".to_string(),
    }));
    db.execute_batch(&savepoint).unwrap();

    db.execute("INSERT INTO items (id, name) VALUES (2, 'banana')", [])
        .unwrap();

    let rollback_to = render(&TransactionStmt::Rollback(RollbackStmt {
        to_savepoint: Some("sp1".to_string()),
        and_chain: false,
        release: false,
        name: None,
        force: None,
    }));
    db.execute_batch(&rollback_to).unwrap();

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
    assert!(row_exists(&db, 1));
    assert!(!row_exists(&db, 2));
}

#[test]
fn nested_savepoints() {
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

    let sp_outer = render(&TransactionStmt::Savepoint(SavepointStmt {
        name: "sp_outer".to_string(),
    }));
    db.execute_batch(&sp_outer).unwrap();

    db.execute("INSERT INTO items (id, name) VALUES (2, 'banana')", [])
        .unwrap();

    let sp_inner = render(&TransactionStmt::Savepoint(SavepointStmt {
        name: "sp_inner".to_string(),
    }));
    db.execute_batch(&sp_inner).unwrap();

    db.execute("INSERT INTO items (id, name) VALUES (3, 'cherry')", [])
        .unwrap();

    // Rollback to inner savepoint — discards row 3 only
    let rollback_inner = render(&TransactionStmt::Rollback(RollbackStmt {
        to_savepoint: Some("sp_inner".to_string()),
        and_chain: false,
        release: false,
        name: None,
        force: None,
    }));
    db.execute_batch(&rollback_inner).unwrap();

    let commit = render(&TransactionStmt::Commit(CommitStmt {
        and_chain: false,
        release: false,
        name: None,
        comment: None,
        write_mode: None,
        force: None,
    }));
    db.execute_batch(&commit).unwrap();

    assert_eq!(count_rows(&db), 2);
    assert!(row_exists(&db, 1));
    assert!(row_exists(&db, 2));
    assert!(!row_exists(&db, 3));
}
