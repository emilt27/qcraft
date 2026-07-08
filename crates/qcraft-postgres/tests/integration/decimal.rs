use qcraft_core::ast::ddl::*;
use qcraft_core::ast::value::Value;
use qcraft_postgres::PostgresRenderer;
use rust_decimal::Decimal;
use std::str::FromStr;

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
    PostgresRenderer::new().render_schema_stmt(&stmt).unwrap()[0]
        .0
        .clone()
}

#[test]
fn numeric_decimal_round_trips_via_param() {
    let mut client = crate::test_client("template0");
    let ddl = create_table_sql(ColumnDef::new("val", FieldType::decimal(38, 2)));
    assert_eq!(ddl, r#"CREATE TABLE "t" ("val" NUMERIC(38, 2))"#);
    client.execute(&ddl, &[]).unwrap();

    // Bound through Value::Decimal → the harness binds it as NUMERIC (rust_decimal).
    let boxed = crate::common::to_pg_params(&[Value::Decimal("12345678901234567890.12".into())]);
    let params = crate::common::as_pg_params(&boxed);
    client
        .execute(r#"INSERT INTO "t" ("val") VALUES ($1)"#, params.as_slice())
        .unwrap();

    let row = client.query_one(r#"SELECT "val" FROM "t""#, &[]).unwrap();
    let got: Decimal = row.get(0);
    assert_eq!(got, Decimal::from_str("12345678901234567890.12").unwrap());
}
