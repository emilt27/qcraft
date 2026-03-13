//! Integration tests verifying that PG renderer silently ignores
//! features from other database dialects (e.g. SQLite index hints, TOP).

use std::sync::LazyLock;
use std::sync::atomic::{AtomicU32, Ordering};

use postgres::{Client, NoTls};
use testcontainers::ImageExt;
use testcontainers::runners::SyncRunner;
use testcontainers_modules::postgres::Postgres;

mod common;

use qcraft_core::ast::common::*;
use qcraft_core::ast::query::*;
use qcraft_core::ast::value::Value;
use qcraft_postgres::PostgresRenderer;

fn render(stmt: &QueryStmt) -> (String, Vec<Value>) {
    let renderer = PostgresRenderer::new();
    renderer.render_query_stmt(stmt).unwrap()
}

fn simple_query() -> QueryStmt {
    QueryStmt {
        ctes: None,
        columns: vec![],
        distinct: None,
        from: None,
        joins: None,
        where_clause: None,
        group_by: None,
        having: None,
        window: None,
        order_by: None,
        limit: None,
        lock: None,
    }
}

struct TestDb {
    host: String,
    port: u16,
    _container: Box<dyn std::any::Any + Send + Sync>,
}

static TEST_DB: LazyLock<TestDb> = LazyLock::new(|| {
    let node = Postgres::default().with_tag("16-alpine").start().unwrap();
    let host = node.get_host().unwrap().to_string();
    let port = node.get_host_port_ipv4(5432).unwrap();

    let conn_str =
        format!("host={host} port={port} user=postgres password=postgres dbname=postgres");
    let mut client = Client::connect(&conn_str, NoTls).unwrap();

    client
        .batch_execute("CREATE DATABASE template_ignored TEMPLATE template0")
        .unwrap();
    drop(client);

    let template_conn =
        format!("host={host} port={port} user=postgres password=postgres dbname=template_ignored");
    let mut tmpl = Client::connect(&template_conn, NoTls).unwrap();
    tmpl.batch_execute(
        "
        CREATE TABLE \"users\" (
            \"id\" INTEGER PRIMARY KEY,
            \"name\" TEXT NOT NULL,
            \"active\" BOOLEAN NOT NULL DEFAULT TRUE
        );
        INSERT INTO \"users\" VALUES (1, 'Alice', TRUE);
        INSERT INTO \"users\" VALUES (2, 'Bob', TRUE);
        INSERT INTO \"users\" VALUES (3, 'Charlie', FALSE);
    ",
    )
    .unwrap();
    drop(tmpl);

    TestDb {
        host,
        port,
        _container: Box::new(node),
    }
});

static DB_COUNTER: AtomicU32 = AtomicU32::new(0);

fn test_client() -> Client {
    let db = &*TEST_DB;
    let n = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let test_db = format!("test_ignored_{n}");

    let admin_conn = format!(
        "host={} port={} user=postgres password=postgres dbname=postgres",
        db.host, db.port
    );
    let mut admin = Client::connect(&admin_conn, NoTls).unwrap();
    admin
        .execute(
            &format!("CREATE DATABASE \"{test_db}\" TEMPLATE template_ignored"),
            &[],
        )
        .unwrap();
    drop(admin);

    let conn_str = format!(
        "host={} port={} user=postgres password=postgres dbname={test_db}",
        db.host, db.port
    );
    Client::connect(&conn_str, NoTls).unwrap()
}

// ==========================================================================
// SQLite index hint is silently ignored
// ==========================================================================

#[test]
fn index_hint_ignored() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::Table(SchemaRef::new("users")),
            only: false,
            sample: None,
            index_hint: Some(SqliteIndexHint::IndexedBy("idx_users_name".into())),
        }]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = common::to_pg_params(&values);
    let params = common::as_pg_params(&boxed);
    // The SQL should NOT contain INDEXED BY (PG ignores it)
    assert!(!sql.contains("INDEXED BY"));
    // But the query should still execute fine
    let rows = client.query(&sql, &params).unwrap();
    assert_eq!(rows.len(), 3);
}

#[test]
fn not_indexed_hint_ignored() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::Table(SchemaRef::new("users")),
            only: false,
            sample: None,
            index_hint: Some(SqliteIndexHint::NotIndexed),
        }]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = common::to_pg_params(&values);
    let params = common::as_pg_params(&boxed);
    assert!(!sql.contains("NOT INDEXED"));
    let rows = client.query(&sql, &params).unwrap();
    assert_eq!(rows.len(), 3);
}

// ==========================================================================
// TOP converts to LIMIT on PG
// ==========================================================================

#[test]
fn top_converts_to_limit() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        limit: Some(LimitDef {
            kind: LimitKind::Top {
                count: 2,
                with_ties: false,
                percent: false,
            },
            offset: None,
        }),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = common::to_pg_params(&values);
    let params = common::as_pg_params(&boxed);
    // Should render as LIMIT, not TOP
    assert!(sql.contains("LIMIT $"));
    assert!(!sql.contains("TOP"));
    let rows = client.query(&sql, &params).unwrap();
    assert_eq!(rows.len(), 2);
}
