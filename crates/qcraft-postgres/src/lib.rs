use qcraft_core::ast::common::{FieldRef, NullsOrder, OrderByDef, OrderDir, SchemaRef};
use qcraft_core::ast::conditions::{CompareOp, ConditionNode, Conditions, Connector};
use qcraft_core::ast::custom::CustomBinaryOp;
use qcraft_core::ast::ddl::{
    ColumnDef, ConstraintDef, DeferrableConstraint, FieldType, IdentityColumn, IndexColumnDef,
    IndexDef, IndexExpr, LikeTableDef, MatchType, OnCommitAction, PartitionByDef,
    PartitionStrategy, ReferentialAction, SchemaDef, SchemaMutationStmt,
};
use qcraft_core::ast::dml::{
    ConflictAction, ConflictTarget, DeleteStmt, InsertSource, InsertStmt, MutationStmt,
    OnConflictDef, OverridingKind, UpdateStmt,
};
use qcraft_core::ast::expr::{
    AggregationDef, BinaryOp, CaseDef, Expr, UnaryOp, WindowDef, WindowFrameBound, WindowFrameDef,
    WindowFrameType,
};
use qcraft_core::ast::query::{
    CteDef, CteMaterialized, DistinctDef, FromItem, GroupByItem, JoinCondition, JoinDef, JoinType,
    LimitDef, LimitKind, LockStrength, QueryStmt, SampleMethod, SelectColumn, SelectLockDef,
    SetOpDef, SetOperationType, TableSource, WindowNameDef,
};
use qcraft_core::ast::tcl::{
    BeginStmt, CommitStmt, IsolationLevel, LockMode, LockTableStmt, RollbackStmt,
    SetTransactionStmt, TransactionMode, TransactionScope, TransactionStmt,
};
use qcraft_core::ast::value::Value;
use qcraft_core::error::{RenderError, RenderResult};
use qcraft_core::render::ctx::{ParamStyle, RenderCtx};
use qcraft_core::render::escape_like_value;
use qcraft_core::render::renderer::Renderer;

use std::any::Any;

/// pgvector distance operators.
#[derive(Debug, Clone, Copy)]
pub enum PgVectorOp {
    /// L2 (Euclidean) distance: `<->`
    L2Distance,
    /// Inner product (negative): `<#>`
    InnerProduct,
    /// Cosine distance: `<=>`
    CosineDistance,
    /// L1 (Manhattan) distance: `<+>`
    L1Distance,
}

impl CustomBinaryOp for PgVectorOp {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn clone_box(&self) -> Box<dyn CustomBinaryOp> {
        Box::new(*self)
    }
}

impl From<PgVectorOp> for BinaryOp {
    fn from(op: PgVectorOp) -> Self {
        BinaryOp::Custom(Box::new(op))
    }
}

fn render_custom_binary_op(custom: &dyn CustomBinaryOp, ctx: &mut RenderCtx) -> RenderResult<()> {
    if let Some(op) = custom.as_any().downcast_ref::<PgVectorOp>() {
        ctx.write(match op {
            PgVectorOp::L2Distance => " <-> ",
            PgVectorOp::InnerProduct => " <#> ",
            PgVectorOp::CosineDistance => " <=> ",
            PgVectorOp::L1Distance => " <+> ",
        });
        Ok(())
    } else {
        Err(RenderError::unsupported(
            "CustomBinaryOp",
            "unknown custom binary operator; use a wrapping renderer to handle it",
        ))
    }
}

fn render_like_pattern(op: &CompareOp, right: &Expr, ctx: &mut RenderCtx) -> RenderResult<()> {
    let raw = match right {
        Expr::Value(Value::Str(s)) => s.as_str(),
        _ => {
            return Err(RenderError::unsupported(
                "CompareOp",
                "Contains/StartsWith/EndsWith require a string value on the right side",
            ));
        }
    };
    let escaped = escape_like_value(raw);
    let pattern = match op {
        CompareOp::Contains | CompareOp::IContains => format!("%{escaped}%"),
        CompareOp::StartsWith | CompareOp::IStartsWith => format!("{escaped}%"),
        CompareOp::EndsWith | CompareOp::IEndsWith => format!("%{escaped}"),
        _ => unreachable!(),
    };
    if ctx.parameterize() {
        ctx.param(Value::Str(pattern));
    } else {
        ctx.string_literal(&pattern);
    }
    Ok(())
}

struct PgCreateTableOpts<'a> {
    tablespace: Option<&'a str>,
    partition_by: Option<&'a PartitionByDef>,
    inherits: Option<&'a [SchemaRef]>,
    using_method: Option<&'a str>,
    with_options: Option<&'a [(String, String)]>,
    on_commit: Option<&'a OnCommitAction>,
}

pub struct PostgresRenderer {
    param_style: ParamStyle,
}

impl PostgresRenderer {
    pub fn new() -> Self {
        Self {
            param_style: ParamStyle::Dollar,
        }
    }

    /// Use `%s` placeholders (psycopg / DB-API 2.0) instead of `$1`.
    pub fn with_param_style(mut self, style: ParamStyle) -> Self {
        self.param_style = style;
        self
    }

    /// Convenience: render a DDL statement to SQL string + params.
    pub fn render_schema_stmt(
        &self,
        stmt: &SchemaMutationStmt,
    ) -> RenderResult<(String, Vec<Value>)> {
        let mut ctx = RenderCtx::new(self.param_style);
        self.render_schema_mutation(stmt, &mut ctx)?;
        Ok(ctx.finish())
    }

    /// Convenience: render a TCL statement to SQL string + params.
    pub fn render_transaction_stmt(
        &self,
        stmt: &TransactionStmt,
    ) -> RenderResult<(String, Vec<Value>)> {
        let mut ctx = RenderCtx::new(self.param_style);
        self.render_transaction(stmt, &mut ctx)?;
        Ok(ctx.finish())
    }

    /// Convenience: render a DML statement to SQL string + params.
    pub fn render_mutation_stmt(&self, stmt: &MutationStmt) -> RenderResult<(String, Vec<Value>)> {
        let mut ctx = RenderCtx::new(self.param_style).with_parameterize(true);
        self.render_mutation(stmt, &mut ctx)?;
        Ok(ctx.finish())
    }

    /// Convenience: render a SELECT query to SQL string + params.
    pub fn render_query_stmt(&self, stmt: &QueryStmt) -> RenderResult<(String, Vec<Value>)> {
        let mut ctx = RenderCtx::new(self.param_style).with_parameterize(true);
        self.render_query(stmt, &mut ctx)?;
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
                partition_by,
                inherits,
                using_method,
                with_options,
                on_commit,
                table_options: _, // PG uses WITH options instead
                without_rowid: _, // SQLite-specific — Ignore
                strict: _,        // SQLite-specific — Ignore
            } => self.pg_create_table(
                schema,
                *if_not_exists,
                *temporary,
                *unlogged,
                &PgCreateTableOpts {
                    tablespace: tablespace.as_deref(),
                    partition_by: partition_by.as_ref(),
                    inherits: inherits.as_deref(),
                    using_method: using_method.as_deref(),
                    with_options: with_options.as_deref(),
                    on_commit: on_commit.as_ref(),
                },
                ctx,
            ),

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

            SchemaMutationStmt::TruncateTable {
                schema_ref,
                restart_identity,
                cascade,
            } => {
                ctx.keyword("TRUNCATE TABLE");
                self.pg_schema_ref(schema_ref, ctx);
                if *restart_identity {
                    ctx.keyword("RESTART IDENTITY");
                }
                if *cascade {
                    ctx.keyword("CASCADE");
                }
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
                ctx.keyword("RENAME COLUMN")
                    .ident(old_name)
                    .keyword("TO")
                    .ident(new_name);
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
                ctx.keyword("ALTER COLUMN")
                    .ident(column_name)
                    .keyword("SET DATA TYPE");
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
                ctx.keyword("RENAME CONSTRAINT")
                    .ident(old_name)
                    .keyword("TO")
                    .ident(new_name);
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

            SchemaMutationStmt::CreateCollation {
                name,
                if_not_exists,
                locale,
                lc_collate,
                lc_ctype,
                provider,
                deterministic,
                from_collation,
            } => {
                ctx.keyword("CREATE COLLATION");
                if *if_not_exists {
                    ctx.keyword("IF NOT EXISTS");
                }
                ctx.ident(name);
                if let Some(from) = from_collation {
                    ctx.keyword("FROM").ident(from);
                } else {
                    ctx.write(" (");
                    let mut first = true;
                    if let Some(loc) = locale {
                        ctx.keyword("LOCALE").write(" = ").string_literal(loc);
                        first = false;
                    }
                    if let Some(lc) = lc_collate {
                        if !first {
                            ctx.write(", ");
                        }
                        ctx.keyword("LC_COLLATE").write(" = ").string_literal(lc);
                        first = false;
                    }
                    if let Some(lc) = lc_ctype {
                        if !first {
                            ctx.write(", ");
                        }
                        ctx.keyword("LC_CTYPE").write(" = ").string_literal(lc);
                        first = false;
                    }
                    if let Some(prov) = provider {
                        if !first {
                            ctx.write(", ");
                        }
                        ctx.keyword("PROVIDER").write(" = ").keyword(prov);
                        first = false;
                    }
                    if let Some(det) = deterministic {
                        if !first {
                            ctx.write(", ");
                        }
                        ctx.keyword("DETERMINISTIC").write(" = ").keyword(if *det {
                            "TRUE"
                        } else {
                            "FALSE"
                        });
                    }
                    ctx.write(")");
                }
                Ok(())
            }

            SchemaMutationStmt::DropCollation {
                name,
                if_exists,
                cascade,
            } => {
                ctx.keyword("DROP COLLATION");
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

        if let Some(storage) = &col.storage {
            ctx.keyword("STORAGE").keyword(storage);
        }

        if let Some(compression) = &col.compression {
            ctx.keyword("COMPRESSION").keyword(compression);
        }

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
                ctx.keyword(name).write("(");
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
                ctx.keyword("VECTOR")
                    .write("(")
                    .write(&dim.to_string())
                    .paren_close();
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
                autoincrement: _, // SQLite-specific — Ignore
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
                ctx.keyword("EXCLUDE USING")
                    .keyword(index_method)
                    .paren_open();
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        ctx.comma();
                    }
                    ctx.ident(&elem.column)
                        .keyword("WITH")
                        .keyword(&elem.operator);
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
                // When using %s placeholders (psycopg), literal '%' must be
                // escaped as '%%' so the driver doesn't treat it as a placeholder.
                let mod_op = if self.param_style == ParamStyle::Percent {
                    "%%"
                } else {
                    "%"
                };
                match op {
                    BinaryOp::Custom(custom) => {
                        render_custom_binary_op(custom.as_ref(), ctx)?;
                    }
                    _ => {
                        ctx.keyword(match op {
                            BinaryOp::Add => "+",
                            BinaryOp::Sub => "-",
                            BinaryOp::Mul => "*",
                            BinaryOp::Div => "/",
                            BinaryOp::Mod => mod_op,
                            BinaryOp::BitwiseAnd => "&",
                            BinaryOp::BitwiseOr => "|",
                            BinaryOp::ShiftLeft => "<<",
                            BinaryOp::ShiftRight => ">>",
                            BinaryOp::Concat => "||",
                            BinaryOp::Custom(_) => unreachable!(),
                        });
                    }
                };
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
                ctx.keyword(name).write("(");
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

            Expr::Cast {
                expr: inner,
                to_type,
            } => {
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

            Expr::Collate { expr, collation } => {
                self.render_expr(expr, ctx)?;
                ctx.keyword("COLLATE").ident(collation);
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
        ctx.keyword(&agg.name).write("(");
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
            self.pg_order_by_list(order_by, ctx)?;
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
            self.pg_order_by_list(order_by, ctx)?;
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
            CompareOp::Contains | CompareOp::StartsWith | CompareOp::EndsWith => {
                ctx.keyword("LIKE");
                render_like_pattern(op, right, ctx)?;
                return Ok(());
            }
            CompareOp::IContains | CompareOp::IStartsWith | CompareOp::IEndsWith => {
                ctx.keyword("ILIKE");
                render_like_pattern(op, right, ctx)?;
                return Ok(());
            }
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
            CompareOp::TrigramSimilar => {
                if self.param_style == ParamStyle::Percent {
                    ctx.write(" %% ")
                } else {
                    ctx.write(" % ")
                }
            }
            CompareOp::TrigramWordSimilar => {
                if self.param_style == ParamStyle::Percent {
                    ctx.write(" <%% ")
                } else {
                    ctx.write(" <% ")
                }
            }
            CompareOp::TrigramStrictWordSimilar => {
                if self.param_style == ParamStyle::Percent {
                    ctx.write(" <<%% ")
                } else {
                    ctx.write(" <<% ")
                }
            }
            CompareOp::RangeContains => ctx.write(" @> "),
            CompareOp::RangeContainedBy => ctx.write(" <@ "),
            CompareOp::RangeOverlap => ctx.write(" && "),
            CompareOp::RangeStrictlyLeft => ctx.write(" << "),
            CompareOp::RangeStrictlyRight => ctx.write(" >> "),
            CompareOp::RangeNotLeft => ctx.write(" &> "),
            CompareOp::RangeNotRight => ctx.write(" &< "),
            CompareOp::RangeAdjacent => ctx.write(" -|- "),
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

    fn render_query(&self, stmt: &QueryStmt, ctx: &mut RenderCtx) -> RenderResult<()> {
        // CTEs
        if let Some(ctes) = &stmt.ctes {
            self.render_ctes(ctes, ctx)?;
        }

        // SELECT
        ctx.keyword("SELECT");

        // DISTINCT / DISTINCT ON
        if let Some(distinct) = &stmt.distinct {
            match distinct {
                DistinctDef::Distinct => {
                    ctx.keyword("DISTINCT");
                }
                DistinctDef::DistinctOn(exprs) => {
                    ctx.keyword("DISTINCT ON").paren_open();
                    for (i, expr) in exprs.iter().enumerate() {
                        if i > 0 {
                            ctx.comma();
                        }
                        self.render_expr(expr, ctx)?;
                    }
                    ctx.paren_close();
                }
            }
        }

        // Columns
        self.render_select_columns(&stmt.columns, ctx)?;

        // FROM
        if let Some(from) = &stmt.from {
            ctx.keyword("FROM");
            for (i, item) in from.iter().enumerate() {
                if i > 0 {
                    ctx.comma();
                }
                self.pg_render_from_item(item, ctx)?;
            }
        }

        // JOINs
        if let Some(joins) = &stmt.joins {
            self.render_joins(joins, ctx)?;
        }

        // WHERE
        if let Some(cond) = &stmt.where_clause {
            self.render_where(cond, ctx)?;
        }

        // GROUP BY
        if let Some(group_by) = &stmt.group_by {
            self.pg_render_group_by(group_by, ctx)?;
        }

        // HAVING
        if let Some(having) = &stmt.having {
            ctx.keyword("HAVING");
            self.render_condition(having, ctx)?;
        }

        // WINDOW
        if let Some(windows) = &stmt.window {
            self.pg_render_window_clause(windows, ctx)?;
        }

        // ORDER BY
        if let Some(order_by) = &stmt.order_by {
            self.render_order_by(order_by, ctx)?;
        }

        // LIMIT / OFFSET
        if let Some(limit) = &stmt.limit {
            self.render_limit(limit, ctx)?;
        }

        // FOR UPDATE / SHARE
        if let Some(locks) = &stmt.lock {
            for lock in locks {
                self.render_lock(lock, ctx)?;
            }
        }

        Ok(())
    }

    fn render_select_columns(
        &self,
        cols: &[SelectColumn],
        ctx: &mut RenderCtx,
    ) -> RenderResult<()> {
        for (i, col) in cols.iter().enumerate() {
            if i > 0 {
                ctx.comma();
            }
            match col {
                SelectColumn::Star(None) => {
                    ctx.keyword("*");
                }
                SelectColumn::Star(Some(table)) => {
                    ctx.ident(table).operator(".").keyword("*");
                }
                SelectColumn::Expr { expr, alias } => {
                    self.render_expr(expr, ctx)?;
                    if let Some(a) = alias {
                        ctx.keyword("AS").ident(a);
                    }
                }
                SelectColumn::Field { field, alias } => {
                    self.pg_field_ref(field, ctx);
                    if let Some(a) = alias {
                        ctx.keyword("AS").ident(a);
                    }
                }
            }
        }
        Ok(())
    }
    fn render_from(&self, source: &TableSource, ctx: &mut RenderCtx) -> RenderResult<()> {
        match source {
            TableSource::Table(schema_ref) => {
                self.pg_schema_ref(schema_ref, ctx);
                if let Some(alias) = &schema_ref.alias {
                    ctx.keyword("AS").ident(alias);
                }
            }
            TableSource::SubQuery(sq) => {
                ctx.paren_open();
                self.render_query(&sq.query, ctx)?;
                ctx.paren_close().keyword("AS").ident(&sq.alias);
            }
            TableSource::SetOp(set_op) => {
                ctx.paren_open();
                self.pg_render_set_op(set_op, ctx)?;
                ctx.paren_close();
            }
            TableSource::Lateral(inner) => {
                ctx.keyword("LATERAL");
                self.render_from(&inner.source, ctx)?;
            }
            TableSource::Function { name, args, alias } => {
                ctx.keyword(name).write("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        ctx.comma();
                    }
                    self.render_expr(arg, ctx)?;
                }
                ctx.paren_close();
                if let Some(a) = alias {
                    ctx.keyword("AS").ident(a);
                }
            }
            TableSource::Values {
                rows,
                alias,
                column_aliases,
            } => {
                ctx.paren_open().keyword("VALUES");
                for (i, row) in rows.iter().enumerate() {
                    if i > 0 {
                        ctx.comma();
                    }
                    ctx.paren_open();
                    for (j, val) in row.iter().enumerate() {
                        if j > 0 {
                            ctx.comma();
                        }
                        self.render_expr(val, ctx)?;
                    }
                    ctx.paren_close();
                }
                ctx.paren_close().keyword("AS").ident(alias);
                if let Some(cols) = column_aliases {
                    ctx.paren_open();
                    for (i, c) in cols.iter().enumerate() {
                        if i > 0 {
                            ctx.comma();
                        }
                        ctx.ident(c);
                    }
                    ctx.paren_close();
                }
            }
            TableSource::Custom(_) => {
                return Err(RenderError::unsupported(
                    "CustomTableSource",
                    "custom table source must be handled by a wrapping renderer",
                ));
            }
        }
        Ok(())
    }
    fn render_joins(&self, joins: &[JoinDef], ctx: &mut RenderCtx) -> RenderResult<()> {
        for join in joins {
            if join.natural {
                ctx.keyword("NATURAL");
            }
            ctx.keyword(match join.join_type {
                JoinType::Inner => "INNER JOIN",
                JoinType::Left => "LEFT JOIN",
                JoinType::Right => "RIGHT JOIN",
                JoinType::Full => "FULL JOIN",
                JoinType::Cross => "CROSS JOIN",
                JoinType::CrossApply => "CROSS JOIN LATERAL",
                JoinType::OuterApply => "LEFT JOIN LATERAL",
            });
            self.pg_render_from_item(&join.source, ctx)?;
            if let Some(condition) = &join.condition {
                match condition {
                    JoinCondition::On(cond) => {
                        ctx.keyword("ON");
                        self.render_condition(cond, ctx)?;
                    }
                    JoinCondition::Using(cols) => {
                        ctx.keyword("USING").paren_open();
                        self.pg_comma_idents(cols, ctx);
                        ctx.paren_close();
                    }
                }
            }
            // CrossApply/OuterApply rendered as LATERAL need ON TRUE if no condition
            if matches!(join.join_type, JoinType::OuterApply) && join.condition.is_none() {
                ctx.keyword("ON TRUE");
            }
        }
        Ok(())
    }
    fn render_where(&self, cond: &Conditions, ctx: &mut RenderCtx) -> RenderResult<()> {
        ctx.keyword("WHERE");
        self.render_condition(cond, ctx)
    }
    fn render_order_by(&self, order: &[OrderByDef], ctx: &mut RenderCtx) -> RenderResult<()> {
        ctx.keyword("ORDER BY");
        self.pg_order_by_list(order, ctx)
    }
    fn render_limit(&self, limit: &LimitDef, ctx: &mut RenderCtx) -> RenderResult<()> {
        match &limit.kind {
            LimitKind::Limit(n) => {
                ctx.keyword("LIMIT").space().write(&n.to_string());
            }
            LimitKind::FetchFirst {
                count,
                with_ties,
                percent,
            } => {
                if let Some(offset) = limit.offset {
                    ctx.keyword("OFFSET")
                        .space()
                        .write(&offset.to_string())
                        .keyword("ROWS");
                }
                ctx.keyword("FETCH FIRST");
                if *percent {
                    ctx.space().write(&count.to_string()).keyword("PERCENT");
                } else {
                    ctx.space().write(&count.to_string());
                }
                if *with_ties {
                    ctx.keyword("ROWS WITH TIES");
                } else {
                    ctx.keyword("ROWS ONLY");
                }
                return Ok(());
            }
            LimitKind::Top { count, .. } => {
                // PG doesn't support TOP, convert to LIMIT
                ctx.keyword("LIMIT").space().write(&count.to_string());
            }
        }
        if let Some(offset) = limit.offset {
            ctx.keyword("OFFSET").space().write(&offset.to_string());
        }
        Ok(())
    }
    fn render_ctes(&self, ctes: &[CteDef], ctx: &mut RenderCtx) -> RenderResult<()> {
        // Check if any CTE is recursive — PG uses WITH RECURSIVE once for all.
        let any_recursive = ctes.iter().any(|c| c.recursive);
        ctx.keyword("WITH");
        if any_recursive {
            ctx.keyword("RECURSIVE");
        }
        for (i, cte) in ctes.iter().enumerate() {
            if i > 0 {
                ctx.comma();
            }
            ctx.ident(&cte.name);
            if let Some(col_names) = &cte.column_names {
                ctx.paren_open();
                self.pg_comma_idents(col_names, ctx);
                ctx.paren_close();
            }
            ctx.keyword("AS");
            if let Some(mat) = &cte.materialized {
                match mat {
                    CteMaterialized::Materialized => {
                        ctx.keyword("MATERIALIZED");
                    }
                    CteMaterialized::NotMaterialized => {
                        ctx.keyword("NOT MATERIALIZED");
                    }
                }
            }
            ctx.paren_open();
            self.render_query(&cte.query, ctx)?;
            ctx.paren_close();
        }
        Ok(())
    }
    fn render_lock(&self, lock: &SelectLockDef, ctx: &mut RenderCtx) -> RenderResult<()> {
        ctx.keyword("FOR");
        ctx.keyword(match lock.strength {
            LockStrength::Update => "UPDATE",
            LockStrength::NoKeyUpdate => "NO KEY UPDATE",
            LockStrength::Share => "SHARE",
            LockStrength::KeyShare => "KEY SHARE",
        });
        if let Some(of) = &lock.of {
            ctx.keyword("OF");
            for (i, table) in of.iter().enumerate() {
                if i > 0 {
                    ctx.comma();
                }
                self.pg_schema_ref(table, ctx);
            }
        }
        if lock.nowait {
            ctx.keyword("NOWAIT");
        }
        if lock.skip_locked {
            ctx.keyword("SKIP LOCKED");
        }
        Ok(())
    }

    // ── DML ──────────────────────────────────────────────────────────────

    fn render_mutation(&self, stmt: &MutationStmt, ctx: &mut RenderCtx) -> RenderResult<()> {
        match stmt {
            MutationStmt::Insert(s) => self.render_insert(s, ctx),
            MutationStmt::Update(s) => self.render_update(s, ctx),
            MutationStmt::Delete(s) => self.render_delete(s, ctx),
            MutationStmt::Custom(_) => Err(RenderError::unsupported(
                "CustomMutation",
                "custom DML must be handled by a wrapping renderer",
            )),
        }
    }

    fn render_insert(&self, stmt: &InsertStmt, ctx: &mut RenderCtx) -> RenderResult<()> {
        // CTEs
        if let Some(ctes) = &stmt.ctes {
            self.pg_render_ctes(ctes, ctx)?;
        }

        ctx.keyword("INSERT INTO");
        self.pg_schema_ref(&stmt.table, ctx);

        // Column list
        if let Some(cols) = &stmt.columns {
            ctx.paren_open();
            self.pg_comma_idents(cols, ctx);
            ctx.paren_close();
        }

        // OVERRIDING
        if let Some(overriding) = &stmt.overriding {
            ctx.keyword(match overriding {
                OverridingKind::System => "OVERRIDING SYSTEM VALUE",
                OverridingKind::User => "OVERRIDING USER VALUE",
            });
        }

        // Source
        match &stmt.source {
            InsertSource::Values(rows) => {
                ctx.keyword("VALUES");
                for (i, row) in rows.iter().enumerate() {
                    if i > 0 {
                        ctx.comma();
                    }
                    ctx.paren_open();
                    for (j, expr) in row.iter().enumerate() {
                        if j > 0 {
                            ctx.comma();
                        }
                        self.render_expr(expr, ctx)?;
                    }
                    ctx.paren_close();
                }
            }
            InsertSource::Select(query) => {
                self.render_query(query, ctx)?;
            }
            InsertSource::DefaultValues => {
                ctx.keyword("DEFAULT VALUES");
            }
        }

        // ON CONFLICT
        if let Some(conflicts) = &stmt.on_conflict {
            for oc in conflicts {
                self.render_on_conflict(oc, ctx)?;
            }
        }

        // RETURNING
        if let Some(returning) = &stmt.returning {
            self.render_returning(returning, ctx)?;
        }

        Ok(())
    }

    fn render_update(&self, stmt: &UpdateStmt, ctx: &mut RenderCtx) -> RenderResult<()> {
        // CTEs
        if let Some(ctes) = &stmt.ctes {
            self.pg_render_ctes(ctes, ctx)?;
        }

        ctx.keyword("UPDATE");

        // ONLY
        if stmt.only {
            ctx.keyword("ONLY");
        }

        self.pg_schema_ref(&stmt.table, ctx);

        // Alias
        if let Some(alias) = &stmt.table.alias {
            ctx.keyword("AS").ident(alias);
        }

        // SET
        ctx.keyword("SET");
        for (i, (col, expr)) in stmt.assignments.iter().enumerate() {
            if i > 0 {
                ctx.comma();
            }
            ctx.ident(col).write(" = ");
            self.render_expr(expr, ctx)?;
        }

        // FROM
        if let Some(from) = &stmt.from {
            ctx.keyword("FROM");
            for (i, source) in from.iter().enumerate() {
                if i > 0 {
                    ctx.comma();
                }
                self.render_from(source, ctx)?;
            }
        }

        // WHERE
        if let Some(cond) = &stmt.where_clause {
            ctx.keyword("WHERE");
            self.render_condition(cond, ctx)?;
        }

        // RETURNING
        if let Some(returning) = &stmt.returning {
            self.render_returning(returning, ctx)?;
        }

        Ok(())
    }

    fn render_delete(&self, stmt: &DeleteStmt, ctx: &mut RenderCtx) -> RenderResult<()> {
        // CTEs
        if let Some(ctes) = &stmt.ctes {
            self.pg_render_ctes(ctes, ctx)?;
        }

        ctx.keyword("DELETE FROM");

        // ONLY
        if stmt.only {
            ctx.keyword("ONLY");
        }

        self.pg_schema_ref(&stmt.table, ctx);

        // Alias
        if let Some(alias) = &stmt.table.alias {
            ctx.keyword("AS").ident(alias);
        }

        // USING
        if let Some(using) = &stmt.using {
            ctx.keyword("USING");
            for (i, source) in using.iter().enumerate() {
                if i > 0 {
                    ctx.comma();
                }
                self.render_from(source, ctx)?;
            }
        }

        // WHERE
        if let Some(cond) = &stmt.where_clause {
            ctx.keyword("WHERE");
            self.render_condition(cond, ctx)?;
        }

        // RETURNING
        if let Some(returning) = &stmt.returning {
            self.render_returning(returning, ctx)?;
        }

        Ok(())
    }

    fn render_on_conflict(&self, oc: &OnConflictDef, ctx: &mut RenderCtx) -> RenderResult<()> {
        ctx.keyword("ON CONFLICT");

        // Target
        if let Some(target) = &oc.target {
            match target {
                ConflictTarget::Columns {
                    columns,
                    where_clause,
                } => {
                    ctx.paren_open();
                    self.pg_comma_idents(columns, ctx);
                    ctx.paren_close();
                    if let Some(cond) = where_clause {
                        ctx.keyword("WHERE");
                        self.render_condition(cond, ctx)?;
                    }
                }
                ConflictTarget::Constraint(name) => {
                    ctx.keyword("ON CONSTRAINT").ident(name);
                }
            }
        }

        // Action
        match &oc.action {
            ConflictAction::DoNothing => {
                ctx.keyword("DO NOTHING");
            }
            ConflictAction::DoUpdate {
                assignments,
                where_clause,
            } => {
                ctx.keyword("DO UPDATE SET");
                for (i, (col, expr)) in assignments.iter().enumerate() {
                    if i > 0 {
                        ctx.comma();
                    }
                    ctx.ident(col).write(" = ");
                    self.render_expr(expr, ctx)?;
                }
                if let Some(cond) = where_clause {
                    ctx.keyword("WHERE");
                    self.render_condition(cond, ctx)?;
                }
            }
        }

        Ok(())
    }

    fn render_returning(&self, cols: &[SelectColumn], ctx: &mut RenderCtx) -> RenderResult<()> {
        ctx.keyword("RETURNING");
        for (i, col) in cols.iter().enumerate() {
            if i > 0 {
                ctx.comma();
            }
            match col {
                SelectColumn::Star(None) => {
                    ctx.keyword("*");
                }
                SelectColumn::Star(Some(table)) => {
                    ctx.ident(table).operator(".").keyword("*");
                }
                SelectColumn::Expr { expr, alias } => {
                    self.render_expr(expr, ctx)?;
                    if let Some(a) = alias {
                        ctx.keyword("AS").ident(a);
                    }
                }
                SelectColumn::Field { field, alias } => {
                    self.pg_field_ref(field, ctx);
                    if let Some(a) = alias {
                        ctx.keyword("AS").ident(a);
                    }
                }
            }
        }
        Ok(())
    }

    // ── TCL ──────────────────────────────────────────────────────────────

    fn render_transaction(&self, stmt: &TransactionStmt, ctx: &mut RenderCtx) -> RenderResult<()> {
        match stmt {
            TransactionStmt::Begin(s) => self.pg_begin(s, ctx),
            TransactionStmt::Commit(s) => self.pg_commit(s, ctx),
            TransactionStmt::Rollback(s) => self.pg_rollback(s, ctx),
            TransactionStmt::Savepoint(s) => {
                ctx.keyword("SAVEPOINT").ident(&s.name);
                Ok(())
            }
            TransactionStmt::ReleaseSavepoint(s) => {
                ctx.keyword("RELEASE").keyword("SAVEPOINT").ident(&s.name);
                Ok(())
            }
            TransactionStmt::SetTransaction(s) => self.pg_set_transaction(s, ctx),
            TransactionStmt::LockTable(s) => self.pg_lock_table(s, ctx),
            TransactionStmt::PrepareTransaction(s) => {
                ctx.keyword("PREPARE")
                    .keyword("TRANSACTION")
                    .string_literal(&s.transaction_id);
                Ok(())
            }
            TransactionStmt::CommitPrepared(s) => {
                ctx.keyword("COMMIT")
                    .keyword("PREPARED")
                    .string_literal(&s.transaction_id);
                Ok(())
            }
            TransactionStmt::RollbackPrepared(s) => {
                ctx.keyword("ROLLBACK")
                    .keyword("PREPARED")
                    .string_literal(&s.transaction_id);
                Ok(())
            }
            TransactionStmt::Custom(_) => Err(RenderError::unsupported(
                "Custom TCL",
                "not supported by PostgresRenderer",
            )),
        }
    }
}

// ==========================================================================
// PostgreSQL-specific helpers
// ==========================================================================

impl PostgresRenderer {
    // ── TCL helpers ──────────────────────────────────────────────────────

    fn pg_begin(&self, stmt: &BeginStmt, ctx: &mut RenderCtx) -> RenderResult<()> {
        ctx.keyword("BEGIN");
        if let Some(modes) = &stmt.modes {
            self.pg_transaction_modes(modes, ctx);
        }
        Ok(())
    }

    fn pg_commit(&self, stmt: &CommitStmt, ctx: &mut RenderCtx) -> RenderResult<()> {
        ctx.keyword("COMMIT");
        if stmt.and_chain {
            ctx.keyword("AND").keyword("CHAIN");
        }
        Ok(())
    }

    fn pg_rollback(&self, stmt: &RollbackStmt, ctx: &mut RenderCtx) -> RenderResult<()> {
        ctx.keyword("ROLLBACK");
        if let Some(sp) = &stmt.to_savepoint {
            ctx.keyword("TO").keyword("SAVEPOINT").ident(sp);
        }
        if stmt.and_chain {
            ctx.keyword("AND").keyword("CHAIN");
        }
        Ok(())
    }

    fn pg_set_transaction(
        &self,
        stmt: &SetTransactionStmt,
        ctx: &mut RenderCtx,
    ) -> RenderResult<()> {
        ctx.keyword("SET");
        match &stmt.scope {
            Some(TransactionScope::Session) => {
                ctx.keyword("SESSION")
                    .keyword("CHARACTERISTICS")
                    .keyword("AS")
                    .keyword("TRANSACTION");
            }
            _ => {
                ctx.keyword("TRANSACTION");
            }
        }
        if let Some(snap_id) = &stmt.snapshot_id {
            ctx.keyword("SNAPSHOT").string_literal(snap_id);
        } else {
            self.pg_transaction_modes(&stmt.modes, ctx);
        }
        Ok(())
    }

    fn pg_transaction_modes(&self, modes: &[TransactionMode], ctx: &mut RenderCtx) {
        for (i, mode) in modes.iter().enumerate() {
            if i > 0 {
                ctx.comma();
            }
            match mode {
                TransactionMode::IsolationLevel(lvl) => {
                    ctx.keyword("ISOLATION").keyword("LEVEL");
                    ctx.keyword(match lvl {
                        IsolationLevel::ReadUncommitted => "READ UNCOMMITTED",
                        IsolationLevel::ReadCommitted => "READ COMMITTED",
                        IsolationLevel::RepeatableRead => "REPEATABLE READ",
                        IsolationLevel::Serializable => "SERIALIZABLE",
                        IsolationLevel::Snapshot => "SERIALIZABLE", // PG doesn't have SNAPSHOT
                    });
                }
                TransactionMode::ReadOnly => {
                    ctx.keyword("READ ONLY");
                }
                TransactionMode::ReadWrite => {
                    ctx.keyword("READ WRITE");
                }
                TransactionMode::Deferrable => {
                    ctx.keyword("DEFERRABLE");
                }
                TransactionMode::NotDeferrable => {
                    ctx.keyword("NOT DEFERRABLE");
                }
                TransactionMode::WithConsistentSnapshot => {} // MySQL only, skip
            }
        }
    }

    fn pg_lock_table(&self, stmt: &LockTableStmt, ctx: &mut RenderCtx) -> RenderResult<()> {
        ctx.keyword("LOCK").keyword("TABLE");
        for (i, def) in stmt.tables.iter().enumerate() {
            if i > 0 {
                ctx.comma();
            }
            if def.only {
                ctx.keyword("ONLY");
            }
            if let Some(schema) = &def.schema {
                ctx.ident(schema).operator(".");
            }
            ctx.ident(&def.table);
        }
        // Use mode from first table (PG applies one mode to all).
        if let Some(first) = stmt.tables.first() {
            ctx.keyword("IN");
            ctx.keyword(match first.mode {
                LockMode::AccessShare => "ACCESS SHARE",
                LockMode::RowShare => "ROW SHARE",
                LockMode::RowExclusive => "ROW EXCLUSIVE",
                LockMode::ShareUpdateExclusive => "SHARE UPDATE EXCLUSIVE",
                LockMode::Share => "SHARE",
                LockMode::ShareRowExclusive => "SHARE ROW EXCLUSIVE",
                LockMode::Exclusive => "EXCLUSIVE",
                LockMode::AccessExclusive => "ACCESS EXCLUSIVE",
                _ => "ACCESS EXCLUSIVE", // Non-PG modes default
            });
            ctx.keyword("MODE");
        }
        if stmt.nowait {
            ctx.keyword("NOWAIT");
        }
        Ok(())
    }

    // ── Schema helpers ───────────────────────────────────────────────────

    fn pg_schema_ref(&self, schema_ref: &qcraft_core::ast::common::SchemaRef, ctx: &mut RenderCtx) {
        if let Some(ns) = &schema_ref.namespace {
            ctx.ident(ns).operator(".");
        }
        ctx.ident(&schema_ref.name);
    }

    fn pg_field_ref(&self, field_ref: &FieldRef, ctx: &mut RenderCtx) {
        ctx.ident(&field_ref.table_name)
            .operator(".")
            .ident(&field_ref.field.name);
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
        // NULL is always rendered as keyword, never as parameter.
        if matches!(val, Value::Null) {
            ctx.keyword("NULL");
            return Ok(());
        }

        // In parameterized mode, send values as bind parameters (no casts —
        // the driver transmits types via the binary protocol and PG infers
        // from column context).
        if ctx.parameterize() {
            ctx.param(val.clone());
            return Ok(());
        }

        // Inline literal mode (DDL defaults, TCL, etc.)
        self.pg_value_literal(val, ctx)
    }

    fn pg_value_literal(&self, val: &Value, ctx: &mut RenderCtx) -> RenderResult<()> {
        match val {
            Value::Null => {
                ctx.keyword("NULL");
            }
            Value::Bool(b) => {
                ctx.keyword(if *b { "TRUE" } else { "FALSE" });
            }
            Value::Int(n) => {
                ctx.keyword(&n.to_string());
            }
            Value::Float(f) => {
                ctx.keyword(&f.to_string());
            }
            Value::Str(s) => {
                ctx.string_literal(s);
            }
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
            Value::Decimal(s) => {
                ctx.keyword(s);
            }
            Value::Uuid(s) => {
                ctx.string_literal(s);
            }
            Value::Json(s) => {
                ctx.string_literal(s);
                ctx.write("::json");
            }
            Value::Jsonb(s) => {
                ctx.string_literal(s);
                ctx.write("::jsonb");
            }
            Value::IpNetwork(s) => {
                ctx.string_literal(s);
                ctx.write("::inet");
            }
            Value::Array(items) => {
                ctx.keyword("ARRAY").write("[");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        ctx.comma();
                    }
                    self.pg_value_literal(item, ctx)?;
                }
                ctx.write("]");
            }
            Value::Vector(values) => {
                let parts: Vec<String> = values.iter().map(|v| v.to_string()).collect();
                let literal = format!("[{}]", parts.join(","));
                ctx.string_literal(&literal);
                ctx.write("::vector");
            }
            Value::TimeDelta {
                years,
                months,
                days,
                seconds,
                microseconds,
            } => {
                ctx.keyword("INTERVAL");
                let mut parts = Vec::new();
                if *years != 0 {
                    parts.push(format!("{years} years"));
                }
                if *months != 0 {
                    parts.push(format!("{months} months"));
                }
                if *days != 0 {
                    parts.push(format!("{days} days"));
                }
                if *seconds != 0 {
                    parts.push(format!("{seconds} seconds"));
                }
                if *microseconds != 0 {
                    parts.push(format!("{microseconds} microseconds"));
                }
                if parts.is_empty() {
                    parts.push("0 seconds".into());
                }
                ctx.string_literal(&parts.join(" "));
            }
        }
        Ok(())
    }

    fn pg_referential_action(&self, action: &ReferentialAction, ctx: &mut RenderCtx) {
        match action {
            ReferentialAction::NoAction => {
                ctx.keyword("NO ACTION");
            }
            ReferentialAction::Restrict => {
                ctx.keyword("RESTRICT");
            }
            ReferentialAction::Cascade => {
                ctx.keyword("CASCADE");
            }
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

    fn pg_render_ctes(&self, ctes: &[CteDef], ctx: &mut RenderCtx) -> RenderResult<()> {
        // Delegate to the trait method
        self.render_ctes(ctes, ctx)
    }

    fn pg_render_from_item(&self, item: &FromItem, ctx: &mut RenderCtx) -> RenderResult<()> {
        if item.only {
            ctx.keyword("ONLY");
        }
        self.render_from(&item.source, ctx)?;
        if let Some(sample) = &item.sample {
            ctx.keyword("TABLESAMPLE");
            ctx.keyword(match sample.method {
                SampleMethod::Bernoulli => "BERNOULLI",
                SampleMethod::System => "SYSTEM",
                SampleMethod::Block => "SYSTEM", // Block maps to SYSTEM on PG
            });
            ctx.paren_open()
                .write(&sample.percentage.to_string())
                .paren_close();
            if let Some(seed) = sample.seed {
                ctx.keyword("REPEATABLE")
                    .paren_open()
                    .write(&seed.to_string())
                    .paren_close();
            }
        }
        Ok(())
    }

    fn pg_render_group_by(&self, items: &[GroupByItem], ctx: &mut RenderCtx) -> RenderResult<()> {
        ctx.keyword("GROUP BY");
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                ctx.comma();
            }
            match item {
                GroupByItem::Expr(expr) => {
                    self.render_expr(expr, ctx)?;
                }
                GroupByItem::Rollup(exprs) => {
                    ctx.keyword("ROLLUP").paren_open();
                    for (j, expr) in exprs.iter().enumerate() {
                        if j > 0 {
                            ctx.comma();
                        }
                        self.render_expr(expr, ctx)?;
                    }
                    ctx.paren_close();
                }
                GroupByItem::Cube(exprs) => {
                    ctx.keyword("CUBE").paren_open();
                    for (j, expr) in exprs.iter().enumerate() {
                        if j > 0 {
                            ctx.comma();
                        }
                        self.render_expr(expr, ctx)?;
                    }
                    ctx.paren_close();
                }
                GroupByItem::GroupingSets(sets) => {
                    ctx.keyword("GROUPING SETS").paren_open();
                    for (j, set) in sets.iter().enumerate() {
                        if j > 0 {
                            ctx.comma();
                        }
                        ctx.paren_open();
                        for (k, expr) in set.iter().enumerate() {
                            if k > 0 {
                                ctx.comma();
                            }
                            self.render_expr(expr, ctx)?;
                        }
                        ctx.paren_close();
                    }
                    ctx.paren_close();
                }
            }
        }
        Ok(())
    }

    fn pg_render_window_clause(
        &self,
        windows: &[WindowNameDef],
        ctx: &mut RenderCtx,
    ) -> RenderResult<()> {
        ctx.keyword("WINDOW");
        for (i, win) in windows.iter().enumerate() {
            if i > 0 {
                ctx.comma();
            }
            ctx.ident(&win.name).keyword("AS").paren_open();
            if let Some(base) = &win.base_window {
                ctx.ident(base);
            }
            if let Some(partition_by) = &win.partition_by {
                ctx.keyword("PARTITION BY");
                for (j, expr) in partition_by.iter().enumerate() {
                    if j > 0 {
                        ctx.comma();
                    }
                    self.render_expr(expr, ctx)?;
                }
            }
            if let Some(order_by) = &win.order_by {
                ctx.keyword("ORDER BY");
                self.pg_order_by_list(order_by, ctx)?;
            }
            if let Some(frame) = &win.frame {
                self.pg_window_frame(frame, ctx);
            }
            ctx.paren_close();
        }
        Ok(())
    }

    fn pg_render_set_op(&self, set_op: &SetOpDef, ctx: &mut RenderCtx) -> RenderResult<()> {
        self.render_query(&set_op.left, ctx)?;
        ctx.keyword(match set_op.operation {
            SetOperationType::Union => "UNION",
            SetOperationType::UnionAll => "UNION ALL",
            SetOperationType::Intersect => "INTERSECT",
            SetOperationType::IntersectAll => "INTERSECT ALL",
            SetOperationType::Except => "EXCEPT",
            SetOperationType::ExceptAll => "EXCEPT ALL",
        });
        self.render_query(&set_op.right, ctx)
    }

    fn pg_create_table(
        &self,
        schema: &SchemaDef,
        if_not_exists: bool,
        temporary: bool,
        unlogged: bool,
        opts: &PgCreateTableOpts<'_>,
        ctx: &mut RenderCtx,
    ) -> RenderResult<()> {
        let PgCreateTableOpts {
            tablespace,
            partition_by,
            inherits,
            using_method,
            with_options,
            on_commit,
        } = opts;
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

        // Columns + constraints + LIKE
        ctx.paren_open();
        let mut first = true;
        for col in &schema.columns {
            if !first {
                ctx.comma();
            }
            first = false;
            self.render_column_def(col, ctx)?;
        }
        if let Some(like_tables) = &schema.like_tables {
            for like in like_tables {
                if !first {
                    ctx.comma();
                }
                first = false;
                self.pg_like_table(like, ctx);
            }
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

        // INHERITS
        if let Some(parents) = inherits {
            ctx.keyword("INHERITS").paren_open();
            for (i, parent) in parents.iter().enumerate() {
                if i > 0 {
                    ctx.comma();
                }
                self.pg_schema_ref(parent, ctx);
            }
            ctx.paren_close();
        }

        // PARTITION BY
        if let Some(part) = partition_by {
            ctx.keyword("PARTITION BY");
            ctx.keyword(match part.strategy {
                PartitionStrategy::Range => "RANGE",
                PartitionStrategy::List => "LIST",
                PartitionStrategy::Hash => "HASH",
            });
            ctx.paren_open();
            for (i, col) in part.columns.iter().enumerate() {
                if i > 0 {
                    ctx.comma();
                }
                match &col.expr {
                    IndexExpr::Column(name) => {
                        ctx.ident(name);
                    }
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
            }
            ctx.paren_close();
        }

        // USING method
        if let Some(method) = using_method {
            ctx.keyword("USING").keyword(method);
        }

        // WITH (storage_parameter = value, ...)
        if let Some(opts) = with_options {
            ctx.keyword("WITH").paren_open();
            for (i, (key, value)) in opts.iter().enumerate() {
                if i > 0 {
                    ctx.comma();
                }
                ctx.write(key).write(" = ").write(value);
            }
            ctx.paren_close();
        }

        // ON COMMIT
        if let Some(action) = on_commit {
            ctx.keyword("ON COMMIT");
            ctx.keyword(match action {
                OnCommitAction::PreserveRows => "PRESERVE ROWS",
                OnCommitAction::DeleteRows => "DELETE ROWS",
                OnCommitAction::Drop => "DROP",
            });
        }

        // TABLESPACE
        if let Some(ts) = tablespace {
            ctx.keyword("TABLESPACE").ident(ts);
        }

        Ok(())
    }

    fn pg_like_table(&self, like: &LikeTableDef, ctx: &mut RenderCtx) {
        ctx.keyword("LIKE");
        self.pg_schema_ref(&like.source_table, ctx);
        for opt in &like.options {
            if opt.include {
                ctx.keyword("INCLUDING");
            } else {
                ctx.keyword("EXCLUDING");
            }
            ctx.keyword(match opt.kind {
                qcraft_core::ast::ddl::LikeOptionKind::Comments => "COMMENTS",
                qcraft_core::ast::ddl::LikeOptionKind::Compression => "COMPRESSION",
                qcraft_core::ast::ddl::LikeOptionKind::Constraints => "CONSTRAINTS",
                qcraft_core::ast::ddl::LikeOptionKind::Defaults => "DEFAULTS",
                qcraft_core::ast::ddl::LikeOptionKind::Generated => "GENERATED",
                qcraft_core::ast::ddl::LikeOptionKind::Identity => "IDENTITY",
                qcraft_core::ast::ddl::LikeOptionKind::Indexes => "INDEXES",
                qcraft_core::ast::ddl::LikeOptionKind::Statistics => "STATISTICS",
                qcraft_core::ast::ddl::LikeOptionKind::Storage => "STORAGE",
                qcraft_core::ast::ddl::LikeOptionKind::All => "ALL",
            });
        }
    }

    fn pg_create_index(
        &self,
        schema_ref: &qcraft_core::ast::common::SchemaRef,
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

    fn pg_index_columns(
        &self,
        columns: &[IndexColumnDef],
        ctx: &mut RenderCtx,
    ) -> RenderResult<()> {
        for (i, col) in columns.iter().enumerate() {
            if i > 0 {
                ctx.comma();
            }
            match &col.expr {
                IndexExpr::Column(name) => {
                    ctx.ident(name);
                }
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

    fn pg_order_by_list(&self, order_by: &[OrderByDef], ctx: &mut RenderCtx) -> RenderResult<()> {
        for (i, ob) in order_by.iter().enumerate() {
            if i > 0 {
                ctx.comma();
            }
            self.render_expr(&ob.expr, ctx)?;
            ctx.keyword(match ob.direction {
                OrderDir::Asc => "ASC",
                OrderDir::Desc => "DESC",
            });
            if let Some(nulls) = &ob.nulls {
                ctx.keyword(match nulls {
                    NullsOrder::First => "NULLS FIRST",
                    NullsOrder::Last => "NULLS LAST",
                });
            }
        }
        Ok(())
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
            WindowFrameBound::CurrentRow => {
                ctx.keyword("CURRENT ROW");
            }
            WindowFrameBound::Preceding(None) => {
                ctx.keyword("UNBOUNDED PRECEDING");
            }
            WindowFrameBound::Preceding(Some(n)) => {
                ctx.keyword(&n.to_string()).keyword("PRECEDING");
            }
            WindowFrameBound::Following(None) => {
                ctx.keyword("UNBOUNDED FOLLOWING");
            }
            WindowFrameBound::Following(Some(n)) => {
                ctx.keyword(&n.to_string()).keyword("FOLLOWING");
            }
        }
    }
}
