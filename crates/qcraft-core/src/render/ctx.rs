use crate::ast::value::Value;

/// Style of parameter placeholders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamStyle {
    /// PostgreSQL / asyncpg: `$1`, `$2`, `$3`
    Dollar,
    /// SQLite / MySQL: `?`
    QMark,
    /// psycopg / DB-API 2.0: `%s`
    Percent,
}

/// Rendering context: accumulates SQL string and parameters.
///
/// Provides a semantic, chainable API for building SQL output.
pub struct RenderCtx {
    sql: String,
    params: Vec<Value>,
    param_style: ParamStyle,
    param_index: usize,
    parameterize: bool,
}

impl RenderCtx {
    pub fn new(param_style: ParamStyle) -> Self {
        Self {
            sql: String::with_capacity(256),
            params: Vec::new(),
            param_style,
            param_index: 0,
            parameterize: false,
        }
    }

    pub fn with_parameterize(mut self, parameterize: bool) -> Self {
        self.parameterize = parameterize;
        self
    }

    /// Whether values should be rendered as parameter placeholders.
    pub fn parameterize(&self) -> bool {
        self.parameterize
    }

    // ── Result ──

    /// Consume the context and return the SQL string and parameters.
    pub fn finish(self) -> (String, Vec<Value>) {
        (self.sql, self.params)
    }

    /// Get the current SQL string (for inspection).
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Get the current parameters (for inspection).
    pub fn params(&self) -> &[Value] {
        &self.params
    }

    // ── Semantic writing methods (chainable) ──

    /// Write a SQL keyword. Adds a leading space if the buffer is non-empty
    /// and doesn't end with `(` or whitespace.
    pub fn keyword(&mut self, kw: &str) -> &mut Self {
        self.space_if_needed();
        self.sql.push_str(kw);
        self
    }

    /// Write a quoted identifier: `"name"`.
    pub fn ident(&mut self, name: &str) -> &mut Self {
        self.space_if_needed();
        self.sql.push('"');
        // Escape double quotes within identifier
        if name.contains('"') {
            self.sql.push_str(&name.replace('"', "\"\""));
        } else {
            self.sql.push_str(name);
        }
        self.sql.push('"');
        self
    }

    /// Write a parameterized value. Appends the placeholder ($1, ?, etc.)
    /// and stores the value.
    pub fn param(&mut self, val: Value) -> &mut Self {
        self.space_if_needed();
        self.param_index += 1;
        match self.param_style {
            ParamStyle::Dollar => {
                self.sql.push('$');
                self.sql.push_str(&self.param_index.to_string());
            }
            ParamStyle::QMark => {
                self.sql.push('?');
            }
            ParamStyle::Percent => {
                self.sql.push_str("%s");
            }
        }
        self.params.push(val);
        self
    }

    /// Write a string literal with proper escaping: `'value'`.
    pub fn string_literal(&mut self, s: &str) -> &mut Self {
        self.space_if_needed();
        self.sql.push('\'');
        self.sql.push_str(&s.replace('\'', "''"));
        self.sql.push('\'');
        self
    }

    /// Write an operator: `::`, `=`, `>`, `||`, etc.
    pub fn operator(&mut self, op: &str) -> &mut Self {
        self.sql.push_str(op);
        self
    }

    /// Write opening parenthesis `(`, with space if needed.
    pub fn paren_open(&mut self) -> &mut Self {
        self.space_if_needed();
        self.sql.push('(');
        self
    }

    /// Write closing parenthesis `)`.
    pub fn paren_close(&mut self) -> &mut Self {
        self.sql.push(')');
        self
    }

    /// Write a comma separator `, `.
    pub fn comma(&mut self) -> &mut Self {
        self.sql.push_str(", ");
        self
    }

    /// Write an explicit space.
    pub fn space(&mut self) -> &mut Self {
        self.sql.push(' ');
        self
    }

    /// Write arbitrary text (escape hatch, use sparingly).
    pub fn write(&mut self, s: &str) -> &mut Self {
        self.sql.push_str(s);
        self
    }

    // ── Internal helpers ──

    /// Add a space if the buffer doesn't end with whitespace or `(`.
    fn space_if_needed(&mut self) {
        if let Some(last) = self.sql.as_bytes().last() {
            if !matches!(last, b' ' | b'\n' | b'\t' | b'(' | b'.') {
                self.sql.push(' ');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_auto_spacing() {
        let mut ctx = RenderCtx::new(ParamStyle::Dollar);
        ctx.keyword("SELECT").keyword("*").keyword("FROM");
        assert_eq!(ctx.sql(), "SELECT * FROM");
    }

    #[test]
    fn ident_quoted() {
        let mut ctx = RenderCtx::new(ParamStyle::Dollar);
        ctx.keyword("SELECT").ident("user name");
        assert_eq!(ctx.sql(), "SELECT \"user name\"");
    }

    #[test]
    fn ident_escapes_quotes() {
        let mut ctx = RenderCtx::new(ParamStyle::Dollar);
        ctx.ident("has\"quote");
        assert_eq!(ctx.sql(), "\"has\"\"quote\"");
    }

    #[test]
    fn param_dollar_style() {
        let mut ctx = RenderCtx::new(ParamStyle::Dollar);
        ctx.keyword("WHERE")
            .ident("age")
            .operator(" > ")
            .param(Value::Int(18));
        let (sql, params) = ctx.finish();
        assert_eq!(sql, "WHERE \"age\" > $1");
        assert_eq!(params, vec![Value::Int(18)]);
    }

    #[test]
    fn param_qmark_style() {
        let mut ctx = RenderCtx::new(ParamStyle::QMark);
        ctx.keyword("WHERE")
            .ident("age")
            .operator(" > ")
            .param(Value::Int(18));
        let (sql, params) = ctx.finish();
        assert_eq!(sql, "WHERE \"age\" > ?");
        assert_eq!(params, vec![Value::Int(18)]);
    }

    #[test]
    fn multiple_params() {
        let mut ctx = RenderCtx::new(ParamStyle::Dollar);
        ctx.param(Value::Int(1))
            .comma()
            .param(Value::Str("hello".into()));
        let (sql, params) = ctx.finish();
        assert_eq!(sql, "$1, $2");
        assert_eq!(params, vec![Value::Int(1), Value::Str("hello".into())]);
    }

    #[test]
    fn paren_no_extra_space() {
        let mut ctx = RenderCtx::new(ParamStyle::Dollar);
        ctx.keyword("CAST")
            .paren_open()
            .ident("x")
            .keyword("AS")
            .keyword("TEXT")
            .paren_close();
        assert_eq!(ctx.sql(), "CAST (\"x\" AS TEXT)");
    }

    #[test]
    fn string_literal_escaping() {
        let mut ctx = RenderCtx::new(ParamStyle::Dollar);
        ctx.string_literal("it's a test");
        assert_eq!(ctx.sql(), "'it''s a test'");
    }

    #[test]
    fn chaining() {
        let mut ctx = RenderCtx::new(ParamStyle::Dollar);
        ctx.keyword("CREATE")
            .keyword("TABLE")
            .keyword("IF NOT EXISTS")
            .ident("users")
            .paren_open();
        ctx.ident("id").keyword("BIGINT").keyword("NOT NULL");
        ctx.paren_close();
        assert_eq!(
            ctx.sql(),
            r#"CREATE TABLE IF NOT EXISTS "users" ("id" BIGINT NOT NULL)"#
        );
    }
}
