use qcraft_core::ast::tcl::*;
use qcraft_postgres::PostgresRenderer;

fn render(stmt: TransactionStmt) -> String {
    let renderer = PostgresRenderer::new();
    let (sql, _params) = renderer.render_transaction_stmt(&stmt).unwrap();
    sql
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
    assert_eq!(sql, "BEGIN");
}

#[test]
fn begin_isolation_serializable() {
    let sql = render(TransactionStmt::Begin(BeginStmt {
        modes: Some(vec![TransactionMode::IsolationLevel(
            IsolationLevel::Serializable,
        )]),
        lock_type: None,
        name: None,
        with_mark: None,
    }));
    assert_eq!(sql, "BEGIN ISOLATION LEVEL SERIALIZABLE");
}

#[test]
fn begin_read_only() {
    let sql = render(TransactionStmt::Begin(BeginStmt {
        modes: Some(vec![TransactionMode::ReadOnly]),
        lock_type: None,
        name: None,
        with_mark: None,
    }));
    assert_eq!(sql, "BEGIN READ ONLY");
}

#[test]
fn begin_serializable_read_only_deferrable() {
    let sql = render(TransactionStmt::Begin(BeginStmt {
        modes: Some(vec![
            TransactionMode::IsolationLevel(IsolationLevel::Serializable),
            TransactionMode::ReadOnly,
            TransactionMode::Deferrable,
        ]),
        lock_type: None,
        name: None,
        with_mark: None,
    }));
    assert_eq!(
        sql,
        "BEGIN ISOLATION LEVEL SERIALIZABLE, READ ONLY, DEFERRABLE"
    );
}

#[test]
fn begin_read_committed_read_write() {
    let sql = render(TransactionStmt::Begin(BeginStmt {
        modes: Some(vec![
            TransactionMode::IsolationLevel(IsolationLevel::ReadCommitted),
            TransactionMode::ReadWrite,
        ]),
        lock_type: None,
        name: None,
        with_mark: None,
    }));
    assert_eq!(sql, "BEGIN ISOLATION LEVEL READ COMMITTED, READ WRITE");
}

#[test]
fn begin_not_deferrable() {
    let sql = render(TransactionStmt::Begin(BeginStmt {
        modes: Some(vec![
            TransactionMode::IsolationLevel(IsolationLevel::Serializable),
            TransactionMode::ReadOnly,
            TransactionMode::NotDeferrable,
        ]),
        lock_type: None,
        name: None,
        with_mark: None,
    }));
    assert_eq!(
        sql,
        "BEGIN ISOLATION LEVEL SERIALIZABLE, READ ONLY, NOT DEFERRABLE"
    );
}

// ---------------------------------------------------------------------------
// COMMIT
// ---------------------------------------------------------------------------

#[test]
fn commit_simple() {
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

#[test]
fn commit_and_chain() {
    let sql = render(TransactionStmt::Commit(CommitStmt {
        and_chain: true,
        release: false,
        name: None,
        comment: None,
        write_mode: None,
        force: None,
    }));
    assert_eq!(sql, "COMMIT AND CHAIN");
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

#[test]
fn rollback_and_chain() {
    let sql = render(TransactionStmt::Rollback(RollbackStmt {
        to_savepoint: None,
        and_chain: true,
        release: false,
        name: None,
        force: None,
    }));
    assert_eq!(sql, "ROLLBACK AND CHAIN");
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
// SET TRANSACTION
// ---------------------------------------------------------------------------

#[test]
fn set_transaction_isolation() {
    let sql = render(TransactionStmt::SetTransaction(SetTransactionStmt {
        modes: vec![TransactionMode::IsolationLevel(
            IsolationLevel::RepeatableRead,
        )],
        scope: None,
        snapshot_id: None,
        name: None,
    }));
    assert_eq!(sql, "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ");
}

#[test]
fn set_session_transaction() {
    let sql = render(TransactionStmt::SetTransaction(SetTransactionStmt {
        modes: vec![
            TransactionMode::IsolationLevel(IsolationLevel::ReadCommitted),
            TransactionMode::ReadWrite,
        ],
        scope: Some(TransactionScope::Session),
        snapshot_id: None,
        name: None,
    }));
    assert_eq!(
        sql,
        "SET SESSION CHARACTERISTICS AS TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE"
    );
}

#[test]
fn set_transaction_snapshot() {
    let sql = render(TransactionStmt::SetTransaction(SetTransactionStmt {
        modes: vec![],
        scope: None,
        snapshot_id: Some("00000003-0000001A-1".into()),
        name: None,
    }));
    assert_eq!(sql, "SET TRANSACTION SNAPSHOT '00000003-0000001A-1'");
}

// ---------------------------------------------------------------------------
// LOCK TABLE
// ---------------------------------------------------------------------------

#[test]
fn lock_table_simple() {
    let sql = render(TransactionStmt::LockTable(LockTableStmt {
        tables: vec![LockTableDef {
            table: "users".into(),
            schema: None,
            mode: LockMode::AccessExclusive,
            only: false,
            alias: None,
            wait: None,
            partition: None,
        }],
        nowait: false,
    }));
    assert_eq!(sql, "LOCK TABLE \"users\" IN ACCESS EXCLUSIVE MODE");
}

#[test]
fn lock_table_nowait() {
    let sql = render(TransactionStmt::LockTable(LockTableStmt {
        tables: vec![LockTableDef {
            table: "orders".into(),
            schema: Some("public".into()),
            mode: LockMode::RowExclusive,
            only: false,
            alias: None,
            wait: None,
            partition: None,
        }],
        nowait: true,
    }));
    assert_eq!(
        sql,
        "LOCK TABLE \"public\".\"orders\" IN ROW EXCLUSIVE MODE NOWAIT"
    );
}

#[test]
fn lock_table_only() {
    let sql = render(TransactionStmt::LockTable(LockTableStmt {
        tables: vec![LockTableDef {
            table: "events".into(),
            schema: None,
            mode: LockMode::Share,
            only: true,
            alias: None,
            wait: None,
            partition: None,
        }],
        nowait: false,
    }));
    assert_eq!(sql, "LOCK TABLE ONLY \"events\" IN SHARE MODE");
}

#[test]
fn lock_table_multiple() {
    let sql = render(TransactionStmt::LockTable(LockTableStmt {
        tables: vec![
            LockTableDef {
                table: "t1".into(),
                schema: None,
                mode: LockMode::RowShare,
                only: false,
                alias: None,
                wait: None,
                partition: None,
            },
            LockTableDef {
                table: "t2".into(),
                schema: None,
                mode: LockMode::RowShare,
                only: false,
                alias: None,
                wait: None,
                partition: None,
            },
        ],
        nowait: false,
    }));
    assert_eq!(sql, "LOCK TABLE \"t1\", \"t2\" IN ROW SHARE MODE");
}

#[test]
fn lock_table_all_modes() {
    let modes = [
        (LockMode::AccessShare, "ACCESS SHARE"),
        (LockMode::RowShare, "ROW SHARE"),
        (LockMode::RowExclusive, "ROW EXCLUSIVE"),
        (LockMode::ShareUpdateExclusive, "SHARE UPDATE EXCLUSIVE"),
        (LockMode::Share, "SHARE"),
        (LockMode::ShareRowExclusive, "SHARE ROW EXCLUSIVE"),
        (LockMode::Exclusive, "EXCLUSIVE"),
        (LockMode::AccessExclusive, "ACCESS EXCLUSIVE"),
    ];
    for (mode, expected) in modes {
        let sql = render(TransactionStmt::LockTable(LockTableStmt {
            tables: vec![LockTableDef {
                table: "t".into(),
                schema: None,
                mode,
                only: false,
                alias: None,
                wait: None,
                partition: None,
            }],
            nowait: false,
        }));
        assert_eq!(sql, format!("LOCK TABLE \"t\" IN {} MODE", expected));
    }
}

// ---------------------------------------------------------------------------
// 2PC: PREPARE TRANSACTION
// ---------------------------------------------------------------------------

#[test]
fn prepare_transaction() {
    let sql = render(TransactionStmt::PrepareTransaction(
        PrepareTransactionStmt {
            transaction_id: "foobar".into(),
        },
    ));
    assert_eq!(sql, "PREPARE TRANSACTION 'foobar'");
}

#[test]
fn commit_prepared() {
    let sql = render(TransactionStmt::CommitPrepared(CommitPreparedStmt {
        transaction_id: "foobar".into(),
    }));
    assert_eq!(sql, "COMMIT PREPARED 'foobar'");
}

#[test]
fn rollback_prepared() {
    let sql = render(TransactionStmt::RollbackPrepared(RollbackPreparedStmt {
        transaction_id: "foobar".into(),
    }));
    assert_eq!(sql, "ROLLBACK PREPARED 'foobar'");
}
