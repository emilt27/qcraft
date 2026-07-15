//! Extensibility: a user teaches qcraft a syntax it does not know, without waiting
//! for a release. The node itself knows how to render; it is handed the renderer so
//! it can recurse into its own sub-expressions and bind parameters through the same
//! context as everything else.

use std::any::Any;
use std::fmt;

use qcraft_core::ast::common::{FieldRef, SchemaRef};
use qcraft_core::ast::conditions::*;
use qcraft_core::ast::custom::{CustomBinaryOp, CustomCondition, CustomExpr};
use qcraft_core::ast::expr::*;
use qcraft_core::ast::query::*;
use qcraft_core::ast::value::Value;
use qcraft_core::error::RenderResult;
use qcraft_core::render::ctx::RenderCtx;
use qcraft_core::render::renderer::Renderer;
use qcraft_postgres::PostgresRenderer;

// ── A user-defined expression: `<expr> AT TIME ZONE '<zone>'` ────────────────
// Infix, so it is NOT self-delimiting: as the operand of `::` it must be bracketed.

#[derive(Clone)]
struct AtTimeZone {
    expr: Expr,
    zone: String,
}

impl fmt::Debug for AtTimeZone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AtTimeZone")
    }
}

impl CustomExpr for AtTimeZone {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn clone_box(&self) -> Box<dyn CustomExpr> {
        Box::new(self.clone())
    }

    fn render(&self, renderer: &dyn Renderer, ctx: &mut RenderCtx) -> RenderResult<()> {
        // AT TIME ZONE is an operator, so its own left-hand side is an operand:
        // `a + b AT TIME ZONE 'UTC'` would bind the zone to `b`. render_operand
        // brackets it when the sub-expression's structure requires it.
        renderer.render_operand(&self.expr, ctx)?;
        ctx.keyword("AT TIME ZONE").string_literal(&self.zone);
        Ok(())
    }

    fn needs_operand_parens(&self) -> bool {
        true
    }
}

// ── A user-defined binary operator: `<->` style, but theirs ──────────────────

#[derive(Debug, Clone)]
struct SameDay;

impl CustomBinaryOp for SameDay {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn clone_box(&self) -> Box<dyn CustomBinaryOp> {
        Box::new(self.clone())
    }
    fn render(&self, _renderer: &dyn Renderer, ctx: &mut RenderCtx) -> RenderResult<()> {
        ctx.write(" <=> ");
        Ok(())
    }
}

// ── A user-defined condition: `<field> IS DISTINCT FROM <value>` ─────────────

#[derive(Clone)]
struct IsDistinctFrom {
    field: FieldRef,
    value: Value,
}

impl fmt::Debug for IsDistinctFrom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IsDistinctFrom")
    }
}

impl CustomCondition for IsDistinctFrom {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn clone_box(&self) -> Box<dyn CustomCondition> {
        Box::new(self.clone())
    }
    fn render(&self, renderer: &dyn Renderer, ctx: &mut RenderCtx) -> RenderResult<()> {
        renderer.render_expr(&Expr::Field(self.field.clone()), ctx)?;
        ctx.keyword("IS DISTINCT FROM");
        ctx.param(self.value.clone());
        Ok(())
    }
}

fn render(stmt: &QueryStmt) -> (String, Vec<Value>) {
    PostgresRenderer::new().render_query_stmt(stmt).unwrap()
}

fn base() -> QueryStmt {
    QueryStmt {
        ctes: None,
        columns: vec![],
        distinct: None,
        from: Some(vec![FromItem::table(SchemaRef::new("events"))]),
        joins: None,
        where_clause: None,
        group_by: None,
        having: None,
        window: None,
        order_by: None,
        limit: None,
        lock: None,
        set_op: None,
    }
}

fn at_utc(table: &str, col: &str) -> Expr {
    Expr::Custom(Box::new(AtTimeZone {
        expr: Expr::field(table, col),
        zone: "UTC".into(),
    }))
}

#[test]
fn custom_expr_renders_in_a_real_query() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: at_utc("events", "created_at"),
            alias: Some("created_utc".into()),
        }],
        ..base()
    };
    let (sql, _) = render(&stmt);
    assert_eq!(
        sql,
        r#"SELECT "events"."created_at" AT TIME ZONE 'UTC' AS "created_utc" FROM "events""#
    );
}

#[test]
fn custom_expr_as_cast_operand_is_bracketed() {
    // `::` binds tighter than AT TIME ZONE, so a bare operand would cast the zone
    // literal: `created_at AT TIME ZONE ('UTC'::date)`. The node says it is infix.
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::cast(at_utc("events", "created_at"), "date"),
            alias: None,
        }],
        ..base()
    };
    let (sql, _) = render(&stmt);
    assert_eq!(
        sql,
        r#"SELECT ("events"."created_at" AT TIME ZONE 'UTC')::date FROM "events""#
    );
}

#[test]
fn custom_expr_recurses_through_the_renderer_and_binds_params() {
    // The custom node holds a sub-expression containing a parameter. Rendering must go
    // back through the renderer so the placeholder is numbered in document order.
    let inner = Expr::Custom(Box::new(AtTimeZone {
        expr: Expr::Binary {
            left: Box::new(Expr::field("events", "created_at")),
            op: BinaryOp::Add,
            right: Box::new(Expr::Value(Value::Int(7))),
        },
        zone: "UTC".into(),
    }));
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: inner,
            alias: None,
        }],
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison::new(
                Expr::field("events", "id"),
                CompareOp::Eq,
                Expr::Value(Value::Int(42)),
            ),
        ))])),
        ..base()
    };
    let (sql, params) = render(&stmt);
    assert_eq!(
        sql,
        r#"SELECT ("events"."created_at" + $1) AT TIME ZONE 'UTC' FROM "events" WHERE "events"."id" = $2"#
    );
    assert_eq!(params, vec![Value::Int(7), Value::Int(42)]);
}

#[test]
fn custom_binary_op_renders() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Binary {
                left: Box::new(Expr::field("events", "a")),
                op: BinaryOp::Custom(Box::new(SameDay)),
                right: Box::new(Expr::field("events", "b")),
            },
            alias: None,
        }],
        ..base()
    };
    let (sql, _) = render(&stmt);
    assert_eq!(sql, r#"SELECT "events"."a" <=> "events"."b" FROM "events""#);
}

#[test]
fn custom_condition_renders_and_binds_params() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::all()],
        where_clause: Some(Conditions::and(vec![ConditionNode::Custom(Box::new(
            IsDistinctFrom {
                field: FieldRef::new("events", "status"),
                value: Value::Str("done".into()),
            },
        ))])),
        ..base()
    };
    let (sql, params) = render(&stmt);
    assert_eq!(
        sql,
        r#"SELECT * FROM "events" WHERE "events"."status" IS DISTINCT FROM $1"#
    );
    assert_eq!(params, vec![Value::Str("done".into())]);
}

#[test]
fn a_custom_node_that_does_not_implement_render_still_errors() {
    // The default keeps the old contract: an unrenderable custom node is a clear error,
    // not silently dropped SQL.
    #[derive(Debug, Clone)]
    struct Unrenderable;
    impl CustomExpr for Unrenderable {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn clone_box(&self) -> Box<dyn CustomExpr> {
            Box::new(self.clone())
        }
    }

    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Custom(Box::new(Unrenderable)),
            alias: None,
        }],
        ..base()
    };
    let err = PostgresRenderer::new()
        .render_query_stmt(&stmt)
        .unwrap_err()
        .to_string();
    assert!(err.contains("CustomExpr"), "unexpected error: {err}");
}
