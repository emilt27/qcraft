//! Tests that verify PostgreSQL renderer silently ignores SQLite-specific DDL features
//! while still producing valid, executable SQL.

use postgres::{Client, NoTls};
use testcontainers::ImageExt;
use testcontainers::runners::SyncRunner;
use testcontainers_modules::postgres::Postgres;

use qcraft_core::ast::ddl::*;
use qcraft_postgres::PostgresRenderer;

fn render(stmt: &SchemaMutationStmt) -> String {
    let renderer = PostgresRenderer::new();
    let (sql, _) = renderer.render_schema_stmt(stmt).unwrap();
    sql
}

// ==========================================================================
// SQLite-specific features silently ignored by PG renderer
// ==========================================================================

#[test]
fn without_rowid_ignored() {
    let node = Postgres::default().with_tag("16-alpine").start().unwrap();
    let conn_str = format!(
        "host={} port={} user=postgres password=postgres dbname=postgres",
        node.get_host().unwrap(),
        node.get_host_port_ipv4(5432).unwrap(),
    );
    let mut client = Client::connect(&conn_str, NoTls).unwrap();

    let mut schema = SchemaDef::new("t");
    schema.columns = vec![ColumnDef::new("id", FieldType::scalar("INTEGER"))];
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: false,
        temporary: false,
        unlogged: false,
        tablespace: None,
        partition_by: None,
        inherits: None,
        using_method: None,
        with_options: None,
        on_commit: None,
        table_options: None,
        without_rowid: true,
        strict: false,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    let exists: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = 't')",
            &[],
        )
        .unwrap()
        .get(0);
    assert!(exists);
}

#[test]
fn strict_ignored() {
    let node = Postgres::default().with_tag("16-alpine").start().unwrap();
    let conn_str = format!(
        "host={} port={} user=postgres password=postgres dbname=postgres",
        node.get_host().unwrap(),
        node.get_host_port_ipv4(5432).unwrap(),
    );
    let mut client = Client::connect(&conn_str, NoTls).unwrap();

    let mut schema = SchemaDef::new("t");
    schema.columns = vec![ColumnDef::new("id", FieldType::scalar("INTEGER"))];
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: false,
        temporary: false,
        unlogged: false,
        tablespace: None,
        partition_by: None,
        inherits: None,
        using_method: None,
        with_options: None,
        on_commit: None,
        table_options: None,
        without_rowid: false,
        strict: true,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    let exists: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = 't')",
            &[],
        )
        .unwrap()
        .get(0);
    assert!(exists);
}

#[test]
fn autoincrement_in_pk_ignored() {
    let node = Postgres::default().with_tag("16-alpine").start().unwrap();
    let conn_str = format!(
        "host={} port={} user=postgres password=postgres dbname=postgres",
        node.get_host().unwrap(),
        node.get_host_port_ipv4(5432).unwrap(),
    );
    let mut client = Client::connect(&conn_str, NoTls).unwrap();

    let mut schema = SchemaDef::new("events");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("SERIAL")),
        ColumnDef::new("name", FieldType::scalar("TEXT")),
    ];
    schema.constraints = Some(vec![ConstraintDef::primary_key(vec!["id"])]);
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: false,
        temporary: false,
        unlogged: false,
        tablespace: None,
        partition_by: None,
        inherits: None,
        using_method: None,
        with_options: None,
        on_commit: None,
        table_options: None,
        without_rowid: false,
        strict: false,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    // Table should work with auto-incrementing SERIAL
    client
        .execute("INSERT INTO \"events\" (\"name\") VALUES ('a')", &[])
        .unwrap();
    client
        .execute("INSERT INTO \"events\" (\"name\") VALUES ('b')", &[])
        .unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM \"events\"", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 2);
}

#[test]
fn all_sqlite_features_ignored_together() {
    let node = Postgres::default().with_tag("16-alpine").start().unwrap();
    let conn_str = format!(
        "host={} port={} user=postgres password=postgres dbname=postgres",
        node.get_host().unwrap(),
        node.get_host_port_ipv4(5432).unwrap(),
    );
    let mut client = Client::connect(&conn_str, NoTls).unwrap();

    let mut schema = SchemaDef::new("t");
    schema.columns = vec![ColumnDef::new("id", FieldType::scalar("INTEGER"))];
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: false,
        temporary: false,
        unlogged: false,
        tablespace: None,
        partition_by: None,
        inherits: None,
        using_method: None,
        with_options: None,
        on_commit: None,
        table_options: None,
        without_rowid: true,
        strict: true,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    client
        .execute("INSERT INTO \"t\" (\"id\") VALUES (42)", &[])
        .unwrap();
    let val: i32 = client
        .query_one("SELECT \"id\" FROM \"t\"", &[])
        .unwrap()
        .get(0);
    assert_eq!(val, 42);
}
