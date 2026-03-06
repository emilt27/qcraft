//! Tests that verify PostgreSQL renderer silently ignores non-PG TCL features
//! (SQLite lock types, named transactions, release flags, etc.)
//! while still producing valid, executable SQL.

use postgres::{Client, NoTls};
use testcontainers::ImageExt;
use testcontainers::runners::SyncRunner;
use testcontainers_modules::postgres::Postgres;

use rquery_core::ast::tcl::*;
use rquery_postgres::PostgresRenderer;

fn render(stmt: &TransactionStmt) -> String {
    let renderer = PostgresRenderer::new();
    let (sql, _) = renderer.render_transaction_stmt(stmt).unwrap();
    sql
}

fn connect() -> (impl std::any::Any, Client) {
    let node = Postgres::default().with_tag("16-alpine").start().unwrap();
    let conn_str = format!(
        "host={} port={} user=postgres password=postgres dbname=postgres",
        node.get_host().unwrap(),
        node.get_host_port_ipv4(5432).unwrap(),
    );
    let client = Client::connect(&conn_str, NoTls).unwrap();
    (node, client)
}

// ==========================================================================
// BEGIN — ignored fields
// ==========================================================================

#[test]
fn begin_lock_type_ignored() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    let begin = TransactionStmt::Begin(BeginStmt {
        modes: None,
        lock_type: Some(SqliteLockType::Exclusive),
        name: None,
        with_mark: None,
    });
    // SQLite lock_type is ignored — renders as plain BEGIN
    client.batch_execute(&render(&begin)).unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();
    client.batch_execute("COMMIT").unwrap();

    let rows = client.query("SELECT id FROM t", &[]).unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn begin_name_ignored() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    let begin = TransactionStmt::Begin(BeginStmt {
        modes: None,
        lock_type: None,
        name: Some("tx1".to_string()),
        with_mark: None,
    });
    // SQL Server named transaction is ignored
    client.batch_execute(&render(&begin)).unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();
    client.batch_execute("COMMIT").unwrap();

    let rows = client.query("SELECT id FROM t", &[]).unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn begin_with_mark_ignored() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    let begin = TransactionStmt::Begin(BeginStmt {
        modes: None,
        lock_type: None,
        name: None,
        with_mark: Some("mark1".to_string()),
    });
    // SQL Server WITH MARK is ignored
    client.batch_execute(&render(&begin)).unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();
    client.batch_execute("COMMIT").unwrap();

    let rows = client.query("SELECT id FROM t", &[]).unwrap();
    assert_eq!(rows.len(), 1);
}

// ==========================================================================
// COMMIT — ignored fields
// ==========================================================================

#[test]
fn commit_release_ignored() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    client.batch_execute("BEGIN").unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();

    let commit = TransactionStmt::Commit(CommitStmt {
        and_chain: false,
        release: true, // MySQL RELEASE — ignored
        name: None,
        comment: None,
        write_mode: None,
        force: None,
    });
    client.batch_execute(&render(&commit)).unwrap();

    let rows = client.query("SELECT id FROM t", &[]).unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn commit_name_ignored() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    client.batch_execute("BEGIN").unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();

    let commit = TransactionStmt::Commit(CommitStmt {
        and_chain: false,
        release: false,
        name: Some("tx1".to_string()), // SQL Server name — ignored
        comment: None,
        write_mode: None,
        force: None,
    });
    client.batch_execute(&render(&commit)).unwrap();

    let rows = client.query("SELECT id FROM t", &[]).unwrap();
    assert_eq!(rows.len(), 1);
}

// ==========================================================================
// ROLLBACK — ignored fields
// ==========================================================================

#[test]
fn rollback_release_ignored() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    client.batch_execute("BEGIN").unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();

    let rollback = TransactionStmt::Rollback(RollbackStmt {
        to_savepoint: None,
        and_chain: false,
        release: true, // MySQL RELEASE — ignored
        name: None,
        force: None,
    });
    client.batch_execute(&render(&rollback)).unwrap();

    let rows = client.query("SELECT id FROM t", &[]).unwrap();
    assert_eq!(rows.len(), 0);
}

#[test]
fn rollback_name_ignored() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    client.batch_execute("BEGIN").unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();

    let rollback = TransactionStmt::Rollback(RollbackStmt {
        to_savepoint: None,
        and_chain: false,
        release: false,
        name: Some("tx1".to_string()), // SQL Server name — ignored
        force: None,
    });
    client.batch_execute(&render(&rollback)).unwrap();

    let rows = client.query("SELECT id FROM t", &[]).unwrap();
    assert_eq!(rows.len(), 0);
}
