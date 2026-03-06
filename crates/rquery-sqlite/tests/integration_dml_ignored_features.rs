//! Integration tests that verify the SQLite renderer silently ignores
//! database-specific fields (PG, MySQL) and still produces valid SQL.

use rquery_core::ast::common::SchemaRef;
use rquery_core::ast::conditions::{CompareOp, Comparison, ConditionNode, Conditions, Connector};
use rquery_core::ast::dml::*;
use rquery_core::ast::expr::Expr;
use rquery_core::ast::query::TableSource;
use rquery_core::ast::value::Value;
use rquery_sqlite::SqliteRenderer;
use rusqlite::Connection;
use rusqlite::types::ToSql as RusqliteToSql;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn conn() -> Connection {
    Connection::open_in_memory().unwrap()
}

fn render(stmt: &MutationStmt) -> (String, Vec<Value>) {
    let renderer = SqliteRenderer::new();
    renderer.render_mutation_stmt(stmt).unwrap()
}

fn to_sqlite_params(values: &[Value]) -> Vec<Box<dyn RusqliteToSql>> {
    values
        .iter()
        .map(|v| -> Box<dyn RusqliteToSql> {
            match v {
                Value::Null => Box::new(rusqlite::types::Null),
                Value::Bool(b) => Box::new(*b),
                Value::Int(n) => Box::new(*n),
                Value::Float(f) => Box::new(*f),
                Value::Str(s) => Box::new(s.clone()),
                Value::Bytes(b) => Box::new(b.clone()),
                Value::Date(s) | Value::DateTime(s) | Value::Time(s) => Box::new(s.clone()),
                Value::Decimal(s) => Box::new(s.clone()),
                Value::Uuid(s) => Box::new(s.clone()),
                Value::Json(s) | Value::Jsonb(s) => Box::new(s.clone()),
                Value::IpNetwork(s) => Box::new(s.clone()),
                _ => Box::new(format!("{:?}", v)),
            }
        })
        .collect()
}

fn as_sqlite_params(boxed: &[Box<dyn RusqliteToSql>]) -> Vec<&dyn RusqliteToSql> {
    boxed.iter().map(|b| b.as_ref()).collect()
}

fn setup_users(conn: &Connection) {
    conn.execute(
        r#"CREATE TABLE "users" ("id" INTEGER PRIMARY KEY, "name" TEXT NOT NULL, "email" TEXT UNIQUE)"#,
        [],
    )
    .unwrap();
}

fn seed_user(conn: &Connection) {
    conn.execute(
        r#"INSERT INTO "users" ("id", "name", "email") VALUES (1, 'Alice', 'alice@example.com')"#,
        [],
    )
    .unwrap();
}

fn count_rows(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM \"{table}\""), [], |row| {
        row.get(0)
    })
    .unwrap()
}

// ==========================================================================
// INSERT — ignored fields
// ==========================================================================

#[test]
fn insert_overriding_ignored() {
    let c = conn();
    setup_users(&c);

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into(), "email".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("Alice".into())),
            Expr::Value(Value::Str("alice@example.com".into())),
        ]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: Some(OverridingKind::System),
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });

    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();
    assert_eq!(count_rows(&c, "users"), 1);
}

#[test]
fn insert_partition_ignored() {
    let c = conn();
    setup_users(&c);

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into(), "email".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("Alice".into())),
            Expr::Value(Value::Str("alice@example.com".into())),
        ]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: Some(vec!["p1".into()]),
        ignore: false,
    });

    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();
    assert_eq!(count_rows(&c, "users"), 1);
}

#[test]
fn insert_ignore_flag_ignored() {
    let c = conn();
    setup_users(&c);

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into(), "email".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("Alice".into())),
            Expr::Value(Value::Str("alice@example.com".into())),
        ]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: true,
    });

    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();
    assert_eq!(count_rows(&c, "users"), 1);
}

// ==========================================================================
// UPDATE — ignored fields
// ==========================================================================

#[test]
fn update_only_ignored() {
    let c = conn();
    setup_users(&c);
    seed_user(&c);

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users"),
        assignments: vec![("name".into(), Expr::Value(Value::Str("Bob".into())))],
        from: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw {
                    sql: "\"id\"".into(),
                    params: vec![],
                },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            })],
            connector: Connector::And,
            negated: false,
        }),
        returning: None,
        ctes: None,
        conflict_resolution: None,
        order_by: None,
        limit: None,
        offset: None,
        only: true,
        partition: None,
        ignore: false,
    });

    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    let name: String = c
        .query_row(r#"SELECT "name" FROM "users" WHERE "id" = 1"#, [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(name, "Bob");
}

#[test]
fn update_partition_ignored() {
    let c = conn();
    setup_users(&c);
    seed_user(&c);

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users"),
        assignments: vec![("name".into(), Expr::Value(Value::Str("Bob".into())))],
        from: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw {
                    sql: "\"id\"".into(),
                    params: vec![],
                },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            })],
            connector: Connector::And,
            negated: false,
        }),
        returning: None,
        ctes: None,
        conflict_resolution: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: Some(vec!["p0".into()]),
        ignore: false,
    });

    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    let name: String = c
        .query_row(r#"SELECT "name" FROM "users" WHERE "id" = 1"#, [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(name, "Bob");
}

#[test]
fn update_ignore_flag_ignored() {
    let c = conn();
    setup_users(&c);
    seed_user(&c);

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users"),
        assignments: vec![("name".into(), Expr::Value(Value::Str("Bob".into())))],
        from: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw {
                    sql: "\"id\"".into(),
                    params: vec![],
                },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            })],
            connector: Connector::And,
            negated: false,
        }),
        returning: None,
        ctes: None,
        conflict_resolution: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: true,
    });

    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    let name: String = c
        .query_row(r#"SELECT "name" FROM "users" WHERE "id" = 1"#, [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(name, "Bob");
}

// ==========================================================================
// DELETE — ignored fields
// ==========================================================================

#[test]
fn delete_using_ignored() {
    let c = conn();
    setup_users(&c);
    seed_user(&c);

    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("users"),
        using: Some(vec![TableSource::Table(SchemaRef::new("other"))]),
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw {
                    sql: "\"id\"".into(),
                    params: vec![],
                },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            })],
            connector: Connector::And,
            negated: false,
        }),
        returning: None,
        ctes: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });

    // USING is silently dropped — the DELETE should still work
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();
    assert_eq!(count_rows(&c, "users"), 0);
}

#[test]
fn delete_only_ignored() {
    let c = conn();
    setup_users(&c);
    seed_user(&c);

    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("users"),
        using: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw {
                    sql: "\"id\"".into(),
                    params: vec![],
                },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            })],
            connector: Connector::And,
            negated: false,
        }),
        returning: None,
        ctes: None,
        order_by: None,
        limit: None,
        offset: None,
        only: true,
        partition: None,
        ignore: false,
    });

    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();
    assert_eq!(count_rows(&c, "users"), 0);
}

#[test]
fn delete_partition_ignored() {
    let c = conn();
    setup_users(&c);
    seed_user(&c);

    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("users"),
        using: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw {
                    sql: "\"id\"".into(),
                    params: vec![],
                },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            })],
            connector: Connector::And,
            negated: false,
        }),
        returning: None,
        ctes: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: Some(vec!["p0".into(), "p1".into()]),
        ignore: false,
    });

    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();
    assert_eq!(count_rows(&c, "users"), 0);
}

#[test]
fn delete_ignore_flag_ignored() {
    let c = conn();
    setup_users(&c);
    seed_user(&c);

    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("users"),
        using: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw {
                    sql: "\"id\"".into(),
                    params: vec![],
                },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            })],
            connector: Connector::And,
            negated: false,
        }),
        returning: None,
        ctes: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: true,
    });

    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();
    assert_eq!(count_rows(&c, "users"), 0);
}
