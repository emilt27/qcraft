use qcraft_core::ast::common::SchemaRef;
use qcraft_core::ast::conditions::{CompareOp, Comparison, ConditionNode, Conditions, Connector};
use qcraft_core::ast::ddl::*;
use qcraft_core::ast::expr::Expr;
use qcraft_core::ast::value::Value;
use qcraft_postgres::PostgresRenderer;

fn render(stmt: &SchemaMutationStmt) -> String {
    let renderer = PostgresRenderer::new();
    let stmts = renderer.render_schema_stmt(stmt).unwrap();
    stmts[0].0.clone()
}

// ==========================================================================
// CREATE TABLE — columns and types
// ==========================================================================

#[test]
fn create_table_columns_and_types() {
    let mut client = crate::test_client("template0");

    let mut schema = SchemaDef::new("users");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("BIGINT")).not_null(),
        ColumnDef::new("name", FieldType::scalar("TEXT")),
        ColumnDef::new("age", FieldType::scalar("INTEGER")),
        ColumnDef::new("score", FieldType::scalar("REAL")),
        ColumnDef::new("data", FieldType::scalar("BYTEA")),
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
    client.execute(&render(&stmt), &[]).unwrap();

    let rows = client
        .query(
            "SELECT column_name, data_type, is_nullable \
             FROM information_schema.columns \
             WHERE table_name = 'users' ORDER BY ordinal_position",
            &[],
        )
        .unwrap();

    assert_eq!(rows.len(), 5);

    let col_name: &str = rows[0].get("column_name");
    let data_type: &str = rows[0].get("data_type");
    let is_nullable: &str = rows[0].get("is_nullable");
    assert_eq!(col_name, "id");
    assert_eq!(data_type, "bigint");
    assert_eq!(is_nullable, "NO");

    let col_name: &str = rows[1].get("column_name");
    let data_type: &str = rows[1].get("data_type");
    let is_nullable: &str = rows[1].get("is_nullable");
    assert_eq!(col_name, "name");
    assert_eq!(data_type, "text");
    assert_eq!(is_nullable, "YES");

    let col_name: &str = rows[2].get("column_name");
    let data_type: &str = rows[2].get("data_type");
    assert_eq!(col_name, "age");
    assert_eq!(data_type, "integer");

    let col_name: &str = rows[3].get("column_name");
    let data_type: &str = rows[3].get("data_type");
    assert_eq!(col_name, "score");
    assert_eq!(data_type, "real");

    let col_name: &str = rows[4].get("column_name");
    let data_type: &str = rows[4].get("data_type");
    assert_eq!(col_name, "data");
    assert_eq!(data_type, "bytea");
}

// ==========================================================================
// CREATE TABLE — parameterized types
// ==========================================================================

#[test]
fn create_table_parameterized_types() {
    let mut client = crate::test_client("template0");

    let mut schema = SchemaDef::new("data");
    schema.columns = vec![
        ColumnDef::new("code", FieldType::parameterized("VARCHAR", vec!["255"])),
        ColumnDef::new(
            "amount",
            FieldType::parameterized("NUMERIC", vec!["10", "2"]),
        ),
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
    client.execute(&render(&stmt), &[]).unwrap();

    let row = client
        .query_one(
            "SELECT character_maximum_length FROM information_schema.columns \
             WHERE table_name = 'data' AND column_name = 'code'",
            &[],
        )
        .unwrap();
    let max_len: i32 = row.get("character_maximum_length");
    assert_eq!(max_len, 255);

    let row = client
        .query_one(
            "SELECT numeric_precision, numeric_scale FROM information_schema.columns \
             WHERE table_name = 'data' AND column_name = 'amount'",
            &[],
        )
        .unwrap();
    let precision: i32 = row.get("numeric_precision");
    let scale: i32 = row.get("numeric_scale");
    assert_eq!(precision, 10);
    assert_eq!(scale, 2);
}

// ==========================================================================
// CREATE TABLE — default value
// ==========================================================================

#[test]
fn create_table_default_value() {
    let mut client = crate::test_client("template0");

    let mut schema = SchemaDef::new("config");
    schema.columns = vec![
        ColumnDef::new("key", FieldType::scalar("TEXT")).not_null(),
        ColumnDef::new("value", FieldType::scalar("TEXT"))
            .default(Expr::Value(Value::Str("default_val".into()))),
        ColumnDef::new("count", FieldType::scalar("INTEGER")).default(Expr::Value(Value::Int(0))),
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
    client.execute(&render(&stmt), &[]).unwrap();

    client
        .execute("INSERT INTO \"config\" (\"key\") VALUES ('test')", &[])
        .unwrap();

    let row = client
        .query_one(
            "SELECT \"value\", \"count\" FROM \"config\" WHERE \"key\" = 'test'",
            &[],
        )
        .unwrap();
    let val: &str = row.get(0);
    let count: i32 = row.get(1);
    assert_eq!(val, "default_val");
    assert_eq!(count, 0);
}

// ==========================================================================
// CREATE TABLE — primary key
// ==========================================================================

#[test]
fn create_table_primary_key() {
    let mut client = crate::test_client("template0");

    let mut schema = SchemaDef::new("users");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("BIGINT")).not_null(),
        ColumnDef::new("name", FieldType::scalar("TEXT")),
    ];
    schema.constraints = Some(vec![ConstraintDef::PrimaryKey {
        name: Some("pk_users".into()),
        columns: vec!["id".into()],
        include: None,
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
    client.execute(&render(&stmt), &[]).unwrap();

    client
        .execute(
            "INSERT INTO \"users\" (\"id\", \"name\") VALUES (1, 'Alice')",
            &[],
        )
        .unwrap();

    let err = client.execute(
        "INSERT INTO \"users\" (\"id\", \"name\") VALUES (1, 'Bob')",
        &[],
    );
    assert!(err.is_err(), "duplicate PK should fail");
}

// ==========================================================================
// CREATE TABLE — unique constraint
// ==========================================================================

#[test]
fn create_table_unique_constraint() {
    let mut client = crate::test_client("template0");

    let mut schema = SchemaDef::new("users");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("BIGINT")).not_null(),
        ColumnDef::new("email", FieldType::scalar("TEXT")).not_null(),
    ];
    schema.constraints = Some(vec![
        ConstraintDef::PrimaryKey {
            name: Some("pk_users".into()),
            columns: vec!["id".into()],
            include: None,
        },
        ConstraintDef::Unique {
            name: Some("uq_email".into()),
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
    client.execute(&render(&stmt), &[]).unwrap();

    client
        .execute(
            "INSERT INTO \"users\" (\"id\", \"email\") VALUES (1, 'a@b.com')",
            &[],
        )
        .unwrap();

    let err = client.execute(
        "INSERT INTO \"users\" (\"id\", \"email\") VALUES (2, 'a@b.com')",
        &[],
    );
    assert!(err.is_err(), "duplicate email should fail");
}

// ==========================================================================
// CREATE TABLE — check constraint
// ==========================================================================

#[test]
fn create_table_check_constraint() {
    let mut client = crate::test_client("template0");

    let mut schema = SchemaDef::new("users");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("BIGINT")).not_null(),
        ColumnDef::new("age", FieldType::scalar("INTEGER")),
    ];
    schema.constraints = Some(vec![
        ConstraintDef::PrimaryKey {
            name: Some("pk_users".into()),
            columns: vec!["id".into()],
            include: None,
        },
        ConstraintDef::Check {
            name: Some("ck_age_positive".into()),
            condition: Conditions {
                children: vec![ConditionNode::Comparison(Box::new(Comparison {
                    left: Expr::Raw {
                        sql: "\"age\"".into(),
                        params: vec![],
                    },
                    op: CompareOp::Gt,
                    right: Expr::Value(Value::Int(0)),
                    negate: false,
                }))],
                connector: Connector::And,
                negated: false,
            },
            no_inherit: false,
            enforced: None,
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
    client.execute(&render(&stmt), &[]).unwrap();

    client
        .execute(
            "INSERT INTO \"users\" (\"id\", \"age\") VALUES (1, 25)",
            &[],
        )
        .unwrap();

    let err = client.execute(
        "INSERT INTO \"users\" (\"id\", \"age\") VALUES (2, -5)",
        &[],
    );
    assert!(err.is_err(), "negative age should violate CHECK");
}

// ==========================================================================
// CREATE TABLE — foreign key with CASCADE
// ==========================================================================

#[test]
fn create_table_foreign_key() {
    let mut client = crate::test_client("template0");

    // Create parent table directly
    client
        .execute("CREATE TABLE \"users\" (\"id\" BIGINT PRIMARY KEY)", &[])
        .unwrap();

    // Create child table via AST
    let mut schema = SchemaDef::new("posts");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("BIGINT")).not_null(),
        ColumnDef::new("user_id", FieldType::scalar("BIGINT")),
    ];
    schema.constraints = Some(vec![
        ConstraintDef::PrimaryKey {
            name: Some("pk_posts".into()),
            columns: vec!["id".into()],
            include: None,
        },
        ConstraintDef::ForeignKey {
            name: Some("fk_posts_user".into()),
            columns: vec!["user_id".into()],
            ref_table: SchemaRef::new("users"),
            ref_columns: vec!["id".into()],
            on_delete: Some(ReferentialAction::Cascade),
            on_update: Some(ReferentialAction::NoAction),
            deferrable: None,
            match_type: None,
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
    client.execute(&render(&stmt), &[]).unwrap();

    // FK violation should fail
    let err = client.execute(
        "INSERT INTO \"posts\" (\"id\", \"user_id\") VALUES (1, 999)",
        &[],
    );
    assert!(err.is_err(), "FK violation should fail");

    // Valid FK reference
    client
        .execute("INSERT INTO \"users\" (\"id\") VALUES (1)", &[])
        .unwrap();
    client
        .execute(
            "INSERT INTO \"posts\" (\"id\", \"user_id\") VALUES (1, 1)",
            &[],
        )
        .unwrap();

    // CASCADE delete: deleting parent should delete child
    client
        .execute("DELETE FROM \"users\" WHERE \"id\" = 1", &[])
        .unwrap();

    let row = client
        .query_one("SELECT COUNT(*) FROM \"posts\"", &[])
        .unwrap();
    let count: i64 = row.get(0);
    assert_eq!(count, 0, "CASCADE should delete child rows");
}

// ==========================================================================
// CREATE TABLE — identity column
// ==========================================================================

#[test]
fn create_table_with_identity() {
    let mut client = crate::test_client("template0");

    let mut schema = SchemaDef::new("items");
    schema.columns = vec![
        ColumnDef {
            name: "id".into(),
            field_type: FieldType::scalar("BIGINT"),
            not_null: true,
            default: None,
            generated: None,
            identity: Some(IdentityColumn {
                always: true,
                start: Some(1),
                increment: Some(1),
                ..Default::default()
            }),
            collation: None,
            comment: None,
            storage: None,
            compression: None,
        },
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
        strict: false,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    client
        .execute("INSERT INTO \"items\" (\"name\") VALUES ('first')", &[])
        .unwrap();
    client
        .execute("INSERT INTO \"items\" (\"name\") VALUES ('second')", &[])
        .unwrap();

    let rows = client
        .query("SELECT \"id\" FROM \"items\" ORDER BY \"id\"", &[])
        .unwrap();
    let id1: i64 = rows[0].get(0);
    let id2: i64 = rows[1].get(0);
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

// ==========================================================================
// CREATE TABLE — generated column
// ==========================================================================

#[test]
fn create_table_with_generated_column() {
    let mut client = crate::test_client("template0");

    let mut schema = SchemaDef::new("products");
    schema.columns = vec![
        ColumnDef::new("price", FieldType::scalar("NUMERIC")),
        ColumnDef::new("qty", FieldType::scalar("INTEGER")),
        ColumnDef {
            name: "total".into(),
            field_type: FieldType::scalar("NUMERIC"),
            not_null: false,
            default: None,
            generated: Some(GeneratedColumn {
                expr: Expr::Raw {
                    sql: "price * qty".into(),
                    params: vec![],
                },
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
    client.execute(&render(&stmt), &[]).unwrap();

    client
        .execute(
            "INSERT INTO \"products\" (\"price\", \"qty\") VALUES (10.5, 3)",
            &[],
        )
        .unwrap();

    let row = client
        .query_one("SELECT \"total\"::TEXT FROM \"products\"", &[])
        .unwrap();
    let total: &str = row.get(0);
    assert_eq!(total, "31.5");
}

// ==========================================================================
// CREATE TABLE IF NOT EXISTS
// ==========================================================================

#[test]
fn create_table_if_not_exists() {
    let mut client = crate::test_client("template0");

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
    client.execute(&sql, &[]).unwrap();
    // Second create — should not fail because IF NOT EXISTS
    client.execute(&sql, &[]).unwrap();
}

// ==========================================================================
// CREATE INDEX — basic
// ==========================================================================

#[test]
fn create_index_and_verify() {
    let mut client = crate::test_client("template0");

    client
        .execute(
            "CREATE TABLE \"users\" (\"id\" BIGINT, \"email\" TEXT, \"name\" TEXT)",
            &[],
        )
        .unwrap();

    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("users"),
        index: IndexDef::new(
            "idx_email",
            vec![IndexColumnDef {
                expr: IndexExpr::Column("email".into()),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            }],
        ),
        if_not_exists: false,
        concurrently: false,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    let row = client
        .query_one(
            "SELECT indexname FROM pg_indexes WHERE tablename = 'users' AND indexname = 'idx_email'",
            &[],
        )
        .unwrap();
    let idx_name: &str = row.get("indexname");
    assert_eq!(idx_name, "idx_email");
}

// ==========================================================================
// CREATE INDEX — unique
// ==========================================================================

#[test]
fn create_unique_index() {
    let mut client = crate::test_client("template0");

    client
        .execute(
            "CREATE TABLE \"users\" (\"id\" BIGINT, \"email\" TEXT)",
            &[],
        )
        .unwrap();

    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("users"),
        index: IndexDef::new(
            "idx_email",
            vec![IndexColumnDef {
                expr: IndexExpr::Column("email".into()),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            }],
        )
        .unique(),
        if_not_exists: false,
        concurrently: false,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    client
        .execute("INSERT INTO \"users\" VALUES (1, 'a@b.com')", &[])
        .unwrap();

    let err = client.execute("INSERT INTO \"users\" VALUES (2, 'a@b.com')", &[]);
    assert!(err.is_err(), "unique index should prevent duplicates");
}

// ==========================================================================
// CREATE INDEX — multi-column
// ==========================================================================

#[test]
fn create_index_multi_column() {
    let mut client = crate::test_client("template0");

    client
        .execute(
            "CREATE TABLE \"events\" (\"created_at\" TEXT, \"priority\" INTEGER)",
            &[],
        )
        .unwrap();

    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("events"),
        index: IndexDef::new(
            "idx_composite",
            vec![
                IndexColumnDef {
                    expr: IndexExpr::Column("created_at".into()),
                    direction: Some(qcraft_core::ast::common::OrderDir::Desc),
                    nulls: None,
                    opclass: None,
                    collation: None,
                },
                IndexColumnDef {
                    expr: IndexExpr::Column("priority".into()),
                    direction: Some(qcraft_core::ast::common::OrderDir::Asc),
                    nulls: None,
                    opclass: None,
                    collation: None,
                },
            ],
        ),
        if_not_exists: false,
        concurrently: false,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    // Verify the index exists and its definition contains both columns
    let row = client
        .query_one(
            "SELECT indexdef FROM pg_indexes WHERE indexname = 'idx_composite'",
            &[],
        )
        .unwrap();
    let indexdef: &str = row.get("indexdef");
    assert!(
        indexdef.contains("created_at"),
        "index should contain created_at"
    );
    assert!(
        indexdef.contains("priority"),
        "index should contain priority"
    );
}

// ==========================================================================
// CREATE INDEX — partial with WHERE
// ==========================================================================

#[test]
fn create_index_partial_with_where() {
    let mut client = crate::test_client("template0");

    client
        .execute(
            "CREATE TABLE \"users\" (\"email\" TEXT, \"active\" BOOLEAN)",
            &[],
        )
        .unwrap();

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
                children: vec![ConditionNode::Comparison(Box::new(Comparison {
                    left: Expr::Raw {
                        sql: "\"active\"".into(),
                        params: vec![],
                    },
                    op: CompareOp::Eq,
                    right: Expr::Value(Value::Bool(true)),
                    negate: false,
                }))],
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
    client.execute(&render(&stmt), &[]).unwrap();

    // Partial unique: two inactive rows with same email — ok
    client
        .execute("INSERT INTO \"users\" VALUES ('a@b.com', false)", &[])
        .unwrap();
    client
        .execute("INSERT INTO \"users\" VALUES ('a@b.com', false)", &[])
        .unwrap();

    // Two active rows with same email — fails
    client
        .execute("INSERT INTO \"users\" VALUES ('x@y.com', true)", &[])
        .unwrap();

    let err = client.execute("INSERT INTO \"users\" VALUES ('x@y.com', true)", &[]);
    assert!(
        err.is_err(),
        "partial unique index should prevent active duplicates"
    );
}

// ==========================================================================
// CREATE INDEX — expression
// ==========================================================================

#[test]
fn create_index_expression() {
    let mut client = crate::test_client("template0");

    client
        .execute("CREATE TABLE \"users\" (\"email\" TEXT)", &[])
        .unwrap();

    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("users"),
        index: IndexDef::new(
            "idx_lower_email",
            vec![IndexColumnDef {
                expr: IndexExpr::Expression(Expr::Func {
                    name: "lower".into(),
                    args: vec![Expr::Raw {
                        sql: "\"email\"".into(),
                        params: vec![],
                    }],
                }),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            }],
        ),
        if_not_exists: false,
        concurrently: false,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    // Verify the index exists
    let row = client
        .query_one(
            "SELECT indexdef FROM pg_indexes WHERE indexname = 'idx_lower_email'",
            &[],
        )
        .unwrap();
    let indexdef: &str = row.get("indexdef");
    assert!(indexdef.contains("lower"), "index should use lower()");
}

// ==========================================================================
// DROP TABLE
// ==========================================================================

#[test]
fn drop_table() {
    let mut client = crate::test_client("template0");

    client
        .execute("CREATE TABLE \"users\" (\"id\" INTEGER)", &[])
        .unwrap();

    let stmt = SchemaMutationStmt::DropTable {
        schema_ref: SchemaRef::new("users"),
        if_exists: false,
        cascade: false,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    let row = client
        .query_one(
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'users' AND table_schema = 'public'",
            &[],
        )
        .unwrap();
    let count: i64 = row.get(0);
    assert_eq!(count, 0, "table should be gone after DROP");
}

// ==========================================================================
// DROP TABLE IF EXISTS
// ==========================================================================

#[test]
fn drop_table_if_exists() {
    let mut client = crate::test_client("template0");

    let stmt = SchemaMutationStmt::DropTable {
        schema_ref: SchemaRef::new("nonexistent"),
        if_exists: true,
        cascade: false,
    };
    // Should not fail
    client.execute(&render(&stmt), &[]).unwrap();
}

// ==========================================================================
// DROP INDEX
// ==========================================================================

#[test]
fn drop_index() {
    let mut client = crate::test_client("template0");

    client
        .execute("CREATE TABLE \"t\" (\"a\" INTEGER)", &[])
        .unwrap();
    client
        .execute("CREATE INDEX \"idx_a\" ON \"t\" (\"a\")", &[])
        .unwrap();

    let stmt = SchemaMutationStmt::DropIndex {
        schema_ref: SchemaRef::new("t"),
        index_name: "idx_a".into(),
        if_exists: false,
        concurrently: false,
        cascade: false,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    let row = client
        .query_one(
            "SELECT COUNT(*) FROM pg_indexes WHERE indexname = 'idx_a'",
            &[],
        )
        .unwrap();
    let count: i64 = row.get(0);
    assert_eq!(count, 0, "index should be gone after DROP");
}

// ==========================================================================
// ALTER TABLE — rename
// ==========================================================================

#[test]
fn alter_table_rename() {
    let mut client = crate::test_client("template0");

    client
        .execute("CREATE TABLE \"old_name\" (\"id\" INTEGER)", &[])
        .unwrap();

    let stmt = SchemaMutationStmt::RenameTable {
        schema_ref: SchemaRef::new("old_name"),
        new_name: "new_name".into(),
    };
    client.execute(&render(&stmt), &[]).unwrap();

    let row = client
        .query_one(
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'old_name' AND table_schema = 'public'",
            &[],
        )
        .unwrap();
    let count: i64 = row.get(0);
    assert_eq!(count, 0, "old table name should be gone");

    let row = client
        .query_one(
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'new_name' AND table_schema = 'public'",
            &[],
        )
        .unwrap();
    let count: i64 = row.get(0);
    assert_eq!(count, 1, "new table name should exist");
}

// ==========================================================================
// ALTER TABLE — add column
// ==========================================================================

#[test]
fn alter_table_add_column() {
    let mut client = crate::test_client("template0");

    client
        .execute("CREATE TABLE \"users\" (\"id\" INTEGER)", &[])
        .unwrap();

    let stmt = SchemaMutationStmt::AddColumn {
        schema_ref: SchemaRef::new("users"),
        column: Box::new(ColumnDef::new("email", FieldType::scalar("TEXT"))),
        if_not_exists: false,
        position: None,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    let row = client
        .query_one(
            "SELECT column_name, data_type FROM information_schema.columns \
             WHERE table_name = 'users' AND column_name = 'email'",
            &[],
        )
        .unwrap();
    let col_name: &str = row.get("column_name");
    let data_type: &str = row.get("data_type");
    assert_eq!(col_name, "email");
    assert_eq!(data_type, "text");
}

// ==========================================================================
// ALTER TABLE — drop column
// ==========================================================================

#[test]
fn alter_table_drop_column() {
    let mut client = crate::test_client("template0");

    client
        .execute(
            "CREATE TABLE \"users\" (\"id\" INTEGER, \"old_field\" TEXT)",
            &[],
        )
        .unwrap();

    let stmt = SchemaMutationStmt::DropColumn {
        schema_ref: SchemaRef::new("users"),
        name: "old_field".into(),
        if_exists: false,
        cascade: false,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    let row = client
        .query_one(
            "SELECT COUNT(*) FROM information_schema.columns \
             WHERE table_name = 'users' AND column_name = 'old_field'",
            &[],
        )
        .unwrap();
    let count: i64 = row.get(0);
    assert_eq!(count, 0, "column should be gone after DROP COLUMN");
}

// ==========================================================================
// ALTER TABLE — rename column
// ==========================================================================

#[test]
fn alter_table_rename_column() {
    let mut client = crate::test_client("template0");

    client
        .execute("CREATE TABLE \"users\" (\"name\" TEXT)", &[])
        .unwrap();

    let stmt = SchemaMutationStmt::RenameColumn {
        schema_ref: SchemaRef::new("users"),
        old_name: "name".into(),
        new_name: "full_name".into(),
    };
    client.execute(&render(&stmt), &[]).unwrap();

    let row = client
        .query_one(
            "SELECT column_name FROM information_schema.columns \
             WHERE table_name = 'users'",
            &[],
        )
        .unwrap();
    let col_name: &str = row.get("column_name");
    assert_eq!(col_name, "full_name");
}

// ==========================================================================
// ALTER COLUMN — change type
// ==========================================================================

#[test]
fn alter_column_type() {
    let mut client = crate::test_client("template0");

    client
        .execute("CREATE TABLE \"users\" (\"bio\" TEXT)", &[])
        .unwrap();

    let stmt = SchemaMutationStmt::AlterColumnType {
        schema_ref: SchemaRef::new("users"),
        column_name: "bio".into(),
        new_type: FieldType::parameterized("VARCHAR", vec!["100"]),
        using_expr: None,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    let row = client
        .query_one(
            "SELECT data_type, character_maximum_length FROM information_schema.columns \
             WHERE table_name = 'users' AND column_name = 'bio'",
            &[],
        )
        .unwrap();
    let data_type: &str = row.get("data_type");
    let max_len: i32 = row.get("character_maximum_length");
    assert_eq!(data_type, "character varying");
    assert_eq!(max_len, 100);
}

// ==========================================================================
// ALTER COLUMN — set default
// ==========================================================================

#[test]
fn alter_column_set_default() {
    let mut client = crate::test_client("template0");

    client
        .execute(
            "CREATE TABLE \"users\" (\"id\" INTEGER, \"status\" TEXT)",
            &[],
        )
        .unwrap();

    let stmt = SchemaMutationStmt::AlterColumnDefault {
        schema_ref: SchemaRef::new("users"),
        column_name: "status".into(),
        default: Some(Expr::Value(Value::Str("active".into()))),
    };
    client.execute(&render(&stmt), &[]).unwrap();

    client
        .execute("INSERT INTO \"users\" (\"id\") VALUES (1)", &[])
        .unwrap();

    let row = client
        .query_one("SELECT \"status\" FROM \"users\" WHERE \"id\" = 1", &[])
        .unwrap();
    let status: &str = row.get(0);
    assert_eq!(status, "active");
}

// ==========================================================================
// ALTER COLUMN — drop default
// ==========================================================================

#[test]
fn alter_column_drop_default() {
    let mut client = crate::test_client("template0");

    client
        .execute(
            "CREATE TABLE \"users\" (\"id\" INTEGER, \"status\" TEXT DEFAULT 'active')",
            &[],
        )
        .unwrap();

    let stmt = SchemaMutationStmt::AlterColumnDefault {
        schema_ref: SchemaRef::new("users"),
        column_name: "status".into(),
        default: None,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    client
        .execute("INSERT INTO \"users\" (\"id\") VALUES (1)", &[])
        .unwrap();

    let row = client
        .query_one("SELECT \"status\" FROM \"users\" WHERE \"id\" = 1", &[])
        .unwrap();
    let status: Option<&str> = row.get(0);
    assert!(status.is_none(), "should be NULL after dropping default");
}

// ==========================================================================
// ALTER COLUMN — set NOT NULL
// ==========================================================================

#[test]
fn alter_column_set_not_null() {
    let mut client = crate::test_client("template0");

    client
        .execute(
            "CREATE TABLE \"users\" (\"id\" INTEGER, \"email\" TEXT)",
            &[],
        )
        .unwrap();

    let stmt = SchemaMutationStmt::AlterColumnNullability {
        schema_ref: SchemaRef::new("users"),
        column_name: "email".into(),
        not_null: true,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    let err = client.execute(
        "INSERT INTO \"users\" (\"id\", \"email\") VALUES (1, NULL)",
        &[],
    );
    assert!(err.is_err(), "NULL should fail after SET NOT NULL");
}

// ==========================================================================
// ALTER COLUMN — drop NOT NULL
// ==========================================================================

#[test]
fn alter_column_drop_not_null() {
    let mut client = crate::test_client("template0");

    client
        .execute(
            "CREATE TABLE \"users\" (\"id\" INTEGER, \"email\" TEXT NOT NULL)",
            &[],
        )
        .unwrap();

    let stmt = SchemaMutationStmt::AlterColumnNullability {
        schema_ref: SchemaRef::new("users"),
        column_name: "email".into(),
        not_null: false,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    // NULL insert should now succeed
    client
        .execute(
            "INSERT INTO \"users\" (\"id\", \"email\") VALUES (1, NULL)",
            &[],
        )
        .unwrap();

    let row = client
        .query_one("SELECT \"email\" FROM \"users\" WHERE \"id\" = 1", &[])
        .unwrap();
    let email: Option<&str> = row.get(0);
    assert!(
        email.is_none(),
        "NULL should be allowed after DROP NOT NULL"
    );
}

// ==========================================================================
// ALTER TABLE — add constraint
// ==========================================================================

#[test]
fn alter_table_add_constraint() {
    let mut client = crate::test_client("template0");

    client
        .execute(
            "CREATE TABLE \"users\" (\"id\" INTEGER, \"email\" TEXT)",
            &[],
        )
        .unwrap();

    let stmt = SchemaMutationStmt::AddConstraint {
        schema_ref: SchemaRef::new("users"),
        constraint: ConstraintDef::Unique {
            name: Some("uq_email".into()),
            columns: vec!["email".into()],
            include: None,
            nulls_distinct: None,
            condition: None,
        },
        not_valid: false,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    client
        .execute(
            "INSERT INTO \"users\" (\"id\", \"email\") VALUES (1, 'a@b.com')",
            &[],
        )
        .unwrap();

    let err = client.execute(
        "INSERT INTO \"users\" (\"id\", \"email\") VALUES (2, 'a@b.com')",
        &[],
    );
    assert!(
        err.is_err(),
        "duplicate email should fail after ADD CONSTRAINT UNIQUE"
    );
}

// ==========================================================================
// ALTER TABLE — drop constraint
// ==========================================================================

#[test]
fn alter_table_drop_constraint() {
    let mut client = crate::test_client("template0");

    client
        .execute(
            "CREATE TABLE \"users\" (\"id\" INTEGER, \"email\" TEXT, CONSTRAINT \"uq_email\" UNIQUE (\"email\"))",
            &[],
        )
        .unwrap();

    let stmt = SchemaMutationStmt::DropConstraint {
        schema_ref: SchemaRef::new("users"),
        constraint_name: "uq_email".into(),
        if_exists: false,
        cascade: false,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    // Duplicate should now succeed
    client
        .execute(
            "INSERT INTO \"users\" (\"id\", \"email\") VALUES (1, 'a@b.com')",
            &[],
        )
        .unwrap();
    client
        .execute(
            "INSERT INTO \"users\" (\"id\", \"email\") VALUES (2, 'a@b.com')",
            &[],
        )
        .unwrap();
}

// ==========================================================================
// TRUNCATE TABLE
// ==========================================================================

#[test]
fn truncate_table() {
    let mut client = crate::test_client("template0");

    client
        .execute("CREATE TABLE \"users\" (\"id\" INTEGER)", &[])
        .unwrap();
    client
        .execute("INSERT INTO \"users\" VALUES (1)", &[])
        .unwrap();
    client
        .execute("INSERT INTO \"users\" VALUES (2)", &[])
        .unwrap();

    let stmt = SchemaMutationStmt::TruncateTable {
        schema_ref: SchemaRef::new("users"),
        restart_identity: false,
        cascade: false,
    };
    client.execute(&render(&stmt), &[]).unwrap();

    let row = client
        .query_one("SELECT COUNT(*) FROM \"users\"", &[])
        .unwrap();
    let count: i64 = row.get(0);
    assert_eq!(count, 0, "table should be empty after TRUNCATE");
}
