use rusqlite::Connection;
use rquery_core::ast::common::SchemaRef;
use rquery_core::ast::conditions::{CompareOp, Comparison, ConditionNode, Conditions, Connector};
use rquery_core::ast::ddl::*;
use rquery_core::ast::expr::Expr;
use rquery_core::ast::value::Value;
use rquery_sqlite::SqliteRenderer;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn conn() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    c
}

fn render(stmt: &SchemaMutationStmt) -> String {
    let renderer = SqliteRenderer::new();
    let (sql, _) = renderer.render_schema_stmt(stmt).unwrap();
    sql
}

/// Column info from PRAGMA table_info.
#[derive(Debug)]
#[allow(dead_code)]
struct ColInfo {
    name: String,
    col_type: String,
    not_null: bool,
    default_value: Option<String>,
    pk: i32,
}

fn table_info(conn: &Connection, table: &str) -> Vec<ColInfo> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{}\")", table)).unwrap();
    stmt.query_map([], |row| {
        Ok(ColInfo {
            name: row.get(1)?,
            col_type: row.get(2)?,
            not_null: row.get::<_, i32>(3)? != 0,
            default_value: row.get(4)?,
            pk: row.get(5)?,
        })
    })
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap()
}

#[derive(Debug)]
struct IndexInfo {
    name: String,
    unique: bool,
}

fn index_list(conn: &Connection, table: &str) -> Vec<IndexInfo> {
    let mut stmt = conn.prepare(&format!("PRAGMA index_list(\"{}\")", table)).unwrap();
    stmt.query_map([], |row| {
        Ok(IndexInfo {
            name: row.get(1)?,
            unique: row.get::<_, i32>(2)? != 0,
        })
    })
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap()
}

fn index_columns(conn: &Connection, index: &str) -> Vec<String> {
    let mut stmt = conn.prepare(&format!("PRAGMA index_info(\"{}\")", index)).unwrap();
    stmt.query_map([], |row| row.get(2))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

#[derive(Debug)]
struct FkInfo {
    table: String,
    from: String,
    to: String,
    on_update: String,
    on_delete: String,
}

fn foreign_keys(conn: &Connection, table: &str) -> Vec<FkInfo> {
    let mut stmt = conn.prepare(&format!("PRAGMA foreign_key_list(\"{}\")", table)).unwrap();
    stmt.query_map([], |row| {
        Ok(FkInfo {
            table: row.get(2)?,
            from: row.get(3)?,
            to: row.get(4)?,
            on_update: row.get(5)?,
            on_delete: row.get(6)?,
        })
    })
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap()
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
// CREATE TABLE — columns and types
// ==========================================================================

#[test]
fn create_table_columns_and_types() {
    let db = conn();
    let mut schema = SchemaDef::new("users");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("INTEGER")).not_null(),
        ColumnDef::new("name", FieldType::scalar("TEXT")),
        ColumnDef::new("age", FieldType::scalar("INTEGER")),
        ColumnDef::new("score", FieldType::scalar("REAL")),
        ColumnDef::new("data", FieldType::scalar("BLOB")),
    ];
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
    db.execute(&render(&stmt), []).unwrap();

    let cols = table_info(&db, "users");
    assert_eq!(cols.len(), 5);

    assert_eq!(cols[0].name, "id");
    assert_eq!(cols[0].col_type, "INTEGER");
    assert!(cols[0].not_null);

    assert_eq!(cols[1].name, "name");
    assert_eq!(cols[1].col_type, "TEXT");
    assert!(!cols[1].not_null);

    assert_eq!(cols[2].name, "age");
    assert_eq!(cols[2].col_type, "INTEGER");

    assert_eq!(cols[3].name, "score");
    assert_eq!(cols[3].col_type, "REAL");

    assert_eq!(cols[4].name, "data");
    assert_eq!(cols[4].col_type, "BLOB");
}

#[test]
fn create_table_parameterized_types() {
    let db = conn();
    let mut schema = SchemaDef::new("data");
    schema.columns = vec![
        ColumnDef::new("amount", FieldType::parameterized("DECIMAL", vec!["10", "2"])),
        ColumnDef::new("code", FieldType::parameterized("VARCHAR", vec!["50"])),
    ];
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
    db.execute(&render(&stmt), []).unwrap();

    let cols = table_info(&db, "data");
    assert_eq!(cols[0].name, "amount");
    assert_eq!(cols[0].col_type, "DECIMAL(10, 2)");
    assert_eq!(cols[1].name, "code");
    assert_eq!(cols[1].col_type, "VARCHAR(50)");
}

#[test]
fn create_table_default_value() {
    let db = conn();
    let mut schema = SchemaDef::new("config");
    schema.columns = vec![
        ColumnDef::new("key", FieldType::scalar("TEXT")).not_null(),
        ColumnDef::new("value", FieldType::scalar("TEXT"))
            .default(Expr::Value(Value::Str("default_val".into()))),
        ColumnDef::new("count", FieldType::scalar("INTEGER"))
            .default(Expr::Value(Value::Int(0))),
    ];
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
    db.execute(&render(&stmt), []).unwrap();

    // Insert without specifying default columns
    db.execute("INSERT INTO \"config\" (\"key\") VALUES ('test')", []).unwrap();

    let (val, count): (String, i64) = db.query_row(
        "SELECT \"value\", \"count\" FROM \"config\" WHERE \"key\" = 'test'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    assert_eq!(val, "default_val");
    assert_eq!(count, 0);
}

// ==========================================================================
// CREATE TABLE — constraints
// ==========================================================================

#[test]
fn create_table_primary_key() {
    let db = conn();
    let mut schema = SchemaDef::new("users");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("INTEGER")).not_null(),
        ColumnDef::new("name", FieldType::scalar("TEXT")),
    ];
    schema.constraints = Some(vec![ConstraintDef::PrimaryKey {
        name: None,
        columns: vec!["id".into()],
        include: None,
        autoincrement: false,
    }]);
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
    db.execute(&render(&stmt), []).unwrap();

    let cols = table_info(&db, "users");
    let pk_col = cols.iter().find(|c| c.pk > 0).unwrap();
    assert_eq!(pk_col.name, "id");

    // Duplicate PK should fail
    db.execute("INSERT INTO \"users\" (\"id\", \"name\") VALUES (1, 'a')", []).unwrap();
    let err = db.execute("INSERT INTO \"users\" (\"id\", \"name\") VALUES (1, 'b')", []);
    assert!(err.is_err());
}

#[test]
fn create_table_autoincrement() {
    let db = conn();
    let mut schema = SchemaDef::new("events");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("INTEGER")).not_null(),
        ColumnDef::new("name", FieldType::scalar("TEXT")),
    ];
    schema.constraints = Some(vec![ConstraintDef::PrimaryKey {
        name: None,
        columns: vec!["id".into()],
        include: None,
        autoincrement: true,
    }]);
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
    db.execute(&render(&stmt), []).unwrap();

    db.execute("INSERT INTO \"events\" (\"name\") VALUES ('first')", []).unwrap();
    db.execute("INSERT INTO \"events\" (\"name\") VALUES ('second')", []).unwrap();

    let ids: Vec<i64> = {
        let mut s = db.prepare("SELECT \"id\" FROM \"events\" ORDER BY \"id\"").unwrap();
        s.query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };
    assert_eq!(ids, vec![1, 2]);
}

#[test]
fn create_table_unique_constraint() {
    let db = conn();
    let mut schema = SchemaDef::new("users");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("INTEGER")).not_null(),
        ColumnDef::new("email", FieldType::scalar("TEXT")).not_null(),
    ];
    schema.constraints = Some(vec![
        ConstraintDef::PrimaryKey {
            name: None,
            columns: vec!["id".into()],
            include: None,
            autoincrement: true,
        },
        ConstraintDef::Unique {
            name: None,
            columns: vec!["email".into()],
            include: None,
            nulls_distinct: None,
            condition: None,
        },
    ]);
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
    db.execute(&render(&stmt), []).unwrap();

    db.execute("INSERT INTO \"users\" (\"email\") VALUES ('a@b.com')", []).unwrap();
    let err = db.execute("INSERT INTO \"users\" (\"email\") VALUES ('a@b.com')", []);
    assert!(err.is_err(), "duplicate email should fail");
}

#[test]
fn create_table_check_constraint() {
    let db = conn();
    let mut schema = SchemaDef::new("users");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("INTEGER")).not_null(),
        ColumnDef::new("age", FieldType::scalar("INTEGER")),
    ];
    schema.constraints = Some(vec![ConstraintDef::Check {
        name: Some("age_positive".into()),
        condition: Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw { sql: "\"age\"".into(), params: vec![] },
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(0)),
                negate: false,
            })],
            connector: Connector::And,
            negated: false,
        },
        no_inherit: false,
        enforced: None,
    }]);
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
    db.execute(&render(&stmt), []).unwrap();

    db.execute("INSERT INTO \"users\" (\"id\", \"age\") VALUES (1, 25)", []).unwrap();
    let err = db.execute("INSERT INTO \"users\" (\"id\", \"age\") VALUES (2, -5)", []);
    assert!(err.is_err(), "negative age should violate CHECK");
}

#[test]
fn create_table_foreign_key() {
    let db = conn();

    // Parent table
    db.execute("CREATE TABLE \"users\" (\"id\" INTEGER PRIMARY KEY)", []).unwrap();

    // Child table via AST
    let mut schema = SchemaDef::new("posts");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("INTEGER")).not_null(),
        ColumnDef::new("user_id", FieldType::scalar("INTEGER")),
    ];
    schema.constraints = Some(vec![ConstraintDef::ForeignKey {
        name: Some("fk_user".into()),
        columns: vec!["user_id".into()],
        ref_table: SchemaRef::new("users"),
        ref_columns: vec!["id".into()],
        on_delete: Some(ReferentialAction::Cascade),
        on_update: Some(ReferentialAction::NoAction),
        deferrable: None,
        match_type: None,
    }]);
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
    db.execute(&render(&stmt), []).unwrap();

    // Verify FK metadata
    let fks = foreign_keys(&db, "posts");
    assert_eq!(fks.len(), 1);
    assert_eq!(fks[0].table, "users");
    assert_eq!(fks[0].from, "user_id");
    assert_eq!(fks[0].to, "id");
    assert_eq!(fks[0].on_delete, "CASCADE");
    assert_eq!(fks[0].on_update, "NO ACTION");

    // FK enforcement: insert with non-existent parent should fail
    let err = db.execute("INSERT INTO \"posts\" (\"id\", \"user_id\") VALUES (1, 999)", []);
    assert!(err.is_err(), "FK violation should fail");

    // Valid FK reference
    db.execute("INSERT INTO \"users\" (\"id\") VALUES (1)", []).unwrap();
    db.execute("INSERT INTO \"posts\" (\"id\", \"user_id\") VALUES (1, 1)", []).unwrap();

    // CASCADE delete: deleting parent should delete child
    db.execute("DELETE FROM \"users\" WHERE \"id\" = 1", []).unwrap();
    let count: i64 = db.query_row("SELECT COUNT(*) FROM \"posts\"", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 0, "CASCADE should delete child rows");
}

// ==========================================================================
// CREATE TABLE — special SQLite features
// ==========================================================================

#[test]
fn create_table_without_rowid() {
    let db = conn();
    let mut schema = SchemaDef::new("kv");
    schema.columns = vec![
        ColumnDef::new("key", FieldType::scalar("TEXT")).not_null(),
        ColumnDef::new("value", FieldType::scalar("BLOB")),
    ];
    schema.constraints = Some(vec![ConstraintDef::PrimaryKey {
        name: None,
        columns: vec!["key".into()],
        include: None,
        autoincrement: false,
    }]);
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
    db.execute(&render(&stmt), []).unwrap();

    // WITHOUT ROWID tables work — verify via insert/select
    db.execute("INSERT INTO \"kv\" (\"key\", \"value\") VALUES ('k1', X'CAFE')", []).unwrap();
    let val: Vec<u8> = db.query_row(
        "SELECT \"value\" FROM \"kv\" WHERE \"key\" = 'k1'",
        [],
        |r| r.get(0),
    ).unwrap();
    assert_eq!(val, vec![0xCA, 0xFE]);
}

#[test]
fn create_table_strict_mode() {
    let db = conn();
    let mut schema = SchemaDef::new("strict_t");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("INTEGER")).not_null(),
        ColumnDef::new("name", FieldType::scalar("TEXT")),
    ];
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
    db.execute(&render(&stmt), []).unwrap();

    // STRICT mode enforces types — inserting text into INTEGER should fail
    let err = db.execute("INSERT INTO \"strict_t\" (\"id\", \"name\") VALUES ('not_int', 'a')", []);
    assert!(err.is_err(), "STRICT should reject wrong types");
}

#[test]
fn create_table_if_not_exists() {
    let db = conn();
    let mut schema = SchemaDef::new("t");
    schema.columns = vec![ColumnDef::new("id", FieldType::scalar("INTEGER"))];
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: true,
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
    let sql = render(&stmt);

    // First create — ok
    db.execute(&sql, []).unwrap();
    // Second create — should not fail because IF NOT EXISTS
    db.execute(&sql, []).unwrap();
}

#[test]
fn create_table_generated_column() {
    let db = conn();
    let mut schema = SchemaDef::new("products");
    schema.columns = vec![
        ColumnDef::new("price", FieldType::scalar("REAL")),
        ColumnDef::new("qty", FieldType::scalar("INTEGER")),
        ColumnDef {
            name: "total".into(),
            field_type: FieldType::scalar("REAL"),
            not_null: false,
            default: None,
            generated: Some(GeneratedColumn {
                expr: Expr::Raw { sql: "price * qty".into(), params: vec![] },
                stored: true,
            }),
            identity: None,
            collation: None,
            comment: None,
            storage: None,
            compression: None,
        },
    ];
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
    db.execute(&render(&stmt), []).unwrap();

    db.execute("INSERT INTO \"products\" (\"price\", \"qty\") VALUES (10.5, 3)", []).unwrap();

    let total: f64 = db.query_row(
        "SELECT \"total\" FROM \"products\"",
        [],
        |r| r.get(0),
    ).unwrap();
    assert!((total - 31.5).abs() < f64::EPSILON);
}

// ==========================================================================
// CREATE INDEX
// ==========================================================================

#[test]
fn create_index_and_verify() {
    let db = conn();
    db.execute("CREATE TABLE \"users\" (\"id\" INTEGER, \"email\" TEXT, \"name\" TEXT)", []).unwrap();

    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("users"),
        index: IndexDef::new("idx_email", vec![IndexColumnDef {
            expr: IndexExpr::Column("email".into()),
            direction: None,
            nulls: None,
            opclass: None,
            collation: None,
        }]),
        if_not_exists: false,
        concurrently: false,
    };
    db.execute(&render(&stmt), []).unwrap();

    let indexes = index_list(&db, "users");
    let idx = indexes.iter().find(|i| i.name == "idx_email").unwrap();
    assert!(!idx.unique);

    let cols = index_columns(&db, "idx_email");
    assert_eq!(cols, vec!["email"]);
}

#[test]
fn create_unique_index() {
    let db = conn();
    db.execute("CREATE TABLE \"users\" (\"id\" INTEGER, \"email\" TEXT)", []).unwrap();

    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("users"),
        index: IndexDef::new("idx_email", vec![IndexColumnDef {
            expr: IndexExpr::Column("email".into()),
            direction: None,
            nulls: None,
            opclass: None,
            collation: None,
        }]).unique(),
        if_not_exists: false,
        concurrently: false,
    };
    db.execute(&render(&stmt), []).unwrap();

    let indexes = index_list(&db, "users");
    let idx = indexes.iter().find(|i| i.name == "idx_email").unwrap();
    assert!(idx.unique);

    // Unique index enforced
    db.execute("INSERT INTO \"users\" VALUES (1, 'a@b.com')", []).unwrap();
    let err = db.execute("INSERT INTO \"users\" VALUES (2, 'a@b.com')", []);
    assert!(err.is_err(), "unique index should prevent duplicates");
}

#[test]
fn create_index_multi_column() {
    let db = conn();
    db.execute("CREATE TABLE \"events\" (\"created_at\" TEXT, \"priority\" INTEGER)", []).unwrap();

    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("events"),
        index: IndexDef::new("idx_composite", vec![
            IndexColumnDef {
                expr: IndexExpr::Column("created_at".into()),
                direction: Some(rquery_core::ast::common::OrderDir::Desc),
                nulls: None,
                opclass: None,
                collation: None,
            },
            IndexColumnDef {
                expr: IndexExpr::Column("priority".into()),
                direction: Some(rquery_core::ast::common::OrderDir::Asc),
                nulls: None,
                opclass: None,
                collation: None,
            },
        ]),
        if_not_exists: false,
        concurrently: false,
    };
    db.execute(&render(&stmt), []).unwrap();

    let cols = index_columns(&db, "idx_composite");
    assert_eq!(cols, vec!["created_at", "priority"]);
}

#[test]
fn create_index_partial_with_where() {
    let db = conn();
    db.execute("CREATE TABLE \"users\" (\"email\" TEXT, \"active\" INTEGER)", []).unwrap();

    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("users"),
        index: IndexDef {
            name: "idx_active_email".into(),
            columns: vec![IndexColumnDef {
                expr: IndexExpr::Column("email".into()),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            }],
            unique: true,
            index_type: None,
            include: None,
            condition: Some(Conditions {
                children: vec![ConditionNode::Comparison(Comparison {
                    left: Expr::Raw { sql: "\"active\"".into(), params: vec![] },
                    op: CompareOp::Eq,
                    right: Expr::Value(Value::Bool(true)),
                    negate: false,
                })],
                connector: Connector::And,
                negated: false,
            }),
            parameters: None,
            tablespace: None,
            nulls_distinct: None,
        },
        if_not_exists: false,
        concurrently: false,
    };
    db.execute(&render(&stmt), []).unwrap();

    // Partial unique: two inactive rows with same email — ok
    db.execute("INSERT INTO \"users\" VALUES ('a@b.com', 0)", []).unwrap();
    db.execute("INSERT INTO \"users\" VALUES ('a@b.com', 0)", []).unwrap();

    // Two active rows with same email — fails
    db.execute("INSERT INTO \"users\" VALUES ('x@y.com', 1)", []).unwrap();
    let err = db.execute("INSERT INTO \"users\" VALUES ('x@y.com', 1)", []);
    assert!(err.is_err(), "partial unique index should prevent active duplicates");
}

#[test]
fn create_index_expression() {
    let db = conn();
    db.execute("CREATE TABLE \"users\" (\"email\" TEXT)", []).unwrap();

    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("users"),
        index: IndexDef::new("idx_lower_email", vec![IndexColumnDef {
            expr: IndexExpr::Expression(Expr::Func {
                name: "lower".into(),
                args: vec![Expr::Raw { sql: "\"email\"".into(), params: vec![] }],
            }),
            direction: None,
            nulls: None,
            opclass: None,
            collation: None,
        }]),
        if_not_exists: false,
        concurrently: false,
    };
    db.execute(&render(&stmt), []).unwrap();

    // Expression index works — insert and query by lower
    db.execute("INSERT INTO \"users\" VALUES ('Hello@World.COM')", []).unwrap();
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM \"users\" WHERE lower(\"email\") = 'hello@world.com'",
        [],
        |r| r.get(0),
    ).unwrap();
    assert_eq!(count, 1);
}

// ==========================================================================
// DROP TABLE / DROP INDEX
// ==========================================================================

#[test]
fn drop_table() {
    let db = conn();
    db.execute("CREATE TABLE \"users\" (\"id\" INTEGER)", []).unwrap();
    assert!(table_exists(&db, "users"));

    let stmt = SchemaMutationStmt::DropTable {
        schema_ref: SchemaRef::new("users"),
        if_exists: false,
        cascade: false,
    };
    db.execute(&render(&stmt), []).unwrap();
    assert!(!table_exists(&db, "users"));
}

#[test]
fn drop_table_if_exists() {
    let db = conn();
    let stmt = SchemaMutationStmt::DropTable {
        schema_ref: SchemaRef::new("nonexistent"),
        if_exists: true,
        cascade: false,
    };
    // Should not fail
    db.execute(&render(&stmt), []).unwrap();
}

#[test]
fn drop_index() {
    let db = conn();
    db.execute("CREATE TABLE \"t\" (\"a\" INTEGER)", []).unwrap();
    db.execute("CREATE INDEX \"idx_a\" ON \"t\" (\"a\")", []).unwrap();

    let stmt = SchemaMutationStmt::DropIndex {
        schema_ref: SchemaRef::new("t"),
        index_name: "idx_a".into(),
        if_exists: false,
        concurrently: false,
        cascade: false,
    };
    db.execute(&render(&stmt), []).unwrap();

    let indexes = index_list(&db, "t");
    assert!(indexes.iter().all(|i| i.name != "idx_a"));
}

// ==========================================================================
// ALTER TABLE
// ==========================================================================

#[test]
fn alter_table_rename() {
    let db = conn();
    db.execute("CREATE TABLE \"old_name\" (\"id\" INTEGER)", []).unwrap();

    let stmt = SchemaMutationStmt::RenameTable {
        schema_ref: SchemaRef::new("old_name"),
        new_name: "new_name".into(),
    };
    db.execute(&render(&stmt), []).unwrap();

    assert!(!table_exists(&db, "old_name"));
    assert!(table_exists(&db, "new_name"));
}

#[test]
fn alter_table_rename_column() {
    let db = conn();
    db.execute("CREATE TABLE \"users\" (\"name\" TEXT)", []).unwrap();

    let stmt = SchemaMutationStmt::RenameColumn {
        schema_ref: SchemaRef::new("users"),
        old_name: "name".into(),
        new_name: "full_name".into(),
    };
    db.execute(&render(&stmt), []).unwrap();

    let cols = table_info(&db, "users");
    assert_eq!(cols[0].name, "full_name");
}

#[test]
fn alter_table_add_column() {
    let db = conn();
    db.execute("CREATE TABLE \"users\" (\"id\" INTEGER)", []).unwrap();

    let stmt = SchemaMutationStmt::AddColumn {
        schema_ref: SchemaRef::new("users"),
        column: ColumnDef::new("email", FieldType::scalar("TEXT")),
        if_not_exists: false,
        position: None,
    };
    db.execute(&render(&stmt), []).unwrap();

    let cols = table_info(&db, "users");
    assert_eq!(cols.len(), 2);
    assert_eq!(cols[1].name, "email");
    assert_eq!(cols[1].col_type, "TEXT");
}

#[test]
fn alter_table_drop_column() {
    let db = conn();
    db.execute("CREATE TABLE \"users\" (\"id\" INTEGER, \"old_field\" TEXT)", []).unwrap();

    let stmt = SchemaMutationStmt::DropColumn {
        schema_ref: SchemaRef::new("users"),
        name: "old_field".into(),
        if_exists: false,
        cascade: false,
    };
    db.execute(&render(&stmt), []).unwrap();

    let cols = table_info(&db, "users");
    assert_eq!(cols.len(), 1);
    assert_eq!(cols[0].name, "id");
}

// ==========================================================================
// TRUNCATE (DELETE FROM in SQLite)
// ==========================================================================

#[test]
fn truncate_table() {
    let db = conn();
    db.execute("CREATE TABLE \"users\" (\"id\" INTEGER)", []).unwrap();
    db.execute("INSERT INTO \"users\" VALUES (1)", []).unwrap();
    db.execute("INSERT INTO \"users\" VALUES (2)", []).unwrap();

    let stmt = SchemaMutationStmt::TruncateTable {
        schema_ref: SchemaRef::new("users"),
        restart_identity: false,
        cascade: false,
    };
    db.execute(&render(&stmt), []).unwrap();

    let count: i64 = db.query_row("SELECT COUNT(*) FROM \"users\"", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 0);
}
