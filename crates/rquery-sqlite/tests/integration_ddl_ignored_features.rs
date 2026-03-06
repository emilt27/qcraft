//! Tests that verify SQLite silently ignores unsupported DDL features
//! while still producing valid, executable SQL.

use rquery_core::ast::common::{OrderDir, SchemaRef};
use rquery_core::ast::ddl::*;
use rquery_sqlite::SqliteRenderer;
use rusqlite::Connection;

fn conn() -> Connection {
    Connection::open_in_memory().unwrap()
}

fn render(stmt: &SchemaMutationStmt) -> String {
    let renderer = SqliteRenderer::new();
    let (sql, _) = renderer.render_schema_stmt(stmt).unwrap();
    sql
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
        [table],
        |row| row.get::<_, i32>(0),
    )
    .unwrap()
        > 0
}

// ==========================================================================
// CREATE TABLE — ignored features
// ==========================================================================

#[test]
fn unlogged_ignored() {
    let db = conn();
    let mut schema = SchemaDef::new("t");
    schema.columns = vec![ColumnDef::new("id", FieldType::scalar("INTEGER"))];
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: false,
        temporary: false,
        unlogged: true,
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
    db.execute(&render(&stmt), []).unwrap();
    assert!(table_exists(&db, "t"));
}

#[test]
fn tablespace_ignored() {
    let db = conn();
    let mut schema = SchemaDef::new("t");
    schema.columns = vec![ColumnDef::new("id", FieldType::scalar("INTEGER"))];
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: false,
        temporary: false,
        unlogged: false,
        tablespace: Some("fast_disk".into()),
        partition_by: None,
        inherits: None,
        using_method: None,
        with_options: None,
        on_commit: None,
        table_options: None,
        without_rowid: false,
        strict: false,
    };
    db.execute(&render(&stmt), []).unwrap();
    assert!(table_exists(&db, "t"));
}

#[test]
fn unlogged_and_tablespace_together_ignored() {
    let db = conn();
    let mut schema = SchemaDef::new("t");
    schema.columns = vec![ColumnDef::new("id", FieldType::scalar("INTEGER"))];
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: false,
        temporary: false,
        unlogged: true,
        tablespace: Some("ssd_storage".into()),
        partition_by: None,
        inherits: None,
        using_method: None,
        with_options: None,
        on_commit: None,
        table_options: None,
        without_rowid: false,
        strict: false,
    };
    db.execute(&render(&stmt), []).unwrap();
    assert!(table_exists(&db, "t"));
}

// ==========================================================================
// DROP TABLE — cascade ignored
// ==========================================================================

#[test]
fn drop_table_cascade_ignored() {
    let db = conn();
    db.execute("CREATE TABLE \"t\" (\"id\" INTEGER)", [])
        .unwrap();

    let stmt = SchemaMutationStmt::DropTable {
        schema_ref: SchemaRef::new("t"),
        if_exists: false,
        cascade: true,
    };
    db.execute(&render(&stmt), []).unwrap();
    assert!(!table_exists(&db, "t"));
}

// ==========================================================================
// CREATE INDEX — ignored features
// ==========================================================================

#[test]
fn index_concurrently_ignored() {
    let db = conn();
    db.execute("CREATE TABLE \"t\" (\"a\" INTEGER)", [])
        .unwrap();

    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("t"),
        index: IndexDef::new(
            "idx_a",
            vec![IndexColumnDef {
                expr: IndexExpr::Column("a".into()),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            }],
        ),
        if_not_exists: false,
        concurrently: true,
    };
    db.execute(&render(&stmt), []).unwrap();

    // Index was created despite CONCURRENTLY
    let count: i32 = db
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_a'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn index_type_ignored() {
    let db = conn();
    db.execute("CREATE TABLE \"t\" (\"a\" TEXT)", []).unwrap();

    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("t"),
        index: IndexDef {
            name: "idx_a".into(),
            columns: vec![IndexColumnDef {
                expr: IndexExpr::Column("a".into()),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            }],
            unique: false,
            index_type: Some("GIN".into()),
            include: None,
            condition: None,
            parameters: None,
            tablespace: None,
            nulls_distinct: None,
        },
        if_not_exists: false,
        concurrently: false,
    };
    db.execute(&render(&stmt), []).unwrap();
}

#[test]
fn index_include_ignored() {
    let db = conn();
    db.execute("CREATE TABLE \"t\" (\"a\" INTEGER, \"b\" TEXT)", [])
        .unwrap();

    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("t"),
        index: IndexDef {
            name: "idx_a".into(),
            columns: vec![IndexColumnDef {
                expr: IndexExpr::Column("a".into()),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            }],
            unique: false,
            index_type: None,
            include: Some(vec!["b".into()]),
            condition: None,
            parameters: None,
            tablespace: None,
            nulls_distinct: None,
        },
        if_not_exists: false,
        concurrently: false,
    };
    db.execute(&render(&stmt), []).unwrap();
}

#[test]
fn index_parameters_and_tablespace_ignored() {
    let db = conn();
    db.execute("CREATE TABLE \"t\" (\"a\" INTEGER)", [])
        .unwrap();

    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("t"),
        index: IndexDef {
            name: "idx_a".into(),
            columns: vec![IndexColumnDef {
                expr: IndexExpr::Column("a".into()),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            }],
            unique: false,
            index_type: None,
            include: None,
            condition: None,
            parameters: Some(vec![("fillfactor".into(), "70".into())]),
            tablespace: Some("fast".into()),
            nulls_distinct: None,
        },
        if_not_exists: false,
        concurrently: false,
    };
    db.execute(&render(&stmt), []).unwrap();
}

#[test]
fn index_opclass_ignored() {
    let db = conn();
    db.execute("CREATE TABLE \"t\" (\"a\" TEXT)", []).unwrap();

    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("t"),
        index: IndexDef::new(
            "idx_a",
            vec![IndexColumnDef {
                expr: IndexExpr::Column("a".into()),
                direction: Some(OrderDir::Asc),
                nulls: None,
                opclass: Some("text_pattern_ops".into()),
                collation: None,
            }],
        ),
        if_not_exists: false,
        concurrently: false,
    };
    db.execute(&render(&stmt), []).unwrap();
}

#[test]
fn index_all_pg_features_ignored_together() {
    let db = conn();
    db.execute("CREATE TABLE \"t\" (\"a\" INTEGER, \"b\" TEXT)", [])
        .unwrap();

    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("t"),
        index: IndexDef {
            name: "idx_full".into(),
            columns: vec![IndexColumnDef {
                expr: IndexExpr::Column("a".into()),
                direction: Some(OrderDir::Desc),
                nulls: None,
                opclass: Some("int4_ops".into()),
                collation: None,
            }],
            unique: true,
            index_type: Some("btree".into()),
            include: Some(vec!["b".into()]),
            condition: None,
            parameters: Some(vec![("fillfactor".into(), "90".into())]),
            tablespace: Some("fast".into()),
            nulls_distinct: Some(false),
        },
        if_not_exists: true,
        concurrently: true,
    };
    db.execute(&render(&stmt), []).unwrap();

    // Verify it's still a valid unique index
    db.execute("INSERT INTO \"t\" VALUES (1, 'a')", []).unwrap();
    let err = db.execute("INSERT INTO \"t\" VALUES (1, 'b')", []);
    assert!(err.is_err(), "unique constraint should still work");
}

// ==========================================================================
// DROP INDEX — concurrently and cascade ignored
// ==========================================================================

#[test]
fn drop_index_concurrently_and_cascade_ignored() {
    let db = conn();
    db.execute("CREATE TABLE \"t\" (\"a\" INTEGER)", [])
        .unwrap();
    db.execute("CREATE INDEX \"idx_a\" ON \"t\" (\"a\")", [])
        .unwrap();

    let stmt = SchemaMutationStmt::DropIndex {
        schema_ref: SchemaRef::new("t"),
        index_name: "idx_a".into(),
        if_exists: false,
        concurrently: true,
        cascade: true,
    };
    db.execute(&render(&stmt), []).unwrap();

    let count: i32 = db
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_a'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}
