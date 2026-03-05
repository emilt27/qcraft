use rquery_core::ast::common::{FieldRef, OrderByDef, OrderDir};
use rquery_core::ast::conditions::{CompareOp, ConditionNode, Conditions, Connector};
use rquery_core::ast::ddl::{
    ColumnDef, ConstraintDef, DeferrableConstraint, FieldType,
    IndexColumnDef, IndexDef, IndexExpr,
    ReferentialAction, SchemaDef, SchemaMutationStmt,
};
use rquery_core::ast::dml::{
    DeleteStmt, InsertStmt, MutationStmt, OnConflictDef, UpdateStmt,
};
use rquery_core::ast::expr::{
    AggregationDef, BinaryOp, CaseDef, Expr, UnaryOp, WindowDef,
};
use rquery_core::ast::query::{
    CteDef, JoinDef, LimitDef, QueryStmt, SelectColumn, SelectLockDef, TableSource,
};
use rquery_core::ast::value::Value;
use rquery_core::error::{RenderError, RenderResult};
use rquery_core::render::ctx::{ParamStyle, RenderCtx};
use rquery_core::render::renderer::Renderer;

pub struct SqliteRenderer;

impl SqliteRenderer {
    pub fn new() -> Self {
        Self
    }

    pub fn render_schema_stmt(&self, stmt: &SchemaMutationStmt) -> RenderResult<(String, Vec<Value>)> {
        let mut ctx = RenderCtx::new(ParamStyle::QMark);
        self.render_schema_mutation(stmt, &mut ctx)?;
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
                unlogged: _, // SQLite doesn't support UNLOGGED — Ignore (logged = safer)
                tablespace: _, // SQLite doesn't support TABLESPACE — Ignore
            } => self.sqlite_create_table(schema, *if_not_exists, *temporary, ctx),

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
                cascade: _, // SQLite doesn't support CASCADE
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
                position: _, // SQLite doesn't support FIRST/AFTER
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
                cascade: _, // SQLite doesn't support CASCADE
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
                ctx.keyword("RENAME COLUMN").ident(old_name).keyword("TO").ident(new_name);
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
                cascade: _, // Ignore
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

        // SQLite doesn't support IDENTITY — use INTEGER PRIMARY KEY AUTOINCREMENT instead
        if col.identity.is_some() {
            return Err(RenderError::unsupported(
                "IdentityColumn",
                "SQLite does not support GENERATED AS IDENTITY. Use INTEGER PRIMARY KEY AUTOINCREMENT.",
            ));
        }

        if let Some(generated) = &col.generated {
            ctx.keyword("GENERATED ALWAYS AS").space().paren_open();
            self.render_expr(&generated.expr, ctx)?;
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
                ctx.keyword(name).paren_open();
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
                include: _, // Ignore
                nulls_distinct: _, // Ignore
                condition: _, // Ignore
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
                enforced: _, // Ignore
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
                ctx.keyword("CAST").paren_open();
                self.render_expr(inner, ctx)?;
                ctx.keyword("AS").keyword(to_type);
                ctx.paren_close();
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

            Expr::ArraySubQuery(_) => Err(RenderError::unsupported(
                "ArraySubQuery",
                "SQLite does not support ARRAY subqueries.",
            )),

            Expr::Raw { sql, params } => {
                ctx.keyword(sql);
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
            self.sqlite_order_by_list(order_by, ctx);
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
            self.sqlite_order_by_list(order_by, ctx);
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
            CompareOp::Regex => ctx.keyword("REGEXP"),
            // SQLite doesn't natively support these — Error
            CompareOp::ILike | CompareOp::Similar | CompareOp::IRegex => {
                return Err(RenderError::unsupported(
                    "CompareOp",
                    "SQLite does not support ILIKE, SIMILAR TO, or case-insensitive regex.",
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
            | CompareOp::RangeOverlap => {
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

    fn render_query(&self, _stmt: &QueryStmt, _ctx: &mut RenderCtx) -> RenderResult<()> {
        todo!("SQLite query rendering not yet implemented")
    }
    fn render_select_columns(&self, _cols: &[SelectColumn], _ctx: &mut RenderCtx) -> RenderResult<()> { todo!() }
    fn render_from(&self, _source: &TableSource, _ctx: &mut RenderCtx) -> RenderResult<()> { todo!() }
    fn render_joins(&self, _joins: &[JoinDef], _ctx: &mut RenderCtx) -> RenderResult<()> { todo!() }
    fn render_where(&self, _cond: &Conditions, _ctx: &mut RenderCtx) -> RenderResult<()> { todo!() }
    fn render_order_by(&self, _order: &[OrderByDef], _ctx: &mut RenderCtx) -> RenderResult<()> { todo!() }
    fn render_limit(&self, _limit: &LimitDef, _ctx: &mut RenderCtx) -> RenderResult<()> { todo!() }
    fn render_ctes(&self, _ctes: &[CteDef], _ctx: &mut RenderCtx) -> RenderResult<()> { todo!() }
    fn render_lock(&self, _lock: &SelectLockDef, _ctx: &mut RenderCtx) -> RenderResult<()> { todo!() }

    // ── DML (stub) ───────────────────────────────────────────────────────

    fn render_mutation(&self, _stmt: &MutationStmt, _ctx: &mut RenderCtx) -> RenderResult<()> { todo!() }
    fn render_insert(&self, _stmt: &InsertStmt, _ctx: &mut RenderCtx) -> RenderResult<()> { todo!() }
    fn render_update(&self, _stmt: &UpdateStmt, _ctx: &mut RenderCtx) -> RenderResult<()> { todo!() }
    fn render_delete(&self, _stmt: &DeleteStmt, _ctx: &mut RenderCtx) -> RenderResult<()> { todo!() }
    fn render_on_conflict(&self, _oc: &OnConflictDef, _ctx: &mut RenderCtx) -> RenderResult<()> { todo!() }
    fn render_returning(&self, _fields: &[FieldRef], _ctx: &mut RenderCtx) -> RenderResult<()> { todo!() }
}

// ==========================================================================
// SQLite-specific helpers
// ==========================================================================

impl SqliteRenderer {
    fn sqlite_schema_ref(&self, schema_ref: &rquery_core::ast::common::SchemaRef, ctx: &mut RenderCtx) {
        if let Some(ns) = &schema_ref.namespace {
            ctx.ident(ns).operator(".");
        }
        ctx.ident(&schema_ref.name);
    }

    fn sqlite_field_ref(&self, field_ref: &FieldRef, ctx: &mut RenderCtx) {
        ctx.ident(&field_ref.table_name).operator(".").ident(&field_ref.field.name);
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
        match val {
            Value::Null => { ctx.keyword("NULL"); }
            Value::Bool(b) => { ctx.keyword(if *b { "1" } else { "0" }); }
            Value::Int(n) => { ctx.write(&n.to_string()); }
            Value::Float(f) => { ctx.write(&f.to_string()); }
            Value::Str(s) => { ctx.string_literal(s); }
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
            Value::Decimal(s) => { ctx.write(s); }
            Value::Uuid(s) => { ctx.string_literal(s); }
            Value::List(_) => {
                return Err(RenderError::unsupported(
                    "ListValue",
                    "SQLite does not support array/list literals.",
                ));
            }
            Value::TimeDelta { .. } => {
                return Err(RenderError::unsupported(
                    "TimeDeltaValue",
                    "SQLite does not support INTERVAL type. Use string expressions with datetime functions.",
                ));
            }
        }
        Ok(())
    }

    fn sqlite_referential_action(&self, action: &ReferentialAction, ctx: &mut RenderCtx) -> RenderResult<()> {
        match action {
            ReferentialAction::NoAction => { ctx.keyword("NO ACTION"); }
            ReferentialAction::Restrict => { ctx.keyword("RESTRICT"); }
            ReferentialAction::Cascade => { ctx.keyword("CASCADE"); }
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

    fn sqlite_create_table(
        &self,
        schema: &SchemaDef,
        if_not_exists: bool,
        temporary: bool,
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

        Ok(())
    }

    fn sqlite_create_index(
        &self,
        schema_ref: &rquery_core::ast::common::SchemaRef,
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

    fn sqlite_index_columns(&self, columns: &[IndexColumnDef], ctx: &mut RenderCtx) -> RenderResult<()> {
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

    fn sqlite_order_by_list(&self, order_by: &[OrderByDef], ctx: &mut RenderCtx) {
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
}
