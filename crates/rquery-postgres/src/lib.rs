use rquery_core::ast::common::{FieldRef, OrderByDef, OrderDir};
use rquery_core::ast::conditions::{CompareOp, ConditionNode, Conditions, Connector};
use rquery_core::ast::ddl::{
    ColumnDef, ConstraintDef, DeferrableConstraint, FieldType,
    IdentityColumn, IndexColumnDef, IndexDef, IndexExpr, MatchType, NullsOrder,
    ReferentialAction, SchemaDef, SchemaMutationStmt,
};
use rquery_core::ast::dml::{
    DeleteStmt, InsertStmt, MutationStmt, OnConflictDef, UpdateStmt,
};
use rquery_core::ast::expr::{
    AggregationDef, BinaryOp, CaseDef, Expr, UnaryOp, WindowDef, WindowFrameBound,
    WindowFrameDef, WindowFrameType,
};
use rquery_core::ast::query::{
    CteDef, JoinDef, LimitDef, QueryStmt, SelectColumn, SelectLockDef, TableSource,
};
use rquery_core::ast::value::Value;
use rquery_core::error::{RenderError, RenderResult};
use rquery_core::render::ctx::{ParamStyle, RenderCtx};
use rquery_core::render::renderer::Renderer;

pub struct PostgresRenderer;

impl PostgresRenderer {
    pub fn new() -> Self {
        Self
    }

    /// Convenience: render a statement to SQL string + params.
    pub fn render_schema_stmt(&self, stmt: &SchemaMutationStmt) -> RenderResult<(String, Vec<Value>)> {
        let mut ctx = RenderCtx::new(ParamStyle::Dollar);
        self.render_schema_mutation(stmt, &mut ctx)?;
        Ok(ctx.finish())
    }
}

impl Default for PostgresRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// ==========================================================================
// Renderer trait implementation
// ==========================================================================

impl Renderer for PostgresRenderer {
    // ── DDL ──────────────────────────────────────────────────────────────

    fn render_schema_mutation(
        &self,
        stmt: &SchemaMutationStmt,
        ctx: &mut RenderCtx,
    ) -> RenderResult<()> {
        match stmt {
            SchemaMutationStmt::CreateTable {
                schema,
                if_not_exists,
                temporary,
                unlogged,
                tablespace,
            } => self.pg_create_table(schema, *if_not_exists, *temporary, *unlogged, tablespace.as_deref(), ctx),

            SchemaMutationStmt::DropTable {
                schema_ref,
                if_exists,
                cascade,
            } => {
                ctx.keyword("DROP TABLE");
                if *if_exists {
                    ctx.keyword("IF EXISTS");
                }
                self.pg_schema_ref(schema_ref, ctx);
                if *cascade {
                    ctx.keyword("CASCADE");
                }
                Ok(())
            }

            SchemaMutationStmt::RenameTable {
                schema_ref,
                new_name,
            } => {
                ctx.keyword("ALTER TABLE");
                self.pg_schema_ref(schema_ref, ctx);
                ctx.keyword("RENAME TO").ident(new_name);
                Ok(())
            }

            SchemaMutationStmt::AddColumn {
                schema_ref,
                column,
                if_not_exists,
                position: _, // PostgreSQL doesn't support FIRST/AFTER
            } => {
                ctx.keyword("ALTER TABLE");
                self.pg_schema_ref(schema_ref, ctx);
                ctx.keyword("ADD COLUMN");
                if *if_not_exists {
                    ctx.keyword("IF NOT EXISTS");
                }
                self.render_column_def(column, ctx)
            }

            SchemaMutationStmt::DropColumn {
                schema_ref,
                name,
                if_exists,
                cascade,
            } => {
                ctx.keyword("ALTER TABLE");
                self.pg_schema_ref(schema_ref, ctx);
                ctx.keyword("DROP COLUMN");
                if *if_exists {
                    ctx.keyword("IF EXISTS");
                }
                ctx.ident(name);
                if *cascade {
                    ctx.keyword("CASCADE");
                }
                Ok(())
            }

            SchemaMutationStmt::RenameColumn {
                schema_ref,
                old_name,
                new_name,
            } => {
                ctx.keyword("ALTER TABLE");
                self.pg_schema_ref(schema_ref, ctx);
                ctx.keyword("RENAME COLUMN").ident(old_name).keyword("TO").ident(new_name);
                Ok(())
            }

            SchemaMutationStmt::AlterColumnType {
                schema_ref,
                column_name,
                new_type,
                using_expr,
            } => {
                ctx.keyword("ALTER TABLE");
                self.pg_schema_ref(schema_ref, ctx);
                ctx.keyword("ALTER COLUMN").ident(column_name).keyword("SET DATA TYPE");
                self.render_column_type(new_type, ctx)?;
                if let Some(expr) = using_expr {
                    ctx.keyword("USING");
                    self.render_expr(expr, ctx)?;
                }
                Ok(())
            }

            SchemaMutationStmt::AlterColumnDefault {
                schema_ref,
                column_name,
                default,
            } => {
                ctx.keyword("ALTER TABLE");
                self.pg_schema_ref(schema_ref, ctx);
                ctx.keyword("ALTER COLUMN").ident(column_name);
                match default {
                    Some(expr) => {
                        ctx.keyword("SET DEFAULT");
                        self.render_expr(expr, ctx)?;
                    }
                    None => {
                        ctx.keyword("DROP DEFAULT");
                    }
                }
                Ok(())
            }

            SchemaMutationStmt::AlterColumnNullability {
                schema_ref,
                column_name,
                not_null,
            } => {
                ctx.keyword("ALTER TABLE");
                self.pg_schema_ref(schema_ref, ctx);
                ctx.keyword("ALTER COLUMN").ident(column_name);
                if *not_null {
                    ctx.keyword("SET NOT NULL");
                } else {
                    ctx.keyword("DROP NOT NULL");
                }
                Ok(())
            }

            SchemaMutationStmt::AddConstraint {
                schema_ref,
                constraint,
                not_valid,
            } => {
                ctx.keyword("ALTER TABLE");
                self.pg_schema_ref(schema_ref, ctx);
                ctx.keyword("ADD");
                self.render_constraint(constraint, ctx)?;
                if *not_valid {
                    ctx.keyword("NOT VALID");
                }
                Ok(())
            }

            SchemaMutationStmt::DropConstraint {
                schema_ref,
                constraint_name,
                if_exists,
                cascade,
            } => {
                ctx.keyword("ALTER TABLE");
                self.pg_schema_ref(schema_ref, ctx);
                ctx.keyword("DROP CONSTRAINT");
                if *if_exists {
                    ctx.keyword("IF EXISTS");
                }
                ctx.ident(constraint_name);
                if *cascade {
                    ctx.keyword("CASCADE");
                }
                Ok(())
            }

            SchemaMutationStmt::RenameConstraint {
                schema_ref,
                old_name,
                new_name,
            } => {
                ctx.keyword("ALTER TABLE");
                self.pg_schema_ref(schema_ref, ctx);
                ctx.keyword("RENAME CONSTRAINT").ident(old_name).keyword("TO").ident(new_name);
                Ok(())
            }

            SchemaMutationStmt::ValidateConstraint {
                schema_ref,
                constraint_name,
            } => {
                ctx.keyword("ALTER TABLE");
                self.pg_schema_ref(schema_ref, ctx);
                ctx.keyword("VALIDATE CONSTRAINT").ident(constraint_name);
                Ok(())
            }

            SchemaMutationStmt::CreateIndex {
                schema_ref,
                index,
                if_not_exists,
                concurrently,
            } => self.pg_create_index(schema_ref, index, *if_not_exists, *concurrently, ctx),

            SchemaMutationStmt::DropIndex {
                schema_ref: _,
                index_name,
                if_exists,
                concurrently,
                cascade,
            } => {
                ctx.keyword("DROP INDEX");
                if *concurrently {
                    ctx.keyword("CONCURRENTLY");
                }
                if *if_exists {
                    ctx.keyword("IF EXISTS");
                }
                ctx.ident(index_name);
                if *cascade {
                    ctx.keyword("CASCADE");
                }
                Ok(())
            }

            SchemaMutationStmt::CreateExtension {
                name,
                if_not_exists,
                schema,
                version,
                cascade,
            } => {
                ctx.keyword("CREATE EXTENSION");
                if *if_not_exists {
                    ctx.keyword("IF NOT EXISTS");
                }
                ctx.ident(name);
                if let Some(s) = schema {
                    ctx.keyword("SCHEMA").ident(s);
                }
                if let Some(v) = version {
                    ctx.keyword("VERSION").string_literal(v);
                }
                if *cascade {
                    ctx.keyword("CASCADE");
                }
                Ok(())
            }

            SchemaMutationStmt::DropExtension {
                name,
                if_exists,
                cascade,
            } => {
                ctx.keyword("DROP EXTENSION");
                if *if_exists {
                    ctx.keyword("IF EXISTS");
                }
                ctx.ident(name);
                if *cascade {
                    ctx.keyword("CASCADE");
                }
                Ok(())
            }

            SchemaMutationStmt::Custom(_) => Err(RenderError::unsupported(
                "CustomSchemaMutation",
                "custom DDL must be handled by a wrapping renderer",
            )),
        }
    }

    fn render_column_def(&self, col: &ColumnDef, ctx: &mut RenderCtx) -> RenderResult<()> {
        ctx.ident(&col.name);
        self.render_column_type(&col.field_type, ctx)?;

        if let Some(collation) = &col.collation {
            ctx.keyword("COLLATE").ident(collation);
        }

        if col.not_null {
            ctx.keyword("NOT NULL");
        }

        if let Some(default) = &col.default {
            ctx.keyword("DEFAULT");
            self.render_expr(default, ctx)?;
        }

        if let Some(identity) = &col.identity {
            self.pg_identity(identity, ctx);
        }

        if let Some(generated) = &col.generated {
            ctx.keyword("GENERATED ALWAYS AS").space().paren_open();
            self.render_expr(&generated.expr, ctx)?;
            ctx.paren_close().keyword("STORED");
        }

        Ok(())
    }

    fn render_column_type(&self, ty: &FieldType, ctx: &mut RenderCtx) -> RenderResult<()> {
        match ty {
            FieldType::Scalar(name) => {
                ctx.keyword(name);
            }
            FieldType::Parameterized { name, params } => {
                ctx.keyword(name).paren_open();
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        ctx.comma();
                    }
                    ctx.write(p);
                }
                ctx.paren_close();
            }
            FieldType::Array(inner) => {
                self.render_column_type(inner, ctx)?;
                ctx.write("[]");
            }
            FieldType::Vector(dim) => {
                ctx.keyword("VECTOR").paren_open().write(&dim.to_string()).paren_close();
            }
            FieldType::Custom(_) => {
                return Err(RenderError::unsupported(
                    "CustomFieldType",
                    "custom field type must be handled by a wrapping renderer",
                ));
            }
        }
        Ok(())
    }

    fn render_constraint(&self, c: &ConstraintDef, ctx: &mut RenderCtx) -> RenderResult<()> {
        match c {
            ConstraintDef::PrimaryKey {
                name,
                columns,
                include,
            } => {
                if let Some(n) = name {
                    ctx.keyword("CONSTRAINT").ident(n);
                }
                ctx.keyword("PRIMARY KEY").paren_open();
                self.pg_comma_idents(columns, ctx);
                ctx.paren_close();
                if let Some(inc) = include {
                    ctx.keyword("INCLUDE").paren_open();
                    self.pg_comma_idents(inc, ctx);
                    ctx.paren_close();
                }
            }

            ConstraintDef::ForeignKey {
                name,
                columns,
                ref_table,
                ref_columns,
                on_delete,
                on_update,
                deferrable,
                match_type,
            } => {
                if let Some(n) = name {
                    ctx.keyword("CONSTRAINT").ident(n);
                }
                ctx.keyword("FOREIGN KEY").paren_open();
                self.pg_comma_idents(columns, ctx);
                ctx.paren_close().keyword("REFERENCES");
                self.pg_schema_ref(ref_table, ctx);
                ctx.paren_open();
                self.pg_comma_idents(ref_columns, ctx);
                ctx.paren_close();
                if let Some(mt) = match_type {
                    ctx.keyword(match mt {
                        MatchType::Full => "MATCH FULL",
                        MatchType::Partial => "MATCH PARTIAL",
                        MatchType::Simple => "MATCH SIMPLE",
                    });
                }
                if let Some(action) = on_delete {
                    ctx.keyword("ON DELETE");
                    self.pg_referential_action(action, ctx);
                }
                if let Some(action) = on_update {
                    ctx.keyword("ON UPDATE");
                    self.pg_referential_action(action, ctx);
                }
                if let Some(def) = deferrable {
                    self.pg_deferrable(def, ctx);
                }
            }

            ConstraintDef::Unique {
                name,
                columns,
                include,
                nulls_distinct,
                condition: _, // Partial unique → rendered as separate CREATE INDEX
            } => {
                if let Some(n) = name {
                    ctx.keyword("CONSTRAINT").ident(n);
                }
                ctx.keyword("UNIQUE");
                if let Some(false) = nulls_distinct {
                    ctx.keyword("NULLS NOT DISTINCT");
                }
                ctx.paren_open();
                self.pg_comma_idents(columns, ctx);
                ctx.paren_close();
                if let Some(inc) = include {
                    ctx.keyword("INCLUDE").paren_open();
                    self.pg_comma_idents(inc, ctx);
                    ctx.paren_close();
                }
            }

            ConstraintDef::Check {
                name,
                condition,
                no_inherit,
                enforced: _, // PostgreSQL always enforces CHECK
            } => {
                if let Some(n) = name {
                    ctx.keyword("CONSTRAINT").ident(n);
                }
                ctx.keyword("CHECK").paren_open();
                self.render_condition(condition, ctx)?;
                ctx.paren_close();
                if *no_inherit {
                    ctx.keyword("NO INHERIT");
                }
            }

            ConstraintDef::Exclusion {
                name,
                elements,
                index_method,
                condition,
            } => {
                if let Some(n) = name {
                    ctx.keyword("CONSTRAINT").ident(n);
                }
                ctx.keyword("EXCLUDE USING").keyword(index_method).paren_open();
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        ctx.comma();
                    }
                    ctx.ident(&elem.column).keyword("WITH").keyword(&elem.operator);
                }
                ctx.paren_close();
                if let Some(cond) = condition {
                    ctx.keyword("WHERE").paren_open();
                    self.render_condition(cond, ctx)?;
                    ctx.paren_close();
                }
            }

            ConstraintDef::Custom(_) => {
                return Err(RenderError::unsupported(
                    "CustomConstraint",
                    "custom constraint must be handled by a wrapping renderer",
                ));
            }
        }
        Ok(())
    }

    fn render_index_def(&self, idx: &IndexDef, ctx: &mut RenderCtx) -> RenderResult<()> {
        // Used for inline index rendering (inside CREATE TABLE context).
        // Full CREATE INDEX is handled in pg_create_index.
        ctx.ident(&idx.name);
        if let Some(index_type) = &idx.index_type {
            ctx.keyword("USING").keyword(index_type);
        }
        ctx.paren_open();
        self.pg_index_columns(&idx.columns, ctx)?;
        ctx.paren_close();
        Ok(())
    }

    // ── Expressions (basic, needed for DDL) ──────────────────────────────

    fn render_expr(&self, expr: &Expr, ctx: &mut RenderCtx) -> RenderResult<()> {
        match expr {
            Expr::Value(val) => self.pg_value(val, ctx),

            Expr::Field(field_ref) => {
                self.pg_field_ref(field_ref, ctx);
                Ok(())
            }

            Expr::Binary { left, op, right } => {
                self.render_expr(left, ctx)?;
                ctx.keyword(match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Mod => "%",
                    BinaryOp::BitwiseAnd => "&",
                    BinaryOp::BitwiseOr => "|",
                    BinaryOp::ShiftLeft => "<<",
                    BinaryOp::ShiftRight => ">>",
                    BinaryOp::Concat => "||",
                });
                self.render_expr(right, ctx)
            }

            Expr::Unary { op, expr: inner } => {
                match op {
                    UnaryOp::Neg => ctx.write("-"),
                    UnaryOp::Not => ctx.keyword("NOT"),
                    UnaryOp::BitwiseNot => ctx.write("~"),
                };
                self.render_expr(inner, ctx)
            }

            Expr::Func { name, args } => {
                ctx.keyword(name).paren_open();
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        ctx.comma();
                    }
                    self.render_expr(arg, ctx)?;
                }
                ctx.paren_close();
                Ok(())
            }

            Expr::Aggregate(agg) => self.render_aggregate(agg, ctx),

            Expr::Cast { expr: inner, to_type } => {
                self.render_expr(inner, ctx)?;
                ctx.operator("::");
                ctx.write(to_type);
                Ok(())
            }

            Expr::Case(case) => self.render_case(case, ctx),

            Expr::Window(win) => self.render_window(win, ctx),

            Expr::Exists(query) => {
                ctx.keyword("EXISTS").paren_open();
                self.render_query(query, ctx)?;
                ctx.paren_close();
                Ok(())
            }

            Expr::SubQuery(query) => {
                ctx.paren_open();
                self.render_query(query, ctx)?;
                ctx.paren_close();
                Ok(())
            }

            Expr::ArraySubQuery(query) => {
                ctx.keyword("ARRAY").paren_open();
                self.render_query(query, ctx)?;
                ctx.paren_close();
                Ok(())
            }

            Expr::Raw { sql, params } => {
                ctx.keyword(sql);
                // Raw params are already embedded in the SQL string
                let _ = params;
                Ok(())
            }

            Expr::Custom(_) => Err(RenderError::unsupported(
                "CustomExpr",
                "custom expression must be handled by a wrapping renderer",
            )),
        }
    }

    fn render_aggregate(&self, agg: &AggregationDef, ctx: &mut RenderCtx) -> RenderResult<()> {
        ctx.keyword(&agg.name).paren_open();
        if agg.distinct {
            ctx.keyword("DISTINCT");
        }
        if let Some(expr) = &agg.expression {
            self.render_expr(expr, ctx)?;
        } else {
            ctx.write("*");
        }
        if let Some(args) = &agg.args {
            for arg in args {
                ctx.comma();
                self.render_expr(arg, ctx)?;
            }
        }
        if let Some(order_by) = &agg.order_by {
            ctx.keyword("ORDER BY");
            self.pg_order_by_list(order_by, ctx);
        }
        ctx.paren_close();
        if let Some(filter) = &agg.filter {
            ctx.keyword("FILTER").paren_open().keyword("WHERE");
            self.render_condition(filter, ctx)?;
            ctx.paren_close();
        }
        Ok(())
    }

    fn render_window(&self, win: &WindowDef, ctx: &mut RenderCtx) -> RenderResult<()> {
        self.render_expr(&win.expression, ctx)?;
        ctx.keyword("OVER").paren_open();
        if let Some(partition_by) = &win.partition_by {
            ctx.keyword("PARTITION BY");
            for (i, expr) in partition_by.iter().enumerate() {
                if i > 0 {
                    ctx.comma();
                }
                self.render_expr(expr, ctx)?;
            }
        }
        if let Some(order_by) = &win.order_by {
            ctx.keyword("ORDER BY");
            self.pg_order_by_list(order_by, ctx);
        }
        if let Some(frame) = &win.frame {
            self.pg_window_frame(frame, ctx);
        }
        ctx.paren_close();
        Ok(())
    }

    fn render_case(&self, case: &CaseDef, ctx: &mut RenderCtx) -> RenderResult<()> {
        ctx.keyword("CASE");
        for clause in &case.cases {
            ctx.keyword("WHEN");
            self.render_condition(&clause.condition, ctx)?;
            ctx.keyword("THEN");
            self.render_expr(&clause.result, ctx)?;
        }
        if let Some(default) = &case.default {
            ctx.keyword("ELSE");
            self.render_expr(default, ctx)?;
        }
        ctx.keyword("END");
        Ok(())
    }

    // ── Conditions ───────────────────────────────────────────────────────

    fn render_condition(&self, cond: &Conditions, ctx: &mut RenderCtx) -> RenderResult<()> {
        if cond.negated {
            ctx.keyword("NOT").paren_open();
        }
        let connector = match cond.connector {
            Connector::And => " AND ",
            Connector::Or => " OR ",
        };
        for (i, child) in cond.children.iter().enumerate() {
            if i > 0 {
                ctx.write(connector);
            }
            match child {
                ConditionNode::Comparison(comp) => {
                    if comp.negate {
                        ctx.keyword("NOT").paren_open();
                    }
                    self.render_compare_op(&comp.op, &comp.left, &comp.right, ctx)?;
                    if comp.negate {
                        ctx.paren_close();
                    }
                }
                ConditionNode::Group(group) => {
                    ctx.paren_open();
                    self.render_condition(group, ctx)?;
                    ctx.paren_close();
                }
                ConditionNode::Exists(query) => {
                    ctx.keyword("EXISTS").paren_open();
                    self.render_query(query, ctx)?;
                    ctx.paren_close();
                }
                ConditionNode::Custom(_) => {
                    return Err(RenderError::unsupported(
                        "CustomCondition",
                        "custom condition must be handled by a wrapping renderer",
                    ));
                }
            }
        }
        if cond.negated {
            ctx.paren_close();
        }
        Ok(())
    }

    fn render_compare_op(
        &self,
        op: &CompareOp,
        left: &Expr,
        right: &Expr,
        ctx: &mut RenderCtx,
    ) -> RenderResult<()> {
        self.render_expr(left, ctx)?;
        match op {
            CompareOp::Eq => ctx.write(" = "),
            CompareOp::Neq => ctx.write(" <> "),
            CompareOp::Gt => ctx.write(" > "),
            CompareOp::Gte => ctx.write(" >= "),
            CompareOp::Lt => ctx.write(" < "),
            CompareOp::Lte => ctx.write(" <= "),
            CompareOp::Like => ctx.keyword("LIKE"),
            CompareOp::ILike => ctx.keyword("ILIKE"),
            CompareOp::In => ctx.keyword("IN"),
            CompareOp::Between => {
                ctx.keyword("BETWEEN");
                self.render_expr(right, ctx)?;
                return Ok(());
            }
            CompareOp::IsNull => {
                ctx.keyword("IS NULL");
                return Ok(());
            }
            CompareOp::Similar => ctx.keyword("SIMILAR TO"),
            CompareOp::Regex => ctx.write(" ~ "),
            CompareOp::IRegex => ctx.write(" ~* "),
            CompareOp::JsonbContains => ctx.write(" @> "),
            CompareOp::JsonbContainedBy => ctx.write(" <@ "),
            CompareOp::JsonbHasKey => ctx.write(" ? "),
            CompareOp::JsonbHasAnyKey => ctx.write(" ?| "),
            CompareOp::JsonbHasAllKeys => ctx.write(" ?& "),
            CompareOp::FtsMatch => ctx.write(" @@ "),
            CompareOp::TrigramSimilar => ctx.write(" % "),
            CompareOp::TrigramWordSimilar => ctx.write(" <% "),
            CompareOp::TrigramStrictWordSimilar => ctx.write(" <<% "),
            CompareOp::RangeContains => ctx.write(" @> "),
            CompareOp::RangeContainedBy => ctx.write(" <@ "),
            CompareOp::RangeOverlap => ctx.write(" && "),
            CompareOp::Custom(_) => {
                return Err(RenderError::unsupported(
                    "CustomCompareOp",
                    "custom compare op must be handled by a wrapping renderer",
                ));
            }
        };
        self.render_expr(right, ctx)
    }

    // ── Query (stub) ─────────────────────────────────────────────────────

    fn render_query(&self, _stmt: &QueryStmt, _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!("PostgreSQL query rendering not yet implemented")
    }

    fn render_select_columns(&self, _cols: &[SelectColumn], _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!()
    }
    fn render_from(&self, _source: &TableSource, _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!()
    }
    fn render_joins(&self, _joins: &[JoinDef], _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!()
    }
    fn render_where(&self, _cond: &Conditions, _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!()
    }
    fn render_order_by(&self, _order: &[OrderByDef], _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!()
    }
    fn render_limit(&self, _limit: &LimitDef, _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!()
    }
    fn render_ctes(&self, _ctes: &[CteDef], _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!()
    }
    fn render_lock(&self, _lock: &SelectLockDef, _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!()
    }

    // ── DML (stub) ───────────────────────────────────────────────────────

    fn render_mutation(&self, _stmt: &MutationStmt, _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!("PostgreSQL DML rendering not yet implemented")
    }
    fn render_insert(&self, _stmt: &InsertStmt, _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!()
    }
    fn render_update(&self, _stmt: &UpdateStmt, _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!()
    }
    fn render_delete(&self, _stmt: &DeleteStmt, _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!()
    }
    fn render_on_conflict(&self, _oc: &OnConflictDef, _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!()
    }
    fn render_returning(&self, _fields: &[FieldRef], _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!()
    }
}

// ==========================================================================
// PostgreSQL-specific helpers
// ==========================================================================

impl PostgresRenderer {
    fn pg_schema_ref(&self, schema_ref: &rquery_core::ast::common::SchemaRef, ctx: &mut RenderCtx) {
        if let Some(ns) = &schema_ref.namespace {
            ctx.ident(ns).operator(".");
        }
        ctx.ident(&schema_ref.name);
    }

    fn pg_field_ref(&self, field_ref: &FieldRef, ctx: &mut RenderCtx) {
        ctx.ident(&field_ref.table_name).operator(".").ident(&field_ref.field.name);
    }

    fn pg_comma_idents(&self, names: &[String], ctx: &mut RenderCtx) {
        for (i, name) in names.iter().enumerate() {
            if i > 0 {
                ctx.comma();
            }
            ctx.ident(name);
        }
    }

    fn pg_value(&self, val: &Value, ctx: &mut RenderCtx) -> RenderResult<()> {
        match val {
            Value::Null => { ctx.keyword("NULL"); }
            Value::Bool(b) => { ctx.keyword(if *b { "TRUE" } else { "FALSE" }); }
            Value::Int(n) => { ctx.write(&n.to_string()); }
            Value::Float(f) => { ctx.write(&f.to_string()); }
            Value::Str(s) => { ctx.string_literal(s); }
            Value::Bytes(b) => {
                ctx.write("'\\x");
                for byte in b {
                    ctx.write(&format!("{byte:02x}"));
                }
                ctx.write("'");
            }
            Value::Date(s) | Value::DateTime(s) | Value::Time(s) => {
                ctx.string_literal(s);
            }
            Value::List(items) => {
                ctx.keyword("ARRAY").write("[");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        ctx.comma();
                    }
                    self.pg_value(item, ctx)?;
                }
                ctx.write("]");
            }
            Value::Decimal(s) => { ctx.write(s); }
            Value::Uuid(s) => { ctx.string_literal(s); }
            Value::TimeDelta { days, seconds, microseconds } => {
                ctx.keyword("INTERVAL");
                let interval = format!("{days} days {seconds} seconds {microseconds} microseconds");
                ctx.string_literal(&interval);
            }
        }
        Ok(())
    }

    fn pg_referential_action(&self, action: &ReferentialAction, ctx: &mut RenderCtx) {
        match action {
            ReferentialAction::NoAction => { ctx.keyword("NO ACTION"); }
            ReferentialAction::Restrict => { ctx.keyword("RESTRICT"); }
            ReferentialAction::Cascade => { ctx.keyword("CASCADE"); }
            ReferentialAction::SetNull(cols) => {
                ctx.keyword("SET NULL");
                if let Some(cols) = cols {
                    ctx.paren_open();
                    self.pg_comma_idents(cols, ctx);
                    ctx.paren_close();
                }
            }
            ReferentialAction::SetDefault(cols) => {
                ctx.keyword("SET DEFAULT");
                if let Some(cols) = cols {
                    ctx.paren_open();
                    self.pg_comma_idents(cols, ctx);
                    ctx.paren_close();
                }
            }
        }
    }

    fn pg_deferrable(&self, def: &DeferrableConstraint, ctx: &mut RenderCtx) {
        if def.deferrable {
            ctx.keyword("DEFERRABLE");
        } else {
            ctx.keyword("NOT DEFERRABLE");
        }
        if def.initially_deferred {
            ctx.keyword("INITIALLY DEFERRED");
        } else {
            ctx.keyword("INITIALLY IMMEDIATE");
        }
    }

    fn pg_identity(&self, identity: &IdentityColumn, ctx: &mut RenderCtx) {
        if identity.always {
            ctx.keyword("GENERATED ALWAYS AS IDENTITY");
        } else {
            ctx.keyword("GENERATED BY DEFAULT AS IDENTITY");
        }
        let has_options = identity.start.is_some()
            || identity.increment.is_some()
            || identity.min_value.is_some()
            || identity.max_value.is_some()
            || identity.cycle
            || identity.cache.is_some();
        if has_options {
            ctx.paren_open();
            if let Some(start) = identity.start {
                ctx.keyword("START WITH").keyword(&start.to_string());
            }
            if let Some(inc) = identity.increment {
                ctx.keyword("INCREMENT BY").keyword(&inc.to_string());
            }
            if let Some(min) = identity.min_value {
                ctx.keyword("MINVALUE").keyword(&min.to_string());
            }
            if let Some(max) = identity.max_value {
                ctx.keyword("MAXVALUE").keyword(&max.to_string());
            }
            if identity.cycle {
                ctx.keyword("CYCLE");
            }
            if let Some(cache) = identity.cache {
                ctx.keyword("CACHE").write(&cache.to_string());
            }
            ctx.paren_close();
        }
    }

    fn pg_create_table(
        &self,
        schema: &SchemaDef,
        if_not_exists: bool,
        temporary: bool,
        unlogged: bool,
        tablespace: Option<&str>,
        ctx: &mut RenderCtx,
    ) -> RenderResult<()> {
        ctx.keyword("CREATE");
        if temporary {
            ctx.keyword("TEMPORARY");
        }
        if unlogged {
            ctx.keyword("UNLOGGED");
        }
        ctx.keyword("TABLE");
        if if_not_exists {
            ctx.keyword("IF NOT EXISTS");
        }
        if let Some(ns) = &schema.namespace {
            ctx.ident(ns).operator(".");
        }
        ctx.ident(&schema.name);

        // Columns + constraints
        ctx.paren_open();
        let mut first = true;
        for col in &schema.columns {
            if !first {
                ctx.comma();
            }
            first = false;
            self.render_column_def(col, ctx)?;
        }
        if let Some(constraints) = &schema.constraints {
            for constraint in constraints {
                if !first {
                    ctx.comma();
                }
                first = false;
                self.render_constraint(constraint, ctx)?;
            }
        }
        ctx.paren_close();

        if let Some(ts) = tablespace {
            ctx.keyword("TABLESPACE").ident(ts);
        }

        Ok(())
    }

    fn pg_create_index(
        &self,
        schema_ref: &rquery_core::ast::common::SchemaRef,
        index: &IndexDef,
        if_not_exists: bool,
        concurrently: bool,
        ctx: &mut RenderCtx,
    ) -> RenderResult<()> {
        ctx.keyword("CREATE");
        if index.unique {
            ctx.keyword("UNIQUE");
        }
        ctx.keyword("INDEX");
        if concurrently {
            ctx.keyword("CONCURRENTLY");
        }
        if if_not_exists {
            ctx.keyword("IF NOT EXISTS");
        }
        ctx.ident(&index.name).keyword("ON");
        self.pg_schema_ref(schema_ref, ctx);

        if let Some(index_type) = &index.index_type {
            ctx.keyword("USING").keyword(index_type);
        }

        ctx.paren_open();
        self.pg_index_columns(&index.columns, ctx)?;
        ctx.paren_close();

        if let Some(include) = &index.include {
            ctx.keyword("INCLUDE").paren_open();
            self.pg_comma_idents(include, ctx);
            ctx.paren_close();
        }

        if let Some(nd) = index.nulls_distinct {
            if !nd {
                ctx.keyword("NULLS NOT DISTINCT");
            }
        }

        if let Some(params) = &index.parameters {
            ctx.keyword("WITH").paren_open();
            for (i, (key, value)) in params.iter().enumerate() {
                if i > 0 {
                    ctx.comma();
                }
                ctx.write(key).write(" = ").write(value);
            }
            ctx.paren_close();
        }

        if let Some(ts) = &index.tablespace {
            ctx.keyword("TABLESPACE").ident(ts);
        }

        if let Some(condition) = &index.condition {
            ctx.keyword("WHERE");
            self.render_condition(condition, ctx)?;
        }

        Ok(())
    }

    fn pg_index_columns(&self, columns: &[IndexColumnDef], ctx: &mut RenderCtx) -> RenderResult<()> {
        for (i, col) in columns.iter().enumerate() {
            if i > 0 {
                ctx.comma();
            }
            match &col.expr {
                IndexExpr::Column(name) => { ctx.ident(name); }
                IndexExpr::Expression(expr) => {
                    ctx.paren_open();
                    self.render_expr(expr, ctx)?;
                    ctx.paren_close();
                }
            }
            if let Some(collation) = &col.collation {
                ctx.keyword("COLLATE").ident(collation);
            }
            if let Some(opclass) = &col.opclass {
                ctx.keyword(opclass);
            }
            if let Some(dir) = col.direction {
                ctx.keyword(match dir {
                    OrderDir::Asc => "ASC",
                    OrderDir::Desc => "DESC",
                });
            }
            if let Some(nulls) = col.nulls {
                ctx.keyword(match nulls {
                    NullsOrder::First => "NULLS FIRST",
                    NullsOrder::Last => "NULLS LAST",
                });
            }
        }
        Ok(())
    }

    fn pg_order_by_list(&self, order_by: &[OrderByDef], ctx: &mut RenderCtx) {
        for (i, ob) in order_by.iter().enumerate() {
            if i > 0 {
                ctx.comma();
            }
            ctx.ident(&ob.field.field.name);
            ctx.keyword(match ob.direction {
                OrderDir::Asc => "ASC",
                OrderDir::Desc => "DESC",
            });
        }
    }

    fn pg_window_frame(&self, frame: &WindowFrameDef, ctx: &mut RenderCtx) {
        ctx.keyword(match frame.frame_type {
            WindowFrameType::Rows => "ROWS",
            WindowFrameType::Range => "RANGE",
            WindowFrameType::Groups => "GROUPS",
        });
        if let Some(end) = &frame.end {
            ctx.keyword("BETWEEN");
            self.pg_frame_bound(&frame.start, ctx);
            ctx.keyword("AND");
            self.pg_frame_bound(end, ctx);
        } else {
            self.pg_frame_bound(&frame.start, ctx);
        }
    }

    fn pg_frame_bound(&self, bound: &WindowFrameBound, ctx: &mut RenderCtx) {
        match bound {
            WindowFrameBound::CurrentRow => { ctx.keyword("CURRENT ROW"); }
            WindowFrameBound::Preceding(None) => { ctx.keyword("UNBOUNDED PRECEDING"); }
            WindowFrameBound::Preceding(Some(n)) => {
                ctx.write(&n.to_string()).keyword("PRECEDING");
            }
            WindowFrameBound::Following(None) => { ctx.keyword("UNBOUNDED FOLLOWING"); }
            WindowFrameBound::Following(Some(n)) => {
                ctx.write(&n.to_string()).keyword("FOLLOWING");
            }
        }
    }
}
