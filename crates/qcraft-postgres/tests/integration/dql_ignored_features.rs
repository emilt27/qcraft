//! Integration tests verifying that PG renderer silently ignores
//! features from other database dialects (e.g. SQLite index hints, TOP).

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

// ==========================================================================
// SQLite index hint is silently ignored
// ==========================================================================

#[test]
fn index_hint_ignored() {
    let mut client = crate::test_client("template_dql_ign");
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
    let boxed = crate::common::to_pg_params(&values);
    let params = crate::common::as_pg_params(&boxed);
    // The SQL should NOT contain INDEXED BY (PG ignores it)
    assert!(!sql.contains("INDEXED BY"));
    // But the query should still execute fine
    let rows = client.query(&sql, &params).unwrap();
    assert_eq!(rows.len(), 3);
}

#[test]
fn not_indexed_hint_ignored() {
    let mut client = crate::test_client("template_dql_ign");
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
    let boxed = crate::common::to_pg_params(&values);
    let params = crate::common::as_pg_params(&boxed);
    assert!(!sql.contains("NOT INDEXED"));
    let rows = client.query(&sql, &params).unwrap();
    assert_eq!(rows.len(), 3);
}

// ==========================================================================
// TOP converts to LIMIT on PG
// ==========================================================================

#[test]
fn top_converts_to_limit() {
    let mut client = crate::test_client("template_dql_ign");
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
    let boxed = crate::common::to_pg_params(&values);
    let params = crate::common::as_pg_params(&boxed);
    // Should render as LIMIT, not TOP
    assert!(sql.contains("LIMIT $"));
    assert!(!sql.contains("TOP"));
    let rows = client.query(&sql, &params).unwrap();
    assert_eq!(rows.len(), 2);
}
