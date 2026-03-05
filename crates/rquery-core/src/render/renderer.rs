use crate::ast::conditions::{CompareOp, Conditions};
use crate::ast::ddl::{
    ColumnDef, ConstraintDef, FieldType, IndexDef, SchemaMutationStmt,
};
use crate::ast::dml::{
    DeleteStmt, InsertStmt, MutationStmt, OnConflictDef, UpdateStmt,
};
use crate::ast::expr::{AggregationDef, CaseDef, Expr, WindowDef};
use crate::ast::common::FieldRef;
use crate::ast::common::OrderByDef;
use crate::ast::query::{
    CteDef, JoinDef, LimitDef, QueryStmt, SelectColumn,
    SelectLockDef, TableSource,
};
use crate::error::RenderResult;
use crate::render::ctx::RenderCtx;

/// The core rendering trait. Each dialect implements this.
///
/// Users can wrap a dialect renderer and override individual methods
/// to customize behavior.
pub trait Renderer {
    // ── Top-level ──

    fn render_query(&self, stmt: &QueryStmt, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_mutation(&self, stmt: &MutationStmt, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_schema_mutation(
        &self,
        stmt: &SchemaMutationStmt,
        ctx: &mut RenderCtx,
    ) -> RenderResult<()>;

    // ── SELECT parts ──

    fn render_select_columns(
        &self,
        cols: &[SelectColumn],
        ctx: &mut RenderCtx,
    ) -> RenderResult<()>;
    fn render_from(&self, source: &TableSource, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_joins(&self, joins: &[JoinDef], ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_where(&self, cond: &Conditions, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_order_by(&self, order: &[OrderByDef], ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_limit(&self, limit: &LimitDef, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_ctes(&self, ctes: &[CteDef], ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_lock(&self, lock: &SelectLockDef, ctx: &mut RenderCtx) -> RenderResult<()>;

    // ── Expressions ──

    fn render_expr(&self, expr: &Expr, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_aggregate(&self, agg: &AggregationDef, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_window(&self, win: &WindowDef, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_case(&self, case: &CaseDef, ctx: &mut RenderCtx) -> RenderResult<()>;

    // ── Conditions ──

    fn render_condition(&self, cond: &Conditions, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_compare_op(
        &self,
        op: &CompareOp,
        left: &Expr,
        right: &Expr,
        ctx: &mut RenderCtx,
    ) -> RenderResult<()>;

    // ── DML parts ──

    fn render_insert(&self, stmt: &InsertStmt, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_update(&self, stmt: &UpdateStmt, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_delete(&self, stmt: &DeleteStmt, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_on_conflict(&self, oc: &OnConflictDef, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_returning(&self, fields: &[FieldRef], ctx: &mut RenderCtx) -> RenderResult<()>;

    // ── DDL parts ──

    fn render_column_def(&self, col: &ColumnDef, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_column_type(&self, ty: &FieldType, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_constraint(&self, c: &ConstraintDef, ctx: &mut RenderCtx) -> RenderResult<()>;
    fn render_index_def(&self, idx: &IndexDef, ctx: &mut RenderCtx) -> RenderResult<()>;
}

/// Macro to delegate all Renderer methods to an inner renderer.
///
/// Usage:
/// ```ignore
/// struct MyRenderer { inner: PostgresRenderer }
/// impl Renderer for MyRenderer {
///     fn render_cast(&self, ...) { /* custom */ }
///     delegate_renderer!(self.inner);
/// }
/// ```
#[macro_export]
macro_rules! delegate_renderer {
    ($self:ident . $inner:ident) => {
        fn render_query(
            &$self,
            stmt: &$crate::ast::query::QueryStmt,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_query(stmt, ctx)
        }
        fn render_mutation(
            &$self,
            stmt: &$crate::ast::dml::MutationStmt,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_mutation(stmt, ctx)
        }
        fn render_schema_mutation(
            &$self,
            stmt: &$crate::ast::ddl::SchemaMutationStmt,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_schema_mutation(stmt, ctx)
        }
        fn render_select_columns(
            &$self,
            cols: &[$crate::ast::query::SelectColumn],
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_select_columns(cols, ctx)
        }
        fn render_from(
            &$self,
            source: &$crate::ast::query::TableSource,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_from(source, ctx)
        }
        fn render_joins(
            &$self,
            joins: &[$crate::ast::query::JoinDef],
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_joins(joins, ctx)
        }
        fn render_where(
            &$self,
            cond: &$crate::ast::conditions::Conditions,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_where(cond, ctx)
        }
        fn render_order_by(
            &$self,
            order: &[$crate::ast::query::OrderByDef],
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_order_by(order, ctx)
        }
        fn render_limit(
            &$self,
            limit: &$crate::ast::query::LimitDef,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_limit(limit, ctx)
        }
        fn render_ctes(
            &$self,
            ctes: &[$crate::ast::query::CteDef],
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_ctes(ctes, ctx)
        }
        fn render_lock(
            &$self,
            lock: &$crate::ast::query::SelectLockDef,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_lock(lock, ctx)
        }
        fn render_expr(
            &$self,
            expr: &$crate::ast::expr::Expr,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_expr(expr, ctx)
        }
        fn render_aggregate(
            &$self,
            agg: &$crate::ast::expr::AggregationDef,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_aggregate(agg, ctx)
        }
        fn render_window(
            &$self,
            win: &$crate::ast::expr::WindowDef,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_window(win, ctx)
        }
        fn render_case(
            &$self,
            case: &$crate::ast::expr::CaseDef,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_case(case, ctx)
        }
        fn render_condition(
            &$self,
            cond: &$crate::ast::conditions::Conditions,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_condition(cond, ctx)
        }
        fn render_compare_op(
            &$self,
            op: &$crate::ast::conditions::CompareOp,
            left: &$crate::ast::expr::Expr,
            right: &$crate::ast::expr::Expr,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_compare_op(op, left, right, ctx)
        }
        fn render_insert(
            &$self,
            stmt: &$crate::ast::dml::InsertStmt,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_insert(stmt, ctx)
        }
        fn render_update(
            &$self,
            stmt: &$crate::ast::dml::UpdateStmt,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_update(stmt, ctx)
        }
        fn render_delete(
            &$self,
            stmt: &$crate::ast::dml::DeleteStmt,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_delete(stmt, ctx)
        }
        fn render_on_conflict(
            &$self,
            oc: &$crate::ast::dml::OnConflictDef,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_on_conflict(oc, ctx)
        }
        fn render_returning(
            &$self,
            fields: &[$crate::ast::common::FieldRef],
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_returning(fields, ctx)
        }
        fn render_column_def(
            &$self,
            col: &$crate::ast::ddl::ColumnDef,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_column_def(col, ctx)
        }
        fn render_column_type(
            &$self,
            ty: &$crate::ast::ddl::FieldType,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_column_type(ty, ctx)
        }
        fn render_constraint(
            &$self,
            c: &$crate::ast::ddl::ConstraintDef,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_constraint(c, ctx)
        }
        fn render_index_def(
            &$self,
            idx: &$crate::ast::ddl::IndexDef,
            ctx: &mut $crate::render::ctx::RenderCtx,
        ) -> $crate::error::RenderResult<()> {
            $self.$inner.render_index_def(idx, ctx)
        }
    };
}
