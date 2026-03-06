//! Integration tests for PostgreSQL TCL (Transaction Control Language) rendering
//! executed against a real PostgreSQL instance via testcontainers.

use postgres::{Client, NoTls};
use testcontainers::runners::SyncRunner;
use testcontainers::ImageExt;
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

fn connect_with_2pc() -> (impl std::any::Any, Client) {
    let node = Postgres::default()
        .with_tag("16-alpine")
        .with_cmd(["postgres", "-c", "max_prepared_transactions=10"])
        .start()
        .unwrap();
    let conn_str = format!(
        "host={} port={} user=postgres password=postgres dbname=postgres",
        node.get_host().unwrap(),
        node.get_host_port_ipv4(5432).unwrap(),
    );
    let client = Client::connect(&conn_str, NoTls).unwrap();
    (node, client)
}

// ==========================================================================
// BEGIN / COMMIT / ROLLBACK — basic
// ==========================================================================

#[test]
fn begin_commit_persists_data() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    let begin = TransactionStmt::Begin(BeginStmt {
        modes: None,
        lock_type: None,
        name: None,
        with_mark: None,
    });
    let commit = TransactionStmt::Commit(CommitStmt {
        and_chain: false,
        release: false,
        name: None,
        comment: None,
        write_mode: None,
        force: None,
    });

    client.batch_execute(&render(&begin)).unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();
    client.batch_execute(&render(&commit)).unwrap();

    let rows = client.query("SELECT id FROM t", &[]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, i32>(0), 1);
}

#[test]
fn begin_rollback_discards_data() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    let begin = TransactionStmt::Begin(BeginStmt {
        modes: None,
        lock_type: None,
        name: None,
        with_mark: None,
    });
    let rollback = TransactionStmt::Rollback(RollbackStmt {
        to_savepoint: None,
        and_chain: false,
        release: false,
        name: None,
        force: None,
    });

    client.batch_execute(&render(&begin)).unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();
    client.batch_execute(&render(&rollback)).unwrap();

    let rows = client.query("SELECT id FROM t", &[]).unwrap();
    assert_eq!(rows.len(), 0);
}

// ==========================================================================
// BEGIN — isolation levels
// ==========================================================================

#[test]
fn begin_isolation_serializable() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    let begin = TransactionStmt::Begin(BeginStmt {
        modes: Some(vec![TransactionMode::IsolationLevel(IsolationLevel::Serializable)]),
        lock_type: None,
        name: None,
        with_mark: None,
    });
    client.batch_execute(&render(&begin)).unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();

    let row = client.query_one("SHOW transaction_isolation", &[]).unwrap();
    let level: String = row.get(0);
    assert_eq!(level, "serializable");

    client.batch_execute("COMMIT").unwrap();
}

#[test]
fn begin_read_committed() {
    let (_node, mut client) = connect();

    let begin = TransactionStmt::Begin(BeginStmt {
        modes: Some(vec![TransactionMode::IsolationLevel(IsolationLevel::ReadCommitted)]),
        lock_type: None,
        name: None,
        with_mark: None,
    });
    client.batch_execute(&render(&begin)).unwrap();

    let row = client.query_one("SHOW transaction_isolation", &[]).unwrap();
    let level: String = row.get(0);
    assert_eq!(level, "read committed");

    client.batch_execute("ROLLBACK").unwrap();
}

#[test]
fn begin_repeatable_read() {
    let (_node, mut client) = connect();

    let begin = TransactionStmt::Begin(BeginStmt {
        modes: Some(vec![TransactionMode::IsolationLevel(IsolationLevel::RepeatableRead)]),
        lock_type: None,
        name: None,
        with_mark: None,
    });
    client.batch_execute(&render(&begin)).unwrap();

    let row = client.query_one("SHOW transaction_isolation", &[]).unwrap();
    let level: String = row.get(0);
    assert_eq!(level, "repeatable read");

    client.batch_execute("ROLLBACK").unwrap();
}

// ==========================================================================
// BEGIN — access modes and deferrable
// ==========================================================================

#[test]
fn begin_read_only() {
    let (_node, mut client) = connect();

    let begin = TransactionStmt::Begin(BeginStmt {
        modes: Some(vec![TransactionMode::ReadOnly]),
        lock_type: None,
        name: None,
        with_mark: None,
    });
    client.batch_execute(&render(&begin)).unwrap();

    let row = client.query_one("SHOW transaction_read_only", &[]).unwrap();
    let read_only: String = row.get(0);
    assert_eq!(read_only, "on");

    client.batch_execute("ROLLBACK").unwrap();
}

#[test]
fn begin_serializable_read_only_deferrable() {
    let (_node, mut client) = connect();

    let begin = TransactionStmt::Begin(BeginStmt {
        modes: Some(vec![
            TransactionMode::IsolationLevel(IsolationLevel::Serializable),
            TransactionMode::ReadOnly,
            TransactionMode::Deferrable,
        ]),
        lock_type: None,
        name: None,
        with_mark: None,
    });
    client.batch_execute(&render(&begin)).unwrap();

    let row = client.query_one("SHOW transaction_isolation", &[]).unwrap();
    assert_eq!(row.get::<_, String>(0), "serializable");

    let row = client.query_one("SHOW transaction_read_only", &[]).unwrap();
    assert_eq!(row.get::<_, String>(0), "on");

    let row = client.query_one("SHOW transaction_deferrable", &[]).unwrap();
    assert_eq!(row.get::<_, String>(0), "on");

    client.batch_execute("ROLLBACK").unwrap();
}

// ==========================================================================
// COMMIT AND CHAIN / ROLLBACK AND CHAIN
// ==========================================================================

#[test]
fn commit_and_chain() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    let begin = TransactionStmt::Begin(BeginStmt {
        modes: None,
        lock_type: None,
        name: None,
        with_mark: None,
    });
    let commit_chain = TransactionStmt::Commit(CommitStmt {
        and_chain: true,
        release: false,
        name: None,
        comment: None,
        write_mode: None,
        force: None,
    });

    client.batch_execute(&render(&begin)).unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();
    // COMMIT AND CHAIN commits and starts a new transaction immediately
    client.batch_execute(&render(&commit_chain)).unwrap();

    // We are now inside a new transaction — insert more data
    client.execute("INSERT INTO t VALUES (2)", &[]).unwrap();
    client.batch_execute("COMMIT").unwrap();

    let rows = client.query("SELECT id FROM t ORDER BY id", &[]).unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn rollback_and_chain() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    let begin = TransactionStmt::Begin(BeginStmt {
        modes: None,
        lock_type: None,
        name: None,
        with_mark: None,
    });
    let rollback_chain = TransactionStmt::Rollback(RollbackStmt {
        to_savepoint: None,
        and_chain: true,
        release: false,
        name: None,
        force: None,
    });

    client.batch_execute(&render(&begin)).unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();
    // ROLLBACK AND CHAIN rolls back and starts a new transaction
    client.batch_execute(&render(&rollback_chain)).unwrap();

    // We are inside a new transaction — insert and commit
    client.execute("INSERT INTO t VALUES (2)", &[]).unwrap();
    client.batch_execute("COMMIT").unwrap();

    let rows = client.query("SELECT id FROM t ORDER BY id", &[]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, i32>(0), 2);
}

// ==========================================================================
// SAVEPOINT / RELEASE SAVEPOINT / ROLLBACK TO SAVEPOINT
// ==========================================================================

#[test]
fn savepoint_release() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    client.batch_execute("BEGIN").unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();

    let savepoint = TransactionStmt::Savepoint(SavepointStmt {
        name: "sp1".to_string(),
    });
    client.batch_execute(&render(&savepoint)).unwrap();

    client.execute("INSERT INTO t VALUES (2)", &[]).unwrap();

    let release = TransactionStmt::ReleaseSavepoint(ReleaseSavepointStmt {
        name: "sp1".to_string(),
    });
    client.batch_execute(&render(&release)).unwrap();

    client.batch_execute("COMMIT").unwrap();

    let rows = client.query("SELECT id FROM t ORDER BY id", &[]).unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn savepoint_rollback_to() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    client.batch_execute("BEGIN").unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();

    let savepoint = TransactionStmt::Savepoint(SavepointStmt {
        name: "sp1".to_string(),
    });
    client.batch_execute(&render(&savepoint)).unwrap();

    client.execute("INSERT INTO t VALUES (2)", &[]).unwrap();

    let rollback_to = TransactionStmt::Rollback(RollbackStmt {
        to_savepoint: Some("sp1".to_string()),
        and_chain: false,
        release: false,
        name: None,
        force: None,
    });
    client.batch_execute(&render(&rollback_to)).unwrap();

    client.batch_execute("COMMIT").unwrap();

    // Only row 1 should persist; row 2 was rolled back to savepoint
    let rows = client.query("SELECT id FROM t ORDER BY id", &[]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, i32>(0), 1);
}

// ==========================================================================
// SET TRANSACTION / SET SESSION CHARACTERISTICS
// ==========================================================================

#[test]
fn set_transaction_isolation() {
    let (_node, mut client) = connect();

    client.batch_execute("BEGIN").unwrap();

    let set_tx = TransactionStmt::SetTransaction(SetTransactionStmt {
        modes: vec![TransactionMode::IsolationLevel(IsolationLevel::Serializable)],
        scope: None,
        snapshot_id: None,
        name: None,
    });
    client.batch_execute(&render(&set_tx)).unwrap();

    let row = client.query_one("SHOW transaction_isolation", &[]).unwrap();
    assert_eq!(row.get::<_, String>(0), "serializable");

    client.batch_execute("ROLLBACK").unwrap();
}

#[test]
fn set_session_characteristics() {
    let (_node, mut client) = connect();

    let set_session = TransactionStmt::SetTransaction(SetTransactionStmt {
        modes: vec![TransactionMode::IsolationLevel(IsolationLevel::Serializable)],
        scope: Some(TransactionScope::Session),
        snapshot_id: None,
        name: None,
    });
    client.batch_execute(&render(&set_session)).unwrap();

    // Verify it applies to new transactions
    client.batch_execute("BEGIN").unwrap();
    let row = client.query_one("SHOW transaction_isolation", &[]).unwrap();
    assert_eq!(row.get::<_, String>(0), "serializable");
    client.batch_execute("ROLLBACK").unwrap();

    // Reset to default
    client.batch_execute("SET SESSION CHARACTERISTICS AS TRANSACTION ISOLATION LEVEL READ COMMITTED").unwrap();
}

// ==========================================================================
// LOCK TABLE
// ==========================================================================

#[test]
fn lock_table_access_exclusive() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    client.batch_execute("BEGIN").unwrap();

    let lock = TransactionStmt::LockTable(LockTableStmt {
        tables: vec![LockTableDef {
            table: "t".to_string(),
            schema: None,
            mode: LockMode::AccessExclusive,
            only: false,
            alias: None,
            wait: None,
            partition: None,
        }],
        nowait: false,
    });
    client.batch_execute(&render(&lock)).unwrap();

    // If we got here, the lock was acquired successfully
    client.batch_execute("COMMIT").unwrap();
}

#[test]
fn lock_table_nowait() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    client.batch_execute("BEGIN").unwrap();

    let lock = TransactionStmt::LockTable(LockTableStmt {
        tables: vec![LockTableDef {
            table: "t".to_string(),
            schema: None,
            mode: LockMode::Exclusive,
            only: false,
            alias: None,
            wait: None,
            partition: None,
        }],
        nowait: true,
    });
    client.batch_execute(&render(&lock)).unwrap();

    client.batch_execute("COMMIT").unwrap();
}

#[test]
fn lock_table_only() {
    let (_node, mut client) = connect();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    client.batch_execute("BEGIN").unwrap();

    let lock = TransactionStmt::LockTable(LockTableStmt {
        tables: vec![LockTableDef {
            table: "t".to_string(),
            schema: None,
            mode: LockMode::Share,
            only: true,
            alias: None,
            wait: None,
            partition: None,
        }],
        nowait: false,
    });
    client.batch_execute(&render(&lock)).unwrap();

    client.batch_execute("COMMIT").unwrap();
}

// ==========================================================================
// Two-Phase Commit (2PC)
// ==========================================================================

#[test]
fn prepare_and_commit_prepared() {
    let (_node, mut client) = connect_with_2pc();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    client.batch_execute("BEGIN").unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();

    let prepare = TransactionStmt::PrepareTransaction(PrepareTransactionStmt {
        transaction_id: "tx_commit".into(),
    });
    client.batch_execute(&render(&prepare)).unwrap();

    // After PREPARE TRANSACTION, the transaction is no longer associated
    // with this session — but persists in the WAL.
    let commit = TransactionStmt::CommitPrepared(CommitPreparedStmt {
        transaction_id: "tx_commit".into(),
    });
    client.batch_execute(&render(&commit)).unwrap();

    let rows = client.query("SELECT id FROM t", &[]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, i32>(0), 1);
}

#[test]
fn prepare_and_rollback_prepared() {
    let (_node, mut client) = connect_with_2pc();
    client.execute("CREATE TABLE t (id INTEGER)", &[]).unwrap();

    client.batch_execute("BEGIN").unwrap();
    client.execute("INSERT INTO t VALUES (1)", &[]).unwrap();

    let prepare = TransactionStmt::PrepareTransaction(PrepareTransactionStmt {
        transaction_id: "tx_rollback".into(),
    });
    client.batch_execute(&render(&prepare)).unwrap();

    let rollback = TransactionStmt::RollbackPrepared(RollbackPreparedStmt {
        transaction_id: "tx_rollback".into(),
    });
    client.batch_execute(&render(&rollback)).unwrap();

    let rows = client.query("SELECT id FROM t", &[]).unwrap();
    assert_eq!(rows.len(), 0);
}
