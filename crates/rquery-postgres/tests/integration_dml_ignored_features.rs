//! Tests that verify PostgreSQL renderer silently ignores SQLite/MySQL-specific DML features
//! while still producing valid, executable SQL.

use postgres::{Client, NoTls};
use testcontainers::runners::SyncRunner;
use testcontainers::ImageExt;
use testcontainers_modules::postgres::Postgres;

use rquery_core::ast::common::{OrderByDef, OrderDir, SchemaRef};
use rquery_core::ast::conditions::{CompareOp, Comparison, ConditionNode, Conditions, Connector};
use rquery_core::ast::dml::*;
use rquery_core::ast::expr::Expr;
use rquery_core::ast::value::Value;
use rquery_postgres::PostgresRenderer;

fn render(stmt: &MutationStmt) -> String {
    let renderer = PostgresRenderer::new();
    let (sql, _) = renderer.render_mutation_stmt(stmt).unwrap();
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
// INSERT — ignored SQLite/MySQL features
// ==========================================================================

#[test]
fn insert_conflict_resolution_ignored() {
    let (_node, mut client) = connect();
    client
        .execute("CREATE TABLE t (id SERIAL PRIMARY KEY, val TEXT)", &[])
        .unwrap();

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("t"),
        columns: Some(vec!["val".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Str("x".into()))]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: Some(ConflictResolution::Replace),
        partition: None,
        ignore: false,
    });
    client.execute(&render(&stmt), &[]).unwrap();

    let val: String = client
        .query_one("SELECT val FROM t", &[])
        .unwrap()
        .get(0);
    assert_eq!(val, "x");
}

#[test]
fn insert_partition_ignored() {
    let (_node, mut client) = connect();
    client
        .execute("CREATE TABLE t (id SERIAL PRIMARY KEY, val TEXT)", &[])
        .unwrap();

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("t"),
        columns: Some(vec!["val".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Str("x".into()))]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: Some(vec!["p1".into()]),
        ignore: false,
    });
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM t", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
}

#[test]
fn insert_ignore_flag_ignored() {
    let (_node, mut client) = connect();
    client
        .execute("CREATE TABLE t (id SERIAL PRIMARY KEY, val TEXT)", &[])
        .unwrap();

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("t"),
        columns: Some(vec!["val".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Str("x".into()))]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: true,
    });
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM t", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
}

// ==========================================================================
// UPDATE — ignored SQLite/MySQL features
// ==========================================================================

#[test]
fn update_conflict_resolution_ignored() {
    let (_node, mut client) = connect();
    client
        .execute("CREATE TABLE t (id SERIAL PRIMARY KEY, val TEXT)", &[])
        .unwrap();
    client
        .execute("INSERT INTO t (val) VALUES ('old')", &[])
        .unwrap();

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("t"),
        assignments: vec![("val".into(), Expr::Value(Value::Str("new".into())))],
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
        conflict_resolution: Some(ConflictResolution::Ignore),
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });
    client.execute(&render(&stmt), &[]).unwrap();

    let val: String = client
        .query_one("SELECT val FROM t WHERE id = 1", &[])
        .unwrap()
        .get(0);
    assert_eq!(val, "new");
}

#[test]
fn update_order_by_ignored() {
    let (_node, mut client) = connect();
    client
        .execute("CREATE TABLE t (id SERIAL PRIMARY KEY, val TEXT)", &[])
        .unwrap();
    client
        .execute("INSERT INTO t (val) VALUES ('a'), ('b')", &[])
        .unwrap();

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("t"),
        assignments: vec![("val".into(), Expr::Value(Value::Str("updated".into())))],
        from: None,
        where_clause: None,
        returning: None,
        ctes: None,
        conflict_resolution: None,
        order_by: Some(vec![OrderByDef {
            expr: Expr::Raw {
                sql: "\"id\"".into(),
                params: vec![],
            },
            direction: OrderDir::Desc,
            nulls: None,
        }]),
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM t WHERE val = 'updated'", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 2);
}

#[test]
fn update_limit_ignored() {
    let (_node, mut client) = connect();
    client
        .execute("CREATE TABLE t (id SERIAL PRIMARY KEY, val TEXT)", &[])
        .unwrap();
    client
        .execute("INSERT INTO t (val) VALUES ('a'), ('b')", &[])
        .unwrap();

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("t"),
        assignments: vec![("val".into(), Expr::Value(Value::Str("updated".into())))],
        from: None,
        where_clause: None,
        returning: None,
        ctes: None,
        conflict_resolution: None,
        order_by: None,
        limit: Some(10),
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM t WHERE val = 'updated'", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 2);
}

#[test]
fn update_partition_ignored() {
    let (_node, mut client) = connect();
    client
        .execute("CREATE TABLE t (id SERIAL PRIMARY KEY, val TEXT)", &[])
        .unwrap();
    client
        .execute("INSERT INTO t (val) VALUES ('old')", &[])
        .unwrap();

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("t"),
        assignments: vec![("val".into(), Expr::Value(Value::Str("new".into())))],
        from: None,
        where_clause: None,
        returning: None,
        ctes: None,
        conflict_resolution: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: Some(vec!["p1".into()]),
        ignore: false,
    });
    client.execute(&render(&stmt), &[]).unwrap();

    let val: String = client
        .query_one("SELECT val FROM t", &[])
        .unwrap()
        .get(0);
    assert_eq!(val, "new");
}

#[test]
fn update_ignore_flag_ignored() {
    let (_node, mut client) = connect();
    client
        .execute("CREATE TABLE t (id SERIAL PRIMARY KEY, val TEXT)", &[])
        .unwrap();
    client
        .execute("INSERT INTO t (val) VALUES ('old')", &[])
        .unwrap();

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("t"),
        assignments: vec![("val".into(), Expr::Value(Value::Str("new".into())))],
        from: None,
        where_clause: None,
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
    client.execute(&render(&stmt), &[]).unwrap();

    let val: String = client
        .query_one("SELECT val FROM t", &[])
        .unwrap()
        .get(0);
    assert_eq!(val, "new");
}

// ==========================================================================
// DELETE — ignored SQLite/MySQL features
// ==========================================================================

#[test]
fn delete_order_by_ignored() {
    let (_node, mut client) = connect();
    client
        .execute("CREATE TABLE t (id SERIAL PRIMARY KEY, val TEXT)", &[])
        .unwrap();
    client
        .execute("INSERT INTO t (val) VALUES ('a'), ('b')", &[])
        .unwrap();

    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("t"),
        using: None,
        where_clause: None,
        returning: None,
        ctes: None,
        order_by: Some(vec![OrderByDef {
            expr: Expr::Raw {
                sql: "\"id\"".into(),
                params: vec![],
            },
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM t", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
}

#[test]
fn delete_limit_ignored() {
    let (_node, mut client) = connect();
    client
        .execute("CREATE TABLE t (id SERIAL PRIMARY KEY, val TEXT)", &[])
        .unwrap();
    client
        .execute("INSERT INTO t (val) VALUES ('a'), ('b')", &[])
        .unwrap();

    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("t"),
        using: None,
        where_clause: None,
        returning: None,
        ctes: None,
        order_by: None,
        limit: Some(10),
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM t", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
}

#[test]
fn delete_partition_ignored() {
    let (_node, mut client) = connect();
    client
        .execute("CREATE TABLE t (id SERIAL PRIMARY KEY, val TEXT)", &[])
        .unwrap();
    client
        .execute("INSERT INTO t (val) VALUES ('a')", &[])
        .unwrap();

    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("t"),
        using: None,
        where_clause: None,
        returning: None,
        ctes: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: Some(vec!["p1".into()]),
        ignore: false,
    });
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM t", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
}

#[test]
fn delete_ignore_flag_ignored() {
    let (_node, mut client) = connect();
    client
        .execute("CREATE TABLE t (id SERIAL PRIMARY KEY, val TEXT)", &[])
        .unwrap();
    client
        .execute("INSERT INTO t (val) VALUES ('a')", &[])
        .unwrap();

    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("t"),
        using: None,
        where_clause: None,
        returning: None,
        ctes: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: true,
    });
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM t", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
}
