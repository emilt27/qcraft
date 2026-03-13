//! Tests verifying that unsupported / custom features in DQL correctly
//! return render errors from the PostgreSQL renderer.

use qcraft_core::ast::common::*;
use qcraft_core::ast::conditions::*;
use qcraft_core::ast::expr::*;
use qcraft_core::ast::query::*;
use qcraft_postgres::PostgresRenderer;

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

fn render_err(stmt: &QueryStmt) -> String {
    let renderer = PostgresRenderer::new();
    renderer.render_query_stmt(stmt).unwrap_err().to_string()
}

// ==========================================================================
// Custom table source is unsupported
// ==========================================================================

#[derive(Debug, Clone)]
struct DummyTableSource;

impl qcraft_core::ast::custom::CustomTableSource for DummyTableSource {
    fn clone_box(&self) -> Box<dyn qcraft_core::ast::custom::CustomTableSource> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn custom_table_source_unsupported() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::Custom(Box::new(DummyTableSource)),
            only: false,
            sample: None,
            index_hint: None,
        }]),
        ..simple_query()
    };
    let err = render_err(&stmt);
    assert!(
        err.contains("CustomTableSource") || err.contains("unsupported"),
        "Expected unsupported error for custom table source, got: {err}"
    );
}

// ==========================================================================
// Custom expression is unsupported
// ==========================================================================

#[derive(Debug, Clone)]
struct DummyExpr;

impl qcraft_core::ast::custom::CustomExpr for DummyExpr {
    fn clone_box(&self) -> Box<dyn qcraft_core::ast::custom::CustomExpr> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn custom_expr_unsupported() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Custom(Box::new(DummyExpr)),
            alias: None,
        }],
        ..simple_query()
    };
    let err = render_err(&stmt);
    assert!(
        err.contains("CustomExpr") || err.contains("unsupported"),
        "Expected unsupported error for custom expr, got: {err}"
    );
}

// ==========================================================================
// Custom condition is unsupported
// ==========================================================================

#[derive(Debug, Clone)]
struct DummyCondition;

impl qcraft_core::ast::custom::CustomCondition for DummyCondition {
    fn clone_box(&self) -> Box<dyn qcraft_core::ast::custom::CustomCondition> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn custom_condition_unsupported() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Custom(Box::new(
            DummyCondition,
        ))])),
        ..simple_query()
    };
    let err = render_err(&stmt);
    assert!(
        err.contains("CustomCondition") || err.contains("unsupported"),
        "Expected unsupported error for custom condition, got: {err}"
    );
}

// ==========================================================================
// Custom compare operator is unsupported
// ==========================================================================

#[derive(Debug, Clone)]
struct DummyCompareOp;

impl qcraft_core::ast::custom::CustomCompareOp for DummyCompareOp {
    fn clone_box(&self) -> Box<dyn qcraft_core::ast::custom::CustomCompareOp> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn custom_compare_op_unsupported() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "id")),
                op: CompareOp::Custom(Box::new(DummyCompareOp)),
                right: Expr::Value(qcraft_core::ast::value::Value::Int(1)),
                negate: false,
            },
        ))])),
        ..simple_query()
    };
    let err = render_err(&stmt);
    assert!(
        err.contains("CustomCompareOp") || err.contains("unsupported"),
        "Expected unsupported error for custom compare op, got: {err}"
    );
}
