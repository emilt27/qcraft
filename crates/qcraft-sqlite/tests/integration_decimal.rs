use qcraft_core::ast::ddl::*;
use qcraft_core::ast::expr::Expr;
use qcraft_core::ast::value::Value;
use qcraft_sqlite::SqliteRenderer;
use rusqlite::Connection;

mod common;

/// qcraft renders a single-column `CREATE TABLE` — this is the code under test.
fn create_table_sql(col: ColumnDef) -> String {
    let mut schema = SchemaDef::new("t");
    schema.columns = vec![col];
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
    SqliteRenderer::new().render_schema_stmt(&stmt).unwrap()[0]
        .0
        .clone()
}

#[test]
fn decimal_text_preserves_precision_via_param() {
    let db = Connection::open_in_memory().unwrap();
    let ddl = create_table_sql(ColumnDef::new("val", FieldType::decimal(38, 10)));
    assert_eq!(ddl, r#"CREATE TABLE "t" ("val" DECIMAL_TEXT(38, 10))"#);
    db.execute_batch(&ddl).unwrap();

    // 22 significant digits — beyond f64 (~17). Bound as a Value::Decimal param.
    let boxed = common::to_sqlite_params(&[Value::Decimal("12345678901234567890.12".into())]);
    let params = common::as_sqlite_params(&boxed);
    db.execute(r#"INSERT INTO "t" ("val") VALUES (?)"#, params.as_slice())
        .unwrap();

    let (val, ty): (String, String) = db
        .query_row(r#"SELECT "val", typeof("val") FROM "t""#, [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(val, "12345678901234567890.12");
    assert_eq!(ty, "text");
}

#[test]
fn decimal_text_inline_default_preserves_precision() {
    let db = Connection::open_in_memory().unwrap();
    let ddl = create_table_sql(
        ColumnDef::new("val", FieldType::decimal(10, 2))
            .default(Expr::Value(Value::Decimal("10.234".into()))),
    );
    assert_eq!(
        ddl,
        r#"CREATE TABLE "t" ("val" DECIMAL_TEXT(10, 2) DEFAULT ('10.234'))"#
    );
    db.execute_batch(&ddl).unwrap();
    db.execute(r#"INSERT INTO "t" DEFAULT VALUES"#, []).unwrap();

    let (val, ty): (String, String) = db
        .query_row(r#"SELECT "val", typeof("val") FROM "t""#, [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(val, "10.234");
    assert_eq!(ty, "text");
}
