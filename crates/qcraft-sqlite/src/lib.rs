use qcraft_core::ast::common::{FieldRef, NullsOrder, OrderByDef, OrderDir};
use qcraft_core::ast::conditions::{CompareOp, ConditionNode, Conditions, Connector};
use qcraft_core::ast::ddl::{
    ColumnDef, ConstraintDef, DeferrableConstraint, FieldType, IndexColumnDef, IndexDef, IndexExpr,
    ReferentialAction, SchemaDef, SchemaMutationStmt,
};
use qcraft_core::ast::dml::{
    ConflictAction, ConflictResolution, ConflictTarget, DeleteStmt, InsertSource, InsertStmt,
    MutationStmt, OnConflictDef, UpdateStmt,
};
use qcraft_core::ast::expr::{
    AggregationDef, BinaryOp, CaseDef, Expr, UnaryOp, WindowDef, WindowFrameBound, WindowFrameDef,
    WindowFrameType,
};
use qcraft_core::ast::query::{
    CteDef, DistinctDef, FromItem, GroupByItem, JoinCondition, JoinDef, JoinType, LimitDef,
    LimitKind, QueryStmt, SelectColumn, SelectLockDef, SetOpDef, SetOperationType, SqliteIndexHint,
    TableSource, WindowNameDef,
};
use qcraft_core::ast::tcl::{SqliteLockType, TransactionStmt};
use qcraft_core::ast::value::Value;
use qcraft_core::error::{RenderError, RenderResult};
use qcraft_core::render::ctx::{ParamStyle, RenderCtx};
use qcraft_core::render::escape_like_value;
use qcraft_core::render::renderer::Renderer;

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

pub struct SqliteRenderer;

impl SqliteRenderer {
    pub fn new() -> Self {
        Self
    }

    pub fn render_schema_stmt(
        &self,
        stmt: &SchemaMutationStmt,
    ) -> RenderResult<Vec<(String, Vec<Value>)>> {
        let mut ctx = RenderCtx::new(ParamStyle::QMark);
        self.render_schema_mutation(stmt, &mut ctx)?;
        Ok(vec![ctx.finish()])
    }

    pub fn render_transaction_stmt(
        &self,
        stmt: &TransactionStmt,
    ) -> RenderResult<(String, Vec<Value>)> {
        let mut ctx = RenderCtx::new(ParamStyle::QMark);
        self.render_transaction(stmt, &mut ctx)?;
        Ok(ctx.finish())
    }

    pub fn render_mutation_stmt(&self, stmt: &MutationStmt) -> RenderResult<(String, Vec<Value>)> {
        let mut ctx = RenderCtx::new(ParamStyle::QMark).with_parameterize(true);
        self.render_mutation(stmt, &mut ctx)?;
        Ok(ctx.finish())
    }

    pub fn render_query_stmt(&self, stmt: &QueryStmt) -> RenderResult<(String, Vec<Value>)> {
        let mut ctx = RenderCtx::new(ParamStyle::QMark).with_parameterize(true);
        self.render_query(stmt, &mut ctx)?;
        Ok(ctx.finish())
    }
}

impl Default for SqliteRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// ==========================================================================
// Renderer trait implementation
// ==========================================================================

impl Renderer for SqliteRenderer {
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
                unlogged: _,
                tablespace: _,
                partition_by: _,  // SQLite doesn't support PARTITION BY
                inherits: _,      // SQLite doesn't support INHERITS
                using_method: _,  // SQLite doesn't support USING method
                with_options: _,  // SQLite doesn't support WITH options
                on_commit: _,     // SQLite doesn't support ON COMMIT
                table_options: _, // SQLite doesn't support generic table options
                without_rowid,
                strict,
            } => self.sqlite_create_table(
                schema,
                *if_not_exists,
                *temporary,
                *without_rowid,
                *strict,
                ctx,
            ),

            SchemaMutationStmt::DropTable {
                schema_ref,
                if_exists,
                cascade: _, // SQLite doesn't support CASCADE — Ignore
            } => {
                ctx.keyword("DROP TABLE");
                if *if_exists {
                    ctx.keyword("IF EXISTS");
                }
                self.sqlite_schema_ref(schema_ref, ctx);
                Ok(())
            }

            SchemaMutationStmt::RenameTable {
                schema_ref,
                new_name,
            } => {
                ctx.keyword("ALTER TABLE");
                self.sqlite_schema_ref(schema_ref, ctx);
                ctx.keyword("RENAME TO").ident(new_name);
                Ok(())
            }

            SchemaMutationStmt::TruncateTable {
                schema_ref,
                restart_identity: _, // SQLite doesn't have RESTART IDENTITY
                cascade: _,          // SQLite doesn't support CASCADE
            } => {
                // SQLite has no TRUNCATE — use DELETE FROM (equivalent semantics)
                ctx.keyword("DELETE FROM");
                self.sqlite_schema_ref(schema_ref, ctx);
                Ok(())
            }

            SchemaMutationStmt::AddColumn {
                schema_ref,
                column,
                if_not_exists: _, // SQLite ADD COLUMN doesn't support IF NOT EXISTS
                position: _,      // SQLite doesn't support FIRST/AFTER
            } => {
                ctx.keyword("ALTER TABLE");
                self.sqlite_schema_ref(schema_ref, ctx);
                ctx.keyword("ADD COLUMN");
                self.render_column_def(column, ctx)
            }

            SchemaMutationStmt::DropColumn {
                schema_ref,
                name,
                if_exists: _, // SQLite DROP COLUMN doesn't support IF EXISTS
                cascade: _,   // SQLite doesn't support CASCADE
            } => {
                ctx.keyword("ALTER TABLE");
                self.sqlite_schema_ref(schema_ref, ctx);
                ctx.keyword("DROP COLUMN").ident(name);
                Ok(())
            }

            SchemaMutationStmt::RenameColumn {
                schema_ref,
                old_name,
                new_name,
            } => {
                ctx.keyword("ALTER TABLE");
                self.sqlite_schema_ref(schema_ref, ctx);
                ctx.keyword("RENAME COLUMN")
                    .ident(old_name)
                    .keyword("TO")
                    .ident(new_name);
                Ok(())
            }

            // SQLite does NOT support these ALTER operations — Error
            SchemaMutationStmt::AlterColumnType { .. } => Err(RenderError::unsupported(
                "AlterColumnType",
                "SQLite does not support ALTER COLUMN TYPE. Use the 12-step table rebuild procedure.",
            )),
            SchemaMutationStmt::AlterColumnDefault { .. } => Err(RenderError::unsupported(
                "AlterColumnDefault",
                "SQLite does not support ALTER COLUMN DEFAULT. Use the 12-step table rebuild procedure.",
            )),
            SchemaMutationStmt::AlterColumnNullability { .. } => Err(RenderError::unsupported(
                "AlterColumnNullability",
                "SQLite does not support ALTER COLUMN NOT NULL. Use the 12-step table rebuild procedure.",
            )),
            SchemaMutationStmt::AddConstraint { .. } => Err(RenderError::unsupported(
                "AddConstraint",
                "SQLite does not support ADD CONSTRAINT. Use the 12-step table rebuild procedure.",
            )),
            SchemaMutationStmt::DropConstraint { .. } => Err(RenderError::unsupported(
                "DropConstraint",
                "SQLite does not support DROP CONSTRAINT. Use the 12-step table rebuild procedure.",
            )),
            SchemaMutationStmt::RenameConstraint { .. } => Err(RenderError::unsupported(
                "RenameConstraint",
                "SQLite does not support RENAME CONSTRAINT.",
            )),
            SchemaMutationStmt::ValidateConstraint { .. } => Err(RenderError::unsupported(
                "ValidateConstraint",
                "SQLite does not support VALIDATE CONSTRAINT.",
            )),

            // ── Index operations ──
            SchemaMutationStmt::CreateIndex {
                schema_ref,
                index,
                if_not_exists,
                concurrently: _, // SQLite doesn't support CONCURRENTLY — Ignore
            } => self.sqlite_create_index(schema_ref, index, *if_not_exists, ctx),

            SchemaMutationStmt::DropIndex {
                schema_ref: _,
                index_name,
                if_exists,
                concurrently: _, // Ignore
                cascade: _,      // Ignore
            } => {
                ctx.keyword("DROP INDEX");
                if *if_exists {
                    ctx.keyword("IF EXISTS");
                }
                ctx.ident(index_name);
                Ok(())
            }

            // SQLite doesn't have extensions
            SchemaMutationStmt::CreateExtension { .. } => Err(RenderError::unsupported(
                "CreateExtension",
                "SQLite does not support extensions.",
            )),
            SchemaMutationStmt::DropExtension { .. } => Err(RenderError::unsupported(
                "DropExtension",
                "SQLite does not support extensions.",
            )),

            SchemaMutationStmt::CreateCollation { .. } => Err(RenderError::unsupported(
                "CreateCollation",
                "SQLite does not support CREATE COLLATION. Use sqlite3_create_collation() C API instead.",
            )),
            SchemaMutationStmt::DropCollation { .. } => Err(RenderError::unsupported(
                "DropCollation",
                "SQLite does not support DROP COLLATION.",
            )),

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
            ctx.keyword("DEFAULT").paren_open();
            self.render_expr(default, ctx)?;
            ctx.paren_close();
        }

        // Identity is handled at CREATE TABLE level (rendered as PRIMARY KEY AUTOINCREMENT inline)
        // Nothing to render here — just skip

        if let Some(generated) = &col.generated {
            ctx.keyword("GENERATED ALWAYS AS").space().paren_open();
            // SQLite generated columns only allow unqualified column names
            self.render_expr_unqualified(&generated.expr, ctx)?;
            ctx.paren_close();
            if generated.stored {
                ctx.keyword("STORED");
            } else {
                ctx.keyword("VIRTUAL");
            }
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
            FieldType::Array(_) => {
                return Err(RenderError::unsupported(
                    "ArrayType",
                    "SQLite does not support array types.",
                ));
            }
            FieldType::Vector(_) => {
                return Err(RenderError::unsupported(
                    "VectorType",
                    "SQLite does not support vector types.",
                ));
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
                include: _, // SQLite doesn't support INCLUDE — Ignore
            } => {
                if let Some(n) = name {
                    ctx.keyword("CONSTRAINT").ident(n);
                }
                ctx.keyword("PRIMARY KEY").paren_open();
                self.sqlite_comma_idents(columns, ctx);
                ctx.paren_close();
            }

            ConstraintDef::ForeignKey {
                name,
                columns,
                ref_table,
                ref_columns,
                on_delete,
                on_update,
                deferrable,
                match_type: _, // SQLite accepts MATCH but it's a no-op — Ignore
            } => {
                if let Some(n) = name {
                    ctx.keyword("CONSTRAINT").ident(n);
                }
                ctx.keyword("FOREIGN KEY").paren_open();
                self.sqlite_comma_idents(columns, ctx);
                ctx.paren_close().keyword("REFERENCES");
                self.sqlite_schema_ref(ref_table, ctx);
                ctx.paren_open();
                self.sqlite_comma_idents(ref_columns, ctx);
                ctx.paren_close();
                if let Some(action) = on_delete {
                    ctx.keyword("ON DELETE");
                    self.sqlite_referential_action(action, ctx)?;
                }
                if let Some(action) = on_update {
                    ctx.keyword("ON UPDATE");
                    self.sqlite_referential_action(action, ctx)?;
                }
                if let Some(def) = deferrable {
                    self.sqlite_deferrable(def, ctx);
                }
            }

            ConstraintDef::Unique {
                name,
                columns,
                include: _,        // Ignore
                nulls_distinct: _, // Ignore
                condition: _,      // Ignore
            } => {
                if let Some(n) = name {
                    ctx.keyword("CONSTRAINT").ident(n);
                }
                ctx.keyword("UNIQUE").paren_open();
                self.sqlite_comma_idents(columns, ctx);
                ctx.paren_close();
            }

            ConstraintDef::Check {
                name,
                condition,
                no_inherit: _, // Ignore
                enforced: _,   // Ignore
            } => {
                if let Some(n) = name {
                    ctx.keyword("CONSTRAINT").ident(n);
                }
                ctx.keyword("CHECK").paren_open();
                self.render_condition(condition, ctx)?;
                ctx.paren_close();
            }

            ConstraintDef::Exclusion { .. } => {
                return Err(RenderError::unsupported(
                    "ExclusionConstraint",
                    "SQLite does not support EXCLUDE constraints.",
                ));
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
        ctx.ident(&idx.name);
        ctx.paren_open();
        self.sqlite_index_columns(&idx.columns, ctx)?;
        ctx.paren_close();
        Ok(())
    }

    // ── Expressions (basic, needed for DDL) ──────────────────────────────

    fn render_expr(&self, expr: &Expr, ctx: &mut RenderCtx) -> RenderResult<()> {
        match expr {
            Expr::Value(val) => self.sqlite_value(val, ctx),

            Expr::Field(field_ref) => {
                self.sqlite_field_ref(field_ref, ctx);
                Ok(())
            }

            Expr::Binary { left, op, right } => {
                self.render_expr(left, ctx)?;
                match op {
                    BinaryOp::Custom(_) => {
                        return Err(RenderError::unsupported(
                            "CustomBinaryOp",
                            "SQLite does not support custom binary operators.",
                        ));
                    }
                    _ => {
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
                ctx.keyword("CAST").write("(");
                self.render_expr(inner, ctx)?;
                ctx.keyword("AS").keyword(to_type).paren_close();
                Ok(())
            }

            Expr::Case(case) => self.render_case(case, ctx),
            Expr::Window(win) => self.render_window(win, ctx),

            Expr::Exists(query) => {
                ctx.keyword("EXISTS").write("(");
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

            Expr::ArraySubQuery(_) => Err(RenderError::unsupported(
                "ArraySubQuery",
                "SQLite does not support ARRAY subqueries.",
            )),

            Expr::Collate { expr, collation } => {
                self.render_expr(expr, ctx)?;
                ctx.keyword("COLLATE").keyword(collation);
                Ok(())
            }

            Expr::JsonArray(items) => {
                ctx.keyword("json_array").write("(");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        ctx.comma();
                    }
                    self.render_expr(item, ctx)?;
                }
                ctx.paren_close();
                Ok(())
            }

            Expr::JsonObject(pairs) => {
                ctx.keyword("json_object").write("(");
                for (i, (key, val)) in pairs.iter().enumerate() {
                    if i > 0 {
                        ctx.comma();
                    }
                    ctx.string_literal(key).comma();
                    self.render_expr(val, ctx)?;
                }
                ctx.paren_close();
                Ok(())
            }

            Expr::JsonAgg {
                expr,
                distinct,
                filter,
                order_by,
            } => {
                ctx.keyword("json_group_array").write("(");
                if *distinct {
                    ctx.keyword("DISTINCT");
                }
                self.render_expr(expr, ctx)?;
                if let Some(ob) = order_by {
                    ctx.keyword("ORDER BY");
                    self.sqlite_order_by_list(ob, ctx)?;
                }
                ctx.paren_close();
                if let Some(f) = filter {
                    ctx.keyword("FILTER").paren_open().keyword("WHERE");
                    self.render_condition(f, ctx)?;
                    ctx.paren_close();
                }
                Ok(())
            }

            Expr::StringAgg {
                expr,
                delimiter,
                distinct,
                filter,
                order_by,
            } => {
                ctx.keyword("group_concat").write("(");
                if *distinct {
                    ctx.keyword("DISTINCT");
                }
                self.render_expr(expr, ctx)?;
                ctx.comma().string_literal(delimiter);
                if let Some(ob) = order_by {
                    ctx.keyword("ORDER BY");
                    self.sqlite_order_by_list(ob, ctx)?;
                }
                ctx.paren_close();
                if let Some(f) = filter {
                    ctx.keyword("FILTER").paren_open().keyword("WHERE");
                    self.render_condition(f, ctx)?;
                    ctx.paren_close();
                }
                Ok(())
            }

            Expr::Now => {
                ctx.keyword("datetime")
                    .write("(")
                    .string_literal("now")
                    .paren_close();
                Ok(())
            }

            Expr::CurrentTimestamp => {
                ctx.keyword("CURRENT_TIMESTAMP");
                Ok(())
            }
            Expr::CurrentDate => {
                ctx.keyword("CURRENT_DATE");
                Ok(())
            }
            Expr::CurrentTime => {
                ctx.keyword("CURRENT_TIME");
                Ok(())
            }

            Expr::JsonPathText { expr, path } => {
                self.render_expr(expr, ctx)?;
                ctx.operator("->>'")
                    .write(&path.replace('\'', "''"))
                    .write("'");
                Ok(())
            }

            Expr::Tuple(exprs) => {
                ctx.paren_open();
                for (i, expr) in exprs.iter().enumerate() {
                    if i > 0 {
                        ctx.comma();
                    }
                    self.render_expr(expr, ctx)?;
                }
                ctx.paren_close();
                Ok(())
            }

            Expr::Param { type_hint: _ } => {
                ctx.placeholder();
                Ok(())
            }

            Expr::Raw { sql, params } => {
                if params.is_empty() {
                    ctx.keyword(sql);
                } else {
                    ctx.raw_with_params(sql, params);
                }
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
            self.sqlite_order_by_list(order_by, ctx)?;
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
            self.sqlite_order_by_list(order_by, ctx)?;
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
        // Special case: negated + single Exists child → NOT EXISTS (...)
        if cond.negated
            && cond.children.len() == 1
            && matches!(cond.children[0], ConditionNode::Exists(_))
        {
            if let ConditionNode::Exists(query) = &cond.children[0] {
                ctx.keyword("NOT EXISTS").write("(");
                self.render_query(query, ctx)?;
                ctx.paren_close();
                return Ok(());
            }
        }

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
                    ctx.keyword("EXISTS").write("(");
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
        let needs_lower = matches!(
            op,
            CompareOp::ILike | CompareOp::IContains | CompareOp::IStartsWith | CompareOp::IEndsWith
        );
        if needs_lower {
            ctx.keyword("LOWER").write("(");
        }
        self.render_expr(left, ctx)?;
        if needs_lower {
            ctx.paren_close();
        }
        match op {
            CompareOp::Eq => ctx.write(" = "),
            CompareOp::Neq => ctx.write(" <> "),
            CompareOp::Gt => ctx.write(" > "),
            CompareOp::Gte => ctx.write(" >= "),
            CompareOp::Lt => ctx.write(" < "),
            CompareOp::Lte => ctx.write(" <= "),
            CompareOp::Like => ctx.keyword("LIKE"),
            CompareOp::Contains | CompareOp::StartsWith | CompareOp::EndsWith => {
                ctx.keyword("LIKE");
                render_like_pattern(op, right, ctx)?;
                ctx.keyword("ESCAPE").string_literal("\\");
                return Ok(());
            }
            CompareOp::IContains | CompareOp::IStartsWith | CompareOp::IEndsWith => {
                ctx.keyword("LIKE");
                ctx.keyword("LOWER").write("(");
                render_like_pattern(op, right, ctx)?;
                ctx.paren_close();
                ctx.keyword("ESCAPE").string_literal("\\");
                return Ok(());
            }
            CompareOp::In => {
                if let Expr::Value(Value::Array(items)) = right {
                    ctx.keyword("IN").paren_open();
                    for (i, item) in items.iter().enumerate() {
                        if i > 0 {
                            ctx.comma();
                        }
                        self.sqlite_value(item, ctx)?;
                    }
                    ctx.paren_close();
                } else {
                    ctx.keyword("IN");
                    self.render_expr(right, ctx)?;
                }
                return Ok(());
            }
            CompareOp::Between => {
                ctx.keyword("BETWEEN");
                if let Expr::Value(Value::Array(items)) = right {
                    if items.len() == 2 {
                        self.sqlite_value(&items[0], ctx)?;
                        ctx.keyword("AND");
                        self.sqlite_value(&items[1], ctx)?;
                    } else {
                        return Err(RenderError::unsupported(
                            "Between",
                            "BETWEEN requires exactly 2 values",
                        ));
                    }
                } else {
                    self.render_expr(right, ctx)?;
                }
                return Ok(());
            }
            CompareOp::IsNull => {
                ctx.keyword("IS NULL");
                return Ok(());
            }
            CompareOp::Regex => ctx.keyword("REGEXP"),
            CompareOp::IRegex => {
                ctx.keyword("REGEXP").string_literal("(?i)").keyword("||");
                self.render_expr(right, ctx)?;
                return Ok(());
            }
            CompareOp::ILike => {
                ctx.keyword("LIKE").keyword("LOWER").write("(");
                self.render_expr(right, ctx)?;
                ctx.paren_close();
                return Ok(());
            }
            // SQLite doesn't natively support this — Error
            CompareOp::Similar => {
                return Err(RenderError::unsupported(
                    "CompareOp",
                    "SQLite does not support SIMILAR TO.",
                ));
            }
            CompareOp::JsonbContains
            | CompareOp::JsonbContainedBy
            | CompareOp::JsonbHasKey
            | CompareOp::JsonbHasAnyKey
            | CompareOp::JsonbHasAllKeys
            | CompareOp::FtsMatch
            | CompareOp::TrigramSimilar
            | CompareOp::TrigramWordSimilar
            | CompareOp::TrigramStrictWordSimilar
            | CompareOp::RangeContains
            | CompareOp::RangeContainedBy
            | CompareOp::RangeOverlap
            | CompareOp::RangeStrictlyLeft
            | CompareOp::RangeStrictlyRight
            | CompareOp::RangeNotLeft
            | CompareOp::RangeNotRight
            | CompareOp::RangeAdjacent => {
                return Err(RenderError::unsupported(
                    "CompareOp",
                    "SQLite does not support PostgreSQL-specific operators (JSONB, FTS, trigram, range).",
                ));
            }
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

        // Set operation: render directly without SELECT wrapper
        if let Some(set_op) = &stmt.set_op {
            return self.sqlite_render_set_op(set_op, ctx);
        }

        // SELECT
        ctx.keyword("SELECT");

        // DISTINCT
        if let Some(distinct) = &stmt.distinct {
            match distinct {
                DistinctDef::Distinct => {
                    ctx.keyword("DISTINCT");
                }
                DistinctDef::DistinctOn(_) => {
                    return Err(RenderError::unsupported(
                        "DISTINCT ON",
                        "not supported in SQLite",
                    ));
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
                self.sqlite_render_from_item(item, ctx)?;
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
            self.sqlite_render_group_by(group_by, ctx)?;
        }

        // HAVING
        if let Some(having) = &stmt.having {
            ctx.keyword("HAVING");
            self.render_condition(having, ctx)?;
        }

        // WINDOW
        if let Some(windows) = &stmt.window {
            self.sqlite_render_window_clause(windows, ctx)?;
        }

        // ORDER BY
        if let Some(order_by) = &stmt.order_by {
            self.render_order_by(order_by, ctx)?;
        }

        // LIMIT / OFFSET
        if let Some(limit) = &stmt.limit {
            self.render_limit(limit, ctx)?;
        }

        // FOR UPDATE — not supported in SQLite
        if let Some(locks) = &stmt.lock {
            if !locks.is_empty() {
                return Err(RenderError::unsupported(
                    "FOR UPDATE/SHARE",
                    "row locking not supported in SQLite",
                ));
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
                    self.sqlite_field_ref(field, ctx);
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
                self.sqlite_schema_ref(schema_ref, ctx);
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
                self.sqlite_render_set_op(set_op, ctx)?;
                ctx.paren_close();
            }
            TableSource::Lateral(_) => {
                return Err(RenderError::unsupported(
                    "LATERAL",
                    "LATERAL subqueries not supported in SQLite",
                ));
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
                columns,
            } => {
                // SQLite does not support AS t(col1, col2) syntax.
                // Wrap in: (SELECT column1 AS "c1", column2 AS "c2"
                //           FROM (VALUES (...), (...))) AS "t"
                ctx.paren_open().keyword("SELECT");
                for (i, c) in columns.iter().enumerate() {
                    if i > 0 {
                        ctx.comma();
                    }
                    ctx.keyword(&format!("column{}", i + 1))
                        .keyword("AS")
                        .ident(c);
                }
                ctx.keyword("FROM").paren_open().keyword("VALUES");
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
                ctx.paren_close().paren_close().keyword("AS").ident(alias);
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
                JoinType::CrossApply | JoinType::OuterApply => {
                    return Err(RenderError::unsupported(
                        "APPLY",
                        "CROSS/OUTER APPLY not supported in SQLite",
                    ));
                }
            });
            self.sqlite_render_from_item(&join.source, ctx)?;
            if !matches!(join.join_type, JoinType::Cross) {
                if let Some(condition) = &join.condition {
                    match condition {
                        JoinCondition::On(cond) => {
                            ctx.keyword("ON");
                            self.render_condition(cond, ctx)?;
                        }
                        JoinCondition::Using(cols) => {
                            ctx.keyword("USING").paren_open();
                            self.sqlite_comma_idents(cols, ctx);
                            ctx.paren_close();
                        }
                    }
                }
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
        self.sqlite_order_by_list(order, ctx)
    }
    fn render_limit(&self, limit: &LimitDef, ctx: &mut RenderCtx) -> RenderResult<()> {
        match &limit.kind {
            LimitKind::Limit(n) => {
                ctx.keyword("LIMIT");
                if ctx.parameterize() {
                    ctx.param(Value::BigInt(*n as i64));
                } else {
                    ctx.space().write(&n.to_string());
                }
            }
            LimitKind::FetchFirst {
                count, with_ties, ..
            } => {
                if *with_ties {
                    return Err(RenderError::unsupported(
                        "FETCH FIRST WITH TIES",
                        "not supported in SQLite",
                    ));
                }
                // Convert FETCH FIRST to LIMIT
                ctx.keyword("LIMIT");
                if ctx.parameterize() {
                    ctx.param(Value::BigInt(*count as i64));
                } else {
                    ctx.space().write(&count.to_string());
                }
            }
            LimitKind::Top {
                count, with_ties, ..
            } => {
                if *with_ties {
                    return Err(RenderError::unsupported(
                        "TOP WITH TIES",
                        "not supported in SQLite",
                    ));
                }
                // Convert TOP to LIMIT
                ctx.keyword("LIMIT");
                if ctx.parameterize() {
                    ctx.param(Value::BigInt(*count as i64));
                } else {
                    ctx.space().write(&count.to_string());
                }
            }
        }
        if let Some(offset) = limit.offset {
            ctx.keyword("OFFSET");
            if ctx.parameterize() {
                ctx.param(Value::BigInt(offset as i64));
            } else {
                ctx.space().write(&offset.to_string());
            }
        }
        Ok(())
    }
    fn render_ctes(&self, ctes: &[CteDef], ctx: &mut RenderCtx) -> RenderResult<()> {
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
                self.sqlite_comma_idents(col_names, ctx);
                ctx.paren_close();
            }
            // SQLite ignores MATERIALIZED hints
            ctx.keyword("AS").paren_open();
            self.render_query(&cte.query, ctx)?;
            ctx.paren_close();
        }
        Ok(())
    }
    fn render_lock(&self, _lock: &SelectLockDef, _ctx: &mut RenderCtx) -> RenderResult<()> {
        Err(RenderError::unsupported(
            "FOR UPDATE/SHARE",
            "row locking not supported in SQLite",
        ))
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
            self.sqlite_render_ctes(ctes, ctx)?;
        }

        // INSERT OR REPLACE / OR IGNORE / etc.
        if let Some(cr) = &stmt.conflict_resolution {
            ctx.keyword("INSERT OR");
            ctx.keyword(match cr {
                ConflictResolution::Rollback => "ROLLBACK",
                ConflictResolution::Abort => "ABORT",
                ConflictResolution::Fail => "FAIL",
                ConflictResolution::Ignore => "IGNORE",
                ConflictResolution::Replace => "REPLACE",
            });
            ctx.keyword("INTO");
        } else {
            ctx.keyword("INSERT INTO");
        }

        self.sqlite_schema_ref(&stmt.table, ctx);

        // Alias
        if let Some(alias) = &stmt.table.alias {
            ctx.keyword("AS").ident(alias);
        }

        // Column list
        if let Some(cols) = &stmt.columns {
            ctx.paren_open();
            self.sqlite_comma_idents(cols, ctx);
            ctx.paren_close();
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
            self.sqlite_render_ctes(ctes, ctx)?;
        }

        // UPDATE OR REPLACE / OR IGNORE / etc.
        if let Some(cr) = &stmt.conflict_resolution {
            ctx.keyword("UPDATE OR");
            ctx.keyword(match cr {
                ConflictResolution::Rollback => "ROLLBACK",
                ConflictResolution::Abort => "ABORT",
                ConflictResolution::Fail => "FAIL",
                ConflictResolution::Ignore => "IGNORE",
                ConflictResolution::Replace => "REPLACE",
            });
        } else {
            ctx.keyword("UPDATE");
        }

        self.sqlite_schema_ref(&stmt.table, ctx);

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

        // FROM (SQLite 3.33+)
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

        // ORDER BY
        if let Some(order_by) = &stmt.order_by {
            ctx.keyword("ORDER BY");
            self.sqlite_order_by_list(order_by, ctx)?;
        }

        // LIMIT / OFFSET
        if let Some(limit) = stmt.limit {
            ctx.keyword("LIMIT").keyword(&limit.to_string());
            if let Some(offset) = stmt.offset {
                ctx.keyword("OFFSET").keyword(&offset.to_string());
            }
        }

        Ok(())
    }

    fn render_delete(&self, stmt: &DeleteStmt, ctx: &mut RenderCtx) -> RenderResult<()> {
        // CTEs
        if let Some(ctes) = &stmt.ctes {
            self.sqlite_render_ctes(ctes, ctx)?;
        }

        ctx.keyword("DELETE FROM");

        self.sqlite_schema_ref(&stmt.table, ctx);

        // Alias
        if let Some(alias) = &stmt.table.alias {
            ctx.keyword("AS").ident(alias);
        }

        // SQLite doesn't support USING — ignore
        // (SQLite has no JOIN syntax in DELETE; use subqueries in WHERE)

        // WHERE
        if let Some(cond) = &stmt.where_clause {
            ctx.keyword("WHERE");
            self.render_condition(cond, ctx)?;
        }

        // RETURNING
        if let Some(returning) = &stmt.returning {
            self.render_returning(returning, ctx)?;
        }

        // ORDER BY
        if let Some(order_by) = &stmt.order_by {
            ctx.keyword("ORDER BY");
            self.sqlite_order_by_list(order_by, ctx)?;
        }

        // LIMIT / OFFSET
        if let Some(limit) = stmt.limit {
            ctx.keyword("LIMIT").keyword(&limit.to_string());
            if let Some(offset) = stmt.offset {
                ctx.keyword("OFFSET").keyword(&offset.to_string());
            }
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
                    self.sqlite_comma_idents(columns, ctx);
                    ctx.paren_close();
                    if let Some(cond) = where_clause {
                        ctx.keyword("WHERE");
                        self.render_condition(cond, ctx)?;
                    }
                }
                ConflictTarget::Constraint(_) => {
                    return Err(RenderError::unsupported(
                        "OnConstraint",
                        "SQLite does not support ON CONFLICT ON CONSTRAINT. Use column list instead.",
                    ));
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
                    self.sqlite_field_ref(field, ctx);
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
            TransactionStmt::Begin(s) => {
                ctx.keyword("BEGIN");
                if let Some(lock_type) = &s.lock_type {
                    ctx.keyword(match lock_type {
                        SqliteLockType::Deferred => "DEFERRED",
                        SqliteLockType::Immediate => "IMMEDIATE",
                        SqliteLockType::Exclusive => "EXCLUSIVE",
                    });
                }
                ctx.keyword("TRANSACTION");
                Ok(())
            }
            TransactionStmt::Commit(_) => {
                ctx.keyword("COMMIT");
                Ok(())
            }
            TransactionStmt::Rollback(s) => {
                ctx.keyword("ROLLBACK");
                if let Some(sp) = &s.to_savepoint {
                    ctx.keyword("TO").keyword("SAVEPOINT").ident(sp);
                }
                Ok(())
            }
            TransactionStmt::Savepoint(s) => {
                ctx.keyword("SAVEPOINT").ident(&s.name);
                Ok(())
            }
            TransactionStmt::ReleaseSavepoint(s) => {
                ctx.keyword("RELEASE").keyword("SAVEPOINT").ident(&s.name);
                Ok(())
            }
            TransactionStmt::SetTransaction(_) => Err(RenderError::unsupported(
                "SET TRANSACTION",
                "not supported in SQLite",
            )),
            TransactionStmt::LockTable(_) => Err(RenderError::unsupported(
                "LOCK TABLE",
                "not supported in SQLite (use BEGIN EXCLUSIVE)",
            )),
            TransactionStmt::PrepareTransaction(_) => Err(RenderError::unsupported(
                "PREPARE TRANSACTION",
                "not supported in SQLite",
            )),
            TransactionStmt::CommitPrepared(_) => Err(RenderError::unsupported(
                "COMMIT PREPARED",
                "not supported in SQLite",
            )),
            TransactionStmt::RollbackPrepared(_) => Err(RenderError::unsupported(
                "ROLLBACK PREPARED",
                "not supported in SQLite",
            )),
            TransactionStmt::Custom(_) => Err(RenderError::unsupported(
                "Custom TCL",
                "not supported by SqliteRenderer",
            )),
        }
    }
}

// ==========================================================================
// SQLite-specific helpers
// ==========================================================================

impl SqliteRenderer {
    fn sqlite_schema_ref(
        &self,
        schema_ref: &qcraft_core::ast::common::SchemaRef,
        ctx: &mut RenderCtx,
    ) {
        if let Some(ns) = &schema_ref.namespace {
            ctx.ident(ns).operator(".");
        }
        ctx.ident(&schema_ref.name);
    }

    /// Render an expression with all FieldRef table names stripped.
    /// Used for SQLite generated column expressions which only allow
    /// unqualified column references.
    fn render_expr_unqualified(&self, expr: &Expr, ctx: &mut RenderCtx) -> RenderResult<()> {
        match expr {
            Expr::Field(field_ref) => {
                ctx.ident(&field_ref.field.name);
                let mut child = &field_ref.field.child;
                while let Some(c) = child {
                    ctx.operator("->'")
                        .write(&c.name.replace('\'', "''"))
                        .write("'");
                    child = &c.child;
                }
                Ok(())
            }
            // For any other expr, delegate to normal render_expr
            other => self.render_expr(other, ctx),
        }
    }

    fn sqlite_field_ref(&self, field_ref: &FieldRef, ctx: &mut RenderCtx) {
        if let Some(ns) = &field_ref.namespace {
            ctx.ident(ns).operator(".");
        }
        if !field_ref.table_name.is_empty() {
            ctx.ident(&field_ref.table_name).operator(".");
        }
        ctx.ident(&field_ref.field.name);
        let mut child = &field_ref.field.child;
        while let Some(c) = child {
            ctx.operator("->'")
                .write(&c.name.replace('\'', "''"))
                .write("'");
            child = &c.child;
        }
    }

    fn sqlite_comma_idents(&self, names: &[String], ctx: &mut RenderCtx) {
        for (i, name) in names.iter().enumerate() {
            if i > 0 {
                ctx.comma();
            }
            ctx.ident(name);
        }
    }

    fn sqlite_value(&self, val: &Value, ctx: &mut RenderCtx) -> RenderResult<()> {
        if matches!(val, Value::Null) && !ctx.parameterize() {
            ctx.keyword("NULL");
            return Ok(());
        }

        // Array in inline literal mode → JSON string (no native array type).
        if let Value::Array(items) = val {
            if !ctx.parameterize() {
                let json = Self::array_to_json(items);
                ctx.string_literal(&json);
                return Ok(());
            }
        }

        // Unsupported types always error, regardless of parameterize mode.
        if let Value::Vector(_) = val {
            return Err(RenderError::unsupported(
                "VectorValue",
                "SQLite does not support vector type.",
            ));
        }

        // In parameterized mode, send values as bind parameters.
        if ctx.parameterize() {
            ctx.param(val.clone());
            return Ok(());
        }

        // Inline literal mode (DDL defaults, etc.)
        self.sqlite_value_literal(val, ctx)
    }

    fn sqlite_value_literal(&self, val: &Value, ctx: &mut RenderCtx) -> RenderResult<()> {
        match val {
            Value::Null => {
                ctx.keyword("NULL");
            }
            Value::Bool(b) => {
                ctx.keyword(if *b { "1" } else { "0" });
            }
            Value::Int(n) | Value::BigInt(n) => {
                ctx.keyword(&n.to_string());
            }
            Value::Float(f) => {
                ctx.keyword(&f.to_string());
            }
            Value::Str(s) => {
                ctx.string_literal(s);
            }
            Value::Bytes(b) => {
                ctx.write("X'");
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
            Value::Json(s) | Value::Jsonb(s) => {
                ctx.string_literal(s);
            }
            Value::IpNetwork(s) => {
                ctx.string_literal(s);
            }
            Value::TimeDelta {
                years,
                months,
                days,
                seconds,
                microseconds,
            } => {
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
            _ => {
                // Vector — already caught in sqlite_value
                unreachable!()
            }
        }
        Ok(())
    }

    fn array_to_json(items: &[Value]) -> String {
        let mut s = String::from("[");
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            Self::value_to_json(item, &mut s);
        }
        s.push(']');
        s
    }

    fn value_to_json(val: &Value, s: &mut String) {
        match val {
            Value::Null => s.push_str("null"),
            Value::Bool(b) => s.push_str(if *b { "true" } else { "false" }),
            Value::Int(n) | Value::BigInt(n) => s.push_str(&n.to_string()),
            Value::Float(f) => s.push_str(&f.to_string()),
            Value::Str(v) => {
                s.push('"');
                for ch in v.chars() {
                    match ch {
                        '"' => s.push_str("\\\""),
                        '\\' => s.push_str("\\\\"),
                        '\n' => s.push_str("\\n"),
                        '\r' => s.push_str("\\r"),
                        '\t' => s.push_str("\\t"),
                        c => s.push(c),
                    }
                }
                s.push('"');
            }
            Value::Array(items) => {
                s.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    Self::value_to_json(item, s);
                }
                s.push(']');
            }
            // Date, DateTime, Time, Uuid, Json, Jsonb, etc. → string
            Value::Date(v)
            | Value::DateTime(v)
            | Value::Time(v)
            | Value::Uuid(v)
            | Value::Decimal(v)
            | Value::IpNetwork(v) => {
                s.push('"');
                s.push_str(v);
                s.push('"');
            }
            Value::Json(v) | Value::Jsonb(v) => {
                // Already JSON — embed directly
                s.push_str(v);
            }
            _ => s.push_str("null"),
        }
    }

    fn sqlite_referential_action(
        &self,
        action: &ReferentialAction,
        ctx: &mut RenderCtx,
    ) -> RenderResult<()> {
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
                if cols.is_some() {
                    return Err(RenderError::unsupported(
                        "SetNullColumns",
                        "SQLite does not support SET NULL with column list.",
                    ));
                }
            }
            ReferentialAction::SetDefault(cols) => {
                ctx.keyword("SET DEFAULT");
                if cols.is_some() {
                    return Err(RenderError::unsupported(
                        "SetDefaultColumns",
                        "SQLite does not support SET DEFAULT with column list.",
                    ));
                }
            }
        }
        Ok(())
    }

    fn sqlite_deferrable(&self, def: &DeferrableConstraint, ctx: &mut RenderCtx) {
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

    fn sqlite_render_ctes(&self, ctes: &[CteDef], ctx: &mut RenderCtx) -> RenderResult<()> {
        self.render_ctes(ctes, ctx)
    }

    fn sqlite_render_from_item(&self, item: &FromItem, ctx: &mut RenderCtx) -> RenderResult<()> {
        // SQLite ignores ONLY (PG-specific)
        self.render_from(&item.source, ctx)?;
        // Index hints
        if let Some(hint) = &item.index_hint {
            match hint {
                SqliteIndexHint::IndexedBy(name) => {
                    ctx.keyword("INDEXED BY").ident(name);
                }
                SqliteIndexHint::NotIndexed => {
                    ctx.keyword("NOT INDEXED");
                }
            }
        }
        // TABLESAMPLE
        if item.sample.is_some() {
            return Err(RenderError::unsupported(
                "TABLESAMPLE",
                "not supported in SQLite",
            ));
        }
        Ok(())
    }

    fn sqlite_render_group_by(
        &self,
        items: &[GroupByItem],
        ctx: &mut RenderCtx,
    ) -> RenderResult<()> {
        ctx.keyword("GROUP BY");
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                ctx.comma();
            }
            match item {
                GroupByItem::Expr(expr) => {
                    self.render_expr(expr, ctx)?;
                }
                GroupByItem::Rollup(_) => {
                    return Err(RenderError::unsupported(
                        "ROLLUP",
                        "not supported in SQLite",
                    ));
                }
                GroupByItem::Cube(_) => {
                    return Err(RenderError::unsupported("CUBE", "not supported in SQLite"));
                }
                GroupByItem::GroupingSets(_) => {
                    return Err(RenderError::unsupported(
                        "GROUPING SETS",
                        "not supported in SQLite",
                    ));
                }
            }
        }
        Ok(())
    }

    fn sqlite_render_window_clause(
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
                self.sqlite_order_by_list(order_by, ctx)?;
            }
            if let Some(frame) = &win.frame {
                self.sqlite_window_frame(frame, ctx);
            }
            ctx.paren_close();
        }
        Ok(())
    }

    fn sqlite_window_frame(&self, frame: &WindowFrameDef, ctx: &mut RenderCtx) {
        ctx.keyword(match frame.frame_type {
            WindowFrameType::Rows => "ROWS",
            WindowFrameType::Range => "RANGE",
            WindowFrameType::Groups => "GROUPS",
        });
        if let Some(end) = &frame.end {
            ctx.keyword("BETWEEN");
            self.sqlite_frame_bound(&frame.start, ctx);
            ctx.keyword("AND");
            self.sqlite_frame_bound(end, ctx);
        } else {
            self.sqlite_frame_bound(&frame.start, ctx);
        }
    }

    fn sqlite_frame_bound(&self, bound: &WindowFrameBound, ctx: &mut RenderCtx) {
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

    fn sqlite_render_set_op(&self, set_op: &SetOpDef, ctx: &mut RenderCtx) -> RenderResult<()> {
        self.render_query(&set_op.left, ctx)?;
        ctx.keyword(match set_op.operation {
            SetOperationType::Union => "UNION",
            SetOperationType::UnionAll => "UNION ALL",
            SetOperationType::Intersect => "INTERSECT",
            SetOperationType::Except => "EXCEPT",
            SetOperationType::IntersectAll => {
                return Err(RenderError::unsupported(
                    "INTERSECT ALL",
                    "not supported in SQLite",
                ));
            }
            SetOperationType::ExceptAll => {
                return Err(RenderError::unsupported(
                    "EXCEPT ALL",
                    "not supported in SQLite",
                ));
            }
        });
        self.render_query(&set_op.right, ctx)
    }

    fn sqlite_create_table(
        &self,
        schema: &SchemaDef,
        if_not_exists: bool,
        temporary: bool,
        without_rowid: bool,
        strict: bool,
        ctx: &mut RenderCtx,
    ) -> RenderResult<()> {
        ctx.keyword("CREATE");
        if temporary {
            ctx.keyword("TEMP");
        }
        ctx.keyword("TABLE");
        if if_not_exists {
            ctx.keyword("IF NOT EXISTS");
        }
        if let Some(ns) = &schema.namespace {
            ctx.ident(ns).operator(".");
        }
        ctx.ident(&schema.name);

        // Collect PK column names for identity detection
        let pk_columns: Vec<&str> = schema
            .constraints
            .as_ref()
            .and_then(|cs| {
                cs.iter().find_map(|c| {
                    if let ConstraintDef::PrimaryKey { columns, .. } = c {
                        Some(columns.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_default();

        // Find identity column that should be rendered as PRIMARY KEY AUTOINCREMENT
        let identity_pk_col = schema.columns.iter().find_map(|col| {
            if col.identity.is_some() {
                if pk_columns.contains(&col.name.as_str()) {
                    Some(col.name.as_str())
                } else {
                    None
                }
            } else {
                None
            }
        });

        // Validate: identity without PK is an error in SQLite
        for col in &schema.columns {
            if col.identity.is_some() && !pk_columns.contains(&col.name.as_str()) {
                return Err(RenderError::unsupported(
                    "IdentityColumn",
                    "SQLite requires identity columns to be PRIMARY KEY. Add a PrimaryKey constraint for this column.",
                ));
            }
        }

        ctx.paren_open();
        let mut first = true;
        for col in &schema.columns {
            if !first {
                ctx.comma();
            }
            first = false;
            self.render_column_def(col, ctx)?;
            // Inline PRIMARY KEY AUTOINCREMENT on the identity column
            if identity_pk_col == Some(col.name.as_str()) {
                ctx.keyword("PRIMARY KEY AUTOINCREMENT");
            }
        }
        if let Some(constraints) = &schema.constraints {
            for constraint in constraints {
                // Skip single-column PK if it was inlined with AUTOINCREMENT
                if let ConstraintDef::PrimaryKey { columns, .. } = constraint {
                    if columns.len() == 1 && identity_pk_col == Some(columns[0].as_str()) {
                        continue;
                    }
                }
                if !first {
                    ctx.comma();
                }
                first = false;
                self.render_constraint(constraint, ctx)?;
            }
        }
        ctx.paren_close();

        // SQLite table modifiers
        let mut modifiers = Vec::new();
        if without_rowid {
            modifiers.push("WITHOUT ROWID");
        }
        if strict {
            modifiers.push("STRICT");
        }
        if !modifiers.is_empty() {
            for (i, m) in modifiers.iter().enumerate() {
                if i > 0 {
                    ctx.comma();
                }
                ctx.keyword(m);
            }
        }

        Ok(())
    }

    fn sqlite_create_index(
        &self,
        schema_ref: &qcraft_core::ast::common::SchemaRef,
        index: &IndexDef,
        if_not_exists: bool,
        ctx: &mut RenderCtx,
    ) -> RenderResult<()> {
        ctx.keyword("CREATE");
        if index.unique {
            ctx.keyword("UNIQUE");
        }
        ctx.keyword("INDEX");
        if if_not_exists {
            ctx.keyword("IF NOT EXISTS");
        }
        ctx.ident(&index.name).keyword("ON");
        self.sqlite_schema_ref(schema_ref, ctx);

        // SQLite doesn't support USING method — Ignore index_type

        ctx.paren_open();
        self.sqlite_index_columns(&index.columns, ctx)?;
        ctx.paren_close();

        // SQLite doesn't support INCLUDE, NULLS DISTINCT, WITH params, TABLESPACE — Ignore

        if let Some(condition) = &index.condition {
            ctx.keyword("WHERE");
            self.render_condition(condition, ctx)?;
        }

        Ok(())
    }

    fn sqlite_index_columns(
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
            // SQLite doesn't support operator classes — Ignore opclass
            if let Some(dir) = col.direction {
                ctx.keyword(match dir {
                    OrderDir::Asc => "ASC",
                    OrderDir::Desc => "DESC",
                });
            }
            // SQLite doesn't support NULLS FIRST/LAST — Ignore
        }
        Ok(())
    }

    fn sqlite_order_by_list(
        &self,
        order_by: &[OrderByDef],
        ctx: &mut RenderCtx,
    ) -> RenderResult<()> {
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
}
