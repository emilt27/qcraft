use super::custom::CustomTransaction;

/// Transaction control statements.
#[derive(Debug, Clone)]
pub enum TransactionStmt {
    Begin(BeginStmt),
    Commit(CommitStmt),
    Rollback(RollbackStmt),
    Savepoint(SavepointStmt),
    ReleaseSavepoint(ReleaseSavepointStmt),
    SetTransaction(SetTransactionStmt),
    LockTable(LockTableStmt),
    /// PG: PREPARE TRANSACTION 'id'
    PrepareTransaction(PrepareTransactionStmt),
    /// PG: COMMIT PREPARED 'id'
    CommitPrepared(CommitPreparedStmt),
    /// PG: ROLLBACK PREPARED 'id'
    RollbackPrepared(RollbackPreparedStmt),
    Custom(Box<dyn CustomTransaction>),
}

impl TransactionStmt {
    pub fn begin() -> Self {
        Self::Begin(BeginStmt::default())
    }

    pub fn commit() -> Self {
        Self::Commit(CommitStmt::default())
    }

    pub fn rollback() -> Self {
        Self::Rollback(RollbackStmt::default())
    }

    pub fn savepoint(name: impl Into<String>) -> Self {
        Self::Savepoint(SavepointStmt { name: name.into() })
    }

    pub fn release(name: impl Into<String>) -> Self {
        Self::ReleaseSavepoint(ReleaseSavepointStmt { name: name.into() })
    }

    pub fn rollback_to(name: impl Into<String>) -> Self {
        Self::Rollback(RollbackStmt {
            to_savepoint: Some(name.into()),
            ..Default::default()
        })
    }
}

// ---------------------------------------------------------------------------
// BEGIN / START TRANSACTION
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct BeginStmt {
    /// Transaction modes (isolation level, access mode, deferrable).
    pub modes: Option<Vec<TransactionMode>>,
    /// SQLite: DEFERRED / IMMEDIATE / EXCLUSIVE.
    pub lock_type: Option<SqliteLockType>,
    /// SQL Server: named transaction.
    pub name: Option<String>,
    /// SQL Server: WITH MARK 'description'.
    pub with_mark: Option<String>,
}

impl BeginStmt {
    pub fn with_isolation(level: IsolationLevel) -> Self {
        Self {
            modes: Some(vec![TransactionMode::IsolationLevel(level)]),
            ..Default::default()
        }
    }

    pub fn read_only() -> Self {
        Self {
            modes: Some(vec![TransactionMode::ReadOnly]),
            ..Default::default()
        }
    }

    pub fn sqlite_deferred() -> Self {
        Self {
            lock_type: Some(SqliteLockType::Deferred),
            ..Default::default()
        }
    }

    pub fn sqlite_immediate() -> Self {
        Self {
            lock_type: Some(SqliteLockType::Immediate),
            ..Default::default()
        }
    }

    pub fn sqlite_exclusive() -> Self {
        Self {
            lock_type: Some(SqliteLockType::Exclusive),
            ..Default::default()
        }
    }
}

/// SQLite BEGIN lock type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqliteLockType {
    Deferred,
    Immediate,
    Exclusive,
}

// ---------------------------------------------------------------------------
// COMMIT
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct CommitStmt {
    /// PG/MySQL: AND CHAIN — start new transaction immediately.
    pub and_chain: bool,
    /// MySQL: RELEASE — disconnect session after commit.
    pub release: bool,
    /// SQL Server: transaction name.
    pub name: Option<String>,
    /// Oracle: COMMENT 'text'.
    pub comment: Option<String>,
    /// Oracle: WRITE mode (sync/async).
    pub write_mode: Option<OracleWriteMode>,
    /// Oracle: FORCE 'transaction_id' for in-doubt distributed txns.
    pub force: Option<String>,
}

/// Oracle COMMIT WRITE options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OracleWriteMode {
    pub wait: OracleWriteWait,
    pub flush: OracleWriteFlush,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleWriteWait {
    Wait,
    NoWait,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleWriteFlush {
    Immediate,
    Batch,
}

// ---------------------------------------------------------------------------
// ROLLBACK
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct RollbackStmt {
    /// Roll back to a savepoint instead of the whole transaction.
    pub to_savepoint: Option<String>,
    /// PG/MySQL: AND CHAIN — start new transaction immediately.
    pub and_chain: bool,
    /// MySQL: RELEASE — disconnect session after rollback.
    pub release: bool,
    /// SQL Server: transaction name (rolls back entire txn, not a savepoint).
    pub name: Option<String>,
    /// Oracle: FORCE 'transaction_id' for in-doubt distributed txns.
    pub force: Option<String>,
}

// ---------------------------------------------------------------------------
// SAVEPOINT
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SavepointStmt {
    pub name: String,
}

// ---------------------------------------------------------------------------
// RELEASE SAVEPOINT
// ---------------------------------------------------------------------------

/// Supported by PG, SQLite, MySQL. Not supported by Oracle, SQL Server.
#[derive(Debug, Clone)]
pub struct ReleaseSavepointStmt {
    pub name: String,
}

// ---------------------------------------------------------------------------
// SET TRANSACTION
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SetTransactionStmt {
    /// Isolation level, access mode, deferrable.
    pub modes: Vec<TransactionMode>,
    /// Scope: current transaction (default), session, or global.
    pub scope: Option<TransactionScope>,
    /// PG: SET TRANSACTION SNAPSHOT snapshot_id.
    pub snapshot_id: Option<String>,
    /// Oracle: transaction name.
    pub name: Option<String>,
}

/// Transaction mode options used in BEGIN and SET TRANSACTION.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionMode {
    IsolationLevel(IsolationLevel),
    ReadOnly,
    ReadWrite,
    /// PG only: DEFERRABLE (only with SERIALIZABLE READ ONLY).
    Deferrable,
    /// PG only: NOT DEFERRABLE.
    NotDeferrable,
    /// MySQL only: WITH CONSISTENT SNAPSHOT.
    WithConsistentSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
    /// SQL Server only.
    Snapshot,
}

/// Scope for SET TRANSACTION.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionScope {
    /// PG: SET SESSION CHARACTERISTICS AS TRANSACTION ...
    /// MySQL: SET SESSION TRANSACTION ...
    Session,
    /// MySQL: SET GLOBAL TRANSACTION ...
    Global,
}

// ---------------------------------------------------------------------------
// LOCK TABLE
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LockTableStmt {
    pub tables: Vec<LockTableDef>,
    /// PG: NOWAIT — error immediately if lock unavailable.
    pub nowait: bool,
}

#[derive(Debug, Clone)]
pub struct LockTableDef {
    pub table: String,
    /// Optional schema prefix.
    pub schema: Option<String>,
    pub mode: LockMode,
    /// PG: ONLY (exclude inherited tables).
    pub only: bool,
    /// MySQL: table alias.
    pub alias: Option<String>,
    /// Oracle: WAIT N seconds.
    pub wait: Option<u64>,
    /// Oracle: partition targeting.
    pub partition: Option<String>,
}

/// Lock modes across all databases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockMode {
    // PG modes (8 levels)
    AccessShare,
    RowShare,
    RowExclusive,
    ShareUpdateExclusive,
    Share,
    ShareRowExclusive,
    Exclusive,
    AccessExclusive,
    // MySQL modes
    Read,
    ReadLocal,
    Write,
    LowPriorityWrite,
}

// ---------------------------------------------------------------------------
// Two-Phase Commit (PG)
// ---------------------------------------------------------------------------

/// PREPARE TRANSACTION 'transaction_id'
#[derive(Debug, Clone)]
pub struct PrepareTransactionStmt {
    pub transaction_id: String,
}

/// COMMIT PREPARED 'transaction_id'
#[derive(Debug, Clone)]
pub struct CommitPreparedStmt {
    pub transaction_id: String,
}

/// ROLLBACK PREPARED 'transaction_id'
#[derive(Debug, Clone)]
pub struct RollbackPreparedStmt {
    pub transaction_id: String,
}
