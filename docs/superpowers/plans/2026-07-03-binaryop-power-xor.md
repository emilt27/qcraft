# BinaryOp Power & BitwiseXor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add first-class `BinaryOp::Power` (`**`) and `BinaryOp::BitwiseXor` (`^`) with per-dialect native rendering — Postgres native infix, SQLite `power()` / composite XOR with numbered-parameter reuse.

**Architecture:** The AST is dialect-agnostic; per-dialect divergence lives in the renderer. PG renders both as native infix operators (`^`, `#`), single-render, no guards. SQLite renders `Power` as `power(l, r)` and `BitwiseXor` as the composite `(((l)|(r)) - ((l)&(r)))`, which duplicates each operand. To let the composite work with unbound params in `executemany`, a new `ParamStyle::QMarkNumbered` (`?1`, `?2`) plus a `RenderCtx::capture` "render-once, splice-text" primitive bind each logical operand once. SQLite guards reject subquery operands (double-execution) always, and unbound `Param` operands in non-numbered mode (positional-binding corruption).

**Tech Stack:** Rust (edition 2024, workspace), `thiserror`, `rusqlite` 0.35 (bundled, dev-dep, for SQLite semantic tests). Cargo workspace: `qcraft-core`, `qcraft-postgres`, `qcraft-sqlite`.

## Global Constraints

- **Spec:** `docs/superpowers/specs/2026-07-03-binaryop-power-xor-design.md` is the source of truth.
- **No AI attribution** anywhere — commits, code, docs, CHANGELOG. (No `Co-Authored-By`, no "Generated with…".)
- **TDD:** every behavioral change starts with a failing test that asserts the *correct* behavior (fails on unmodified code, passes after). Never assert buggy/intermediate behavior.
- **Single test run:** run the relevant test once to confirm; don't re-run repeatedly.
- **Canonical SQLite XOR output (both param modes, identical):** `(((L) | (R)) - ((L) & (R)))` — outer parens + OR-group + AND-group + each operand wrapped. All parens are required for SQLite precedence (`-` binds tighter than `|`/`&`).
- **Version bump:** minor, `2.4.1` → `2.5.0` (additive change), in `[workspace.package]` of root `Cargo.toml`.
- **Run tests with:** `cargo test -p <crate>` (e.g. `cargo test -p qcraft-sqlite`). Build check: `cargo build`.

---

## File Structure

- `crates/qcraft-core/src/render/ctx.rs` — add `ParamStyle::QMarkNumbered`, `RenderCtx::capture`, `RenderCtx::param_style` getter.
- `crates/qcraft-core/src/ast/expr.rs` — add `BinaryOp::Power`, `BinaryOp::BitwiseXor`, `Expr::contains_unbound_param`, `Expr::contains_subquery`.
- `crates/qcraft-postgres/src/lib.rs` — render `Power`→`^`, `BitwiseXor`→`#` (infix).
- `crates/qcraft-sqlite/src/lib.rs` — `SqliteRenderer` param_style plumbing; render `Power`→`power()`, `BitwiseXor`→composite with guards.
- Tests: `crates/qcraft-postgres/tests/dql.rs`, `crates/qcraft-sqlite/tests/dql.rs` (unit / string-assert), `crates/qcraft-sqlite/tests/integration_dql.rs` (rusqlite semantic), `crates/qcraft-core/src/render/ctx.rs` (`#[cfg(test)] mod tests`).
- `Cargo.toml`, `CHANGELOG.md` — release metadata.

---

## Task 1: Core render infra — `QMarkNumbered`, `capture`, `param_style` getter

**Files:**
- Modify: `crates/qcraft-core/src/render/ctx.rs` (enum at 4-12; `placeholder` 97-113; `raw_with_params` match 186-197; add methods to `impl RenderCtx`; import at line 1; tests mod at 227)

**Interfaces:**
- Consumes: nothing (additive).
- Produces:
  - `ParamStyle::QMarkNumbered` (new enum variant).
  - `RenderCtx::param_style(&self) -> ParamStyle` (getter).
  - `RenderCtx::capture<F>(&mut self, f: F) -> RenderResult<String> where F: FnOnce(&mut RenderCtx) -> RenderResult<()>` — runs `f`, returns the trimmed SQL text it appended, removes that text from the buffer, but keeps `param_index`/`params` advanced.

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` block in `crates/qcraft-core/src/render/ctx.rs`:

```rust
#[test]
fn param_qmark_numbered_style() {
    let mut ctx = RenderCtx::new(ParamStyle::QMarkNumbered);
    ctx.param(Value::Int(10)).comma().param(Value::Int(20));
    let (sql, params) = ctx.finish();
    assert_eq!(sql, "?1, ?2");
    assert_eq!(params, vec![Value::Int(10), Value::Int(20)]);
}

#[test]
fn param_style_getter() {
    let ctx = RenderCtx::new(ParamStyle::QMarkNumbered);
    assert_eq!(ctx.param_style(), ParamStyle::QMarkNumbered);
}

#[test]
fn capture_returns_fragment_and_keeps_params_advanced() {
    let mut ctx = RenderCtx::new(ParamStyle::QMarkNumbered);
    ctx.keyword("SELECT");
    let frag = ctx
        .capture(|c| {
            c.param(Value::Int(1)).keyword("+").param(Value::Int(2));
            Ok(())
        })
        .unwrap();
    // Fragment captured and removed from buffer (buffer unchanged past "SELECT"),
    // but params + param_index stayed advanced so the next placeholder is ?3.
    assert_eq!(frag, "?1 + ?2");
    ctx.write(" ").write(&frag).write(" - ").param(Value::Int(9));
    let (sql, params) = ctx.finish();
    assert_eq!(sql, "SELECT ?1 + ?2 - ?3");
    assert_eq!(params, vec![Value::Int(1), Value::Int(2), Value::Int(9)]);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qcraft-core param_qmark_numbered_style param_style_getter capture_returns_fragment`
Expected: FAIL — `no variant named QMarkNumbered` / `no method named param_style` / `no method named capture`.

- [ ] **Step 3: Add the `QMarkNumbered` variant**

In `crates/qcraft-core/src/render/ctx.rs`, extend the enum (lines 4-12):

```rust
pub enum ParamStyle {
    /// PostgreSQL / asyncpg: `$1`, `$2`, `$3`
    Dollar,
    /// SQLite / MySQL: `?`
    QMark,
    /// SQLite numbered: `?1`, `?2` — lets a composite reference a logical
    /// operand multiple times while binding it once (used for XOR / executemany).
    QMarkNumbered,
    /// psycopg / DB-API 2.0: `%s`
    Percent,
}
```

- [ ] **Step 4: Handle `QMarkNumbered` in both placeholder emitters**

In `placeholder()` (the `match self.param_style` at lines 100-111), add an arm after `QMark`:

```rust
ParamStyle::QMark => {
    self.sql.push('?');
}
ParamStyle::QMarkNumbered => {
    self.sql.push('?');
    self.sql.push_str(&self.param_index.to_string());
}
```

In `raw_with_params()` (the `match self.param_style` at lines 186-196), add the same arm after `QMark`:

```rust
ParamStyle::QMark => {
    self.sql.push('?');
}
ParamStyle::QMarkNumbered => {
    self.sql.push('?');
    self.sql.push_str(&self.param_index.to_string());
}
```

(Both emitters already do `self.param_index += 1` before the match, so `?{param_index}` is correct.)

- [ ] **Step 5: Add `param_style` getter and `capture` method**

Add `use crate::error::RenderResult;` to the imports at the top of `ctx.rs` (currently only `use crate::ast::value::Value;`).

Add to `impl RenderCtx` (near `sql()`/`params()` inspectors around line 53-61):

```rust
/// The parameter placeholder style in effect.
pub fn param_style(&self) -> ParamStyle {
    self.param_style
}

/// Render a fragment via `f`, return its SQL text (whitespace-trimmed) and
/// remove that text from the buffer, while leaving `param_index`/`params`
/// advanced by exactly one render of the fragment. The returned text can be
/// spliced in multiple times: in numbered mode its `?N` markers refer to the
/// same already-registered parameters, so an operand binds only once.
pub fn capture<F>(&mut self, f: F) -> RenderResult<String>
where
    F: FnOnce(&mut RenderCtx) -> RenderResult<()>,
{
    let start = self.sql.len();
    f(self)?;
    // Trim is required: space_if_needed may have prepended a leading space
    // depending on the buffer state before the fragment; without trimming the
    // fragment would be context-dependent and exact-string tests would drift.
    let frag = self.sql[start..].trim().to_string();
    self.sql.truncate(start); // start is a valid boundary (a prior length)
    Ok(frag)
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p qcraft-core`
Expected: PASS (all ctx tests, including the 3 new ones).

- [ ] **Step 7: Commit**

```bash
git add crates/qcraft-core/src/render/ctx.rs
git commit -m "feat(core): add QMarkNumbered param style and RenderCtx::capture"
```

---

## Task 2: SqliteRenderer param_style plumbing

**Files:**
- Modify: `crates/qcraft-sqlite/src/lib.rs` (struct at 52; `impl SqliteRenderer` 54-98; 4 ctx constructors at 63, 72, 78, 84)
- Test: `crates/qcraft-sqlite/tests/dql.rs`

**Interfaces:**
- Consumes: `ParamStyle::QMarkNumbered` (Task 1).
- Produces:
  - `SqliteRenderer` now holds `param_style: ParamStyle` (default `QMark`).
  - `SqliteRenderer::with_param_style(self, style: ParamStyle) -> Self` (builder).
  - Existing `SqliteRenderer::new()` / `Default` still yield `QMark` (backward-compatible).

- [ ] **Step 1: Write the failing test**

Add to `crates/qcraft-sqlite/tests/dql.rs` (it already imports `Expr`, `Value`, `SqliteRenderer`, `QueryStmt`, and has `simple_query`-style helpers; mirror the existing `render_with_params` helper). Add a numbered-mode helper + test:

```rust
#[test]
fn numbered_param_style_emits_indexed_placeholders() {
    // SELECT ?1, ?2  with two literal values in QMarkNumbered mode.
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Expr { expr: Expr::Value(Value::Int(10)), alias: None },
            SelectColumn::Expr { expr: Expr::Value(Value::Int(20)), alias: None },
        ],
        ..simple_query()
    };
    let renderer = SqliteRenderer::new().with_param_style(ParamStyle::QMarkNumbered);
    let (sql, params) = renderer.render_query_stmt(&stmt).unwrap();
    assert_eq!(sql, "SELECT ?1, ?2");
    assert_eq!(params, vec![Value::Int(10), Value::Int(20)]);
}
```

Add `use qcraft_core::render::ctx::ParamStyle;` to the test file's imports if not present. Confirm `simple_query()` / `SelectColumn` are already used in this file (they are — mirror existing tests); if `simple_query` doesn't exist here, construct the `QueryStmt` literally as other tests in the file do.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p qcraft-sqlite numbered_param_style_emits_indexed_placeholders`
Expected: FAIL — `no method named with_param_style found for struct SqliteRenderer`.

- [ ] **Step 3: Convert `SqliteRenderer` to a struct with `param_style`**

In `crates/qcraft-sqlite/src/lib.rs`, replace the unit struct (line 52) and its `impl` head (54-57):

```rust
pub struct SqliteRenderer {
    param_style: ParamStyle,
}

impl SqliteRenderer {
    pub fn new() -> Self {
        Self {
            param_style: ParamStyle::QMark,
        }
    }

    /// Set the parameter placeholder style (default `QMark`). Use
    /// `QMarkNumbered` to enable operand reuse for XOR / executemany.
    pub fn with_param_style(mut self, style: ParamStyle) -> Self {
        self.param_style = style;
        self
    }
```

`impl Default for SqliteRenderer` (around line 96) already calls `Self::new()` — leave it unchanged.

- [ ] **Step 4: Thread `param_style` into all 4 ctx constructors**

Replace each `RenderCtx::new(ParamStyle::QMark)` with `RenderCtx::new(self.param_style)`, preserving any `.with_parameterize(true)`:

- Line 63 (`render_schema_stmt`): `RenderCtx::new(self.param_style)`
- Line 72 (`render_transaction_stmt`): `RenderCtx::new(self.param_style)`
- Line 78 (`render_mutation_stmt`): `RenderCtx::new(self.param_style).with_parameterize(true)`
- Line 84 (`render_query_stmt`): `RenderCtx::new(self.param_style).with_parameterize(true)`

- [ ] **Step 5: Run tests to verify pass (incl. no regressions)**

Run: `cargo test -p qcraft-sqlite`
Expected: PASS — the new test passes; all existing tests still pass (default `QMark` unchanged).

- [ ] **Step 6: Commit**

```bash
git add crates/qcraft-sqlite/src/lib.rs crates/qcraft-sqlite/tests/dql.rs
git commit -m "feat(sqlite): configurable ParamStyle via with_param_style"
```

---

## Task 3: Core AST — `Power`/`BitwiseXor` variants + operand predicates

**Files:**
- Modify: `crates/qcraft-core/src/ast/expr.rs` (`BinaryOp` enum 290-305; add `impl Expr` methods)
- Modify (compile-fix, temporary): `crates/qcraft-postgres/src/lib.rs` (Binary arm 894-924), `crates/qcraft-sqlite/src/lib.rs` (Binary arm 469-495)
- Test: `crates/qcraft-core/src/ast/expr.rs` (`#[cfg(test)] mod tests` — add if absent)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `BinaryOp::Power`, `BinaryOp::BitwiseXor`.
  - `Expr::contains_unbound_param(&self) -> bool` — true iff the tree contains an `Expr::Param`. Recurses through Binary/Unary/Func/Cast/Case/Tuple/Aggregate/Window/JsonArray/JsonObject/JsonAgg/StringAgg/JsonPathText/Collate. Does **not** descend into subquery `QueryStmt`s.
  - `Expr::contains_subquery(&self) -> bool` — true iff the tree contains `Exists`/`SubQuery`/`ArraySubQuery` (including nested inside the above expr variants).
  - Both renderers temporarily return `RenderError::unsupported` for the two new ops (replaced in Tasks 4-6). Workspace compiles.

- [ ] **Step 1: Write the failing predicate tests**

Add a test module at the end of `crates/qcraft-core/src/ast/expr.rs`:

```rust
#[cfg(test)]
mod predicate_tests {
    use super::*;
    use crate::ast::query::QueryStmt;

    fn param() -> Expr { Expr::Param { type_hint: None } }

    #[test]
    fn contains_unbound_param_detects_nested_param() {
        let e = Expr::Binary {
            left: Box::new(Expr::Binary {
                left: Box::new(param()),
                op: BinaryOp::Add,
                right: Box::new(Expr::Value(Value::Int(1))),
            }),
            op: BinaryOp::Mul,
            right: Box::new(Expr::Value(Value::Int(2))),
        };
        assert!(e.contains_unbound_param());
    }

    #[test]
    fn contains_unbound_param_false_for_plain_values() {
        let e = Expr::Binary {
            left: Box::new(Expr::field("t", "a")),
            op: BinaryOp::Add,
            right: Box::new(Expr::Value(Value::Int(1))),
        };
        assert!(!e.contains_unbound_param());
    }

    #[test]
    fn contains_subquery_detects_nested_subquery() {
        let sub = Expr::SubQuery(Box::new(QueryStmt::default()));
        let e = Expr::Binary {
            left: Box::new(sub),
            op: BinaryOp::Add,
            right: Box::new(Expr::Value(Value::Int(1))),
        };
        assert!(e.contains_subquery());
    }

    #[test]
    fn contains_subquery_false_for_plain_expr() {
        let e = Expr::Binary {
            left: Box::new(Expr::field("t", "a")),
            op: BinaryOp::BitwiseXor,
            right: Box::new(Expr::Value(Value::Int(1))),
        };
        assert!(!e.contains_subquery());
    }
}
```

Note: if `QueryStmt` has no `Default`, build a minimal one the way other core tests do (check `crates/qcraft-core/src/ast/query.rs`); adjust the `sub` construction accordingly. The assertion (subquery detected) is what matters.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qcraft-core contains_unbound_param contains_subquery`
Expected: FAIL — `no variant named BitwiseXor` and `no method named contains_unbound_param` / `contains_subquery`.

- [ ] **Step 3: Add the two `BinaryOp` variants**

In `crates/qcraft-core/src/ast/expr.rs`, extend the enum (290-305):

```rust
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Power,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
    Concat,

    /// User-defined binary operator (extension point).
    Custom(Box<dyn CustomBinaryOp>),
}
```

- [ ] **Step 4: Add the two predicate methods**

Add to `impl Expr` in the same file:

```rust
/// True if this expression tree contains an unbound `Expr::Param` placeholder.
/// Used to reject double-render forms that would corrupt positional binding.
/// Does not descend into subquery `QueryStmt`s (those are rejected separately).
pub fn contains_unbound_param(&self) -> bool {
    match self {
        Expr::Param { .. } => true,
        Expr::Binary { left, right, .. } => {
            left.contains_unbound_param() || right.contains_unbound_param()
        }
        Expr::Unary { expr, .. }
        | Expr::Cast { expr, .. }
        | Expr::Collate { expr, .. }
        | Expr::JsonPathText { expr, .. } => expr.contains_unbound_param(),
        Expr::Func { args, .. } | Expr::Tuple(args) | Expr::JsonArray(args) => {
            args.iter().any(|a| a.contains_unbound_param())
        }
        Expr::JsonObject(pairs) => pairs.iter().any(|(_, v)| v.contains_unbound_param()),
        Expr::Aggregate(agg) => {
            agg.expression.as_ref().is_some_and(|e| e.contains_unbound_param())
                || agg.args.as_ref().is_some_and(|a| a.iter().any(|e| e.contains_unbound_param()))
        }
        Expr::Window(w) => {
            w.expression.contains_unbound_param()
                || w.partition_by.as_ref().is_some_and(|ps| ps.iter().any(|e| e.contains_unbound_param()))
        }
        Expr::JsonAgg { expr, .. } | Expr::StringAgg { expr, .. } => expr.contains_unbound_param(),
        Expr::Case(c) => {
            c.cases.iter().any(|w| w.result.contains_unbound_param())
                || c.default.as_ref().is_some_and(|d| d.contains_unbound_param())
        }
        _ => false,
    }
}

/// True if this expression tree contains a subquery
/// (`Exists`/`SubQuery`/`ArraySubQuery`), including nested inside other exprs.
/// Used to reject SQLite XOR operands that would be executed twice.
pub fn contains_subquery(&self) -> bool {
    match self {
        Expr::Exists(_) | Expr::SubQuery(_) | Expr::ArraySubQuery(_) => true,
        Expr::Binary { left, right, .. } => {
            left.contains_subquery() || right.contains_subquery()
        }
        Expr::Unary { expr, .. }
        | Expr::Cast { expr, .. }
        | Expr::Collate { expr, .. }
        | Expr::JsonPathText { expr, .. } => expr.contains_subquery(),
        Expr::Func { args, .. } | Expr::Tuple(args) | Expr::JsonArray(args) => {
            args.iter().any(|a| a.contains_subquery())
        }
        Expr::JsonObject(pairs) => pairs.iter().any(|(_, v)| v.contains_subquery()),
        Expr::Aggregate(agg) => {
            agg.expression.as_ref().is_some_and(|e| e.contains_subquery())
                || agg.args.as_ref().is_some_and(|a| a.iter().any(|e| e.contains_subquery()))
        }
        Expr::Window(w) => {
            w.expression.contains_subquery()
                || w.partition_by.as_ref().is_some_and(|ps| ps.iter().any(|e| e.contains_subquery()))
        }
        Expr::JsonAgg { expr, .. } | Expr::StringAgg { expr, .. } => expr.contains_subquery(),
        Expr::Case(c) => {
            c.cases.iter().any(|w| w.result.contains_subquery())
                || c.default.as_ref().is_some_and(|d| d.contains_subquery())
        }
        _ => false,
    }
}
```

(If a referenced field name differs — e.g. `AggregationDef` field names — align with the actual struct in this file. The `..` wildcards keep the matches robust to unrelated fields.)

- [ ] **Step 5: Add temporary compile-fix arms in Postgres**

In `crates/qcraft-postgres/src/lib.rs`, the `Expr::Binary` arm (894-924): add a branch to the outer `match op` (before `_ =>`) and two `unreachable!()` entries to the inner keyword match so both are exhaustive:

```rust
match op {
    BinaryOp::Custom(custom) => {
        render_custom_binary_op(custom.as_ref(), ctx)?;
    }
    // TEMP (Task 4 replaces): render as native infix.
    BinaryOp::Power | BinaryOp::BitwiseXor => {
        return Err(RenderError::unsupported(
            "BinaryOp",
            "Power/BitwiseXor rendering not yet implemented",
        ));
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
            BinaryOp::Power | BinaryOp::BitwiseXor => unreachable!(),
            BinaryOp::Custom(_) => unreachable!(),
        });
    }
};
```

Ensure `RenderError` is in scope (the file already imports it — it's used elsewhere; if not, add `use qcraft_core::error::RenderError;`).

- [ ] **Step 6: Add temporary compile-fix arms in SQLite**

In `crates/qcraft-sqlite/src/lib.rs`, the `Expr::Binary` arm (469-495): add a branch to the outer `match op` and two `unreachable!()` entries to the inner keyword match:

```rust
match op {
    BinaryOp::Custom(_) => {
        return Err(RenderError::unsupported(
            "CustomBinaryOp",
            "SQLite does not support custom binary operators.",
        ));
    }
    // TEMP (Tasks 5-6 replace): power() and composite XOR.
    BinaryOp::Power | BinaryOp::BitwiseXor => {
        return Err(RenderError::unsupported(
            "BinaryOp",
            "Power/BitwiseXor rendering not yet implemented",
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
            BinaryOp::Power | BinaryOp::BitwiseXor => unreachable!(),
            BinaryOp::Custom(_) => unreachable!(),
        });
    }
};
```

Note: the existing SQLite arm pre-renders `left` *before* this `match` (line 470: `self.render_expr(left, ctx)?;`). Leave that line where it is for now — the temp branches `return Err` before using it, which is harmless (a bit of the left operand may be written to the buffer before erroring, but the result is discarded on error). Tasks 5-6 restructure this arm properly.

- [ ] **Step 7: Run the full build + core tests**

Run: `cargo build && cargo test -p qcraft-core`
Expected: build PASS (workspace compiles); the 4 predicate tests PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/qcraft-core/src/ast/expr.rs crates/qcraft-postgres/src/lib.rs crates/qcraft-sqlite/src/lib.rs
git commit -m "feat(core): add BinaryOp Power/BitwiseXor variants and operand predicates"
```

---

## Task 4: Postgres — render `Power` (`^`) and `BitwiseXor` (`#`)

**Files:**
- Modify: `crates/qcraft-postgres/src/lib.rs` (Binary arm — replace Task 3's temp branch)
- Test: `crates/qcraft-postgres/tests/dql.rs`

**Interfaces:**
- Consumes: `BinaryOp::Power`, `BinaryOp::BitwiseXor` (Task 3).
- Produces: PG renders `Power` as infix `^`, `BitwiseXor` as infix `#`. No guards (single render).

- [ ] **Step 1: Write the failing tests**

Add to `crates/qcraft-postgres/tests/dql.rs` (uses the existing `render` helper returning the SQL string; construct a one-column SELECT of the binary expr). Add a small helper if convenient, or inline:

```rust
fn render_expr_pg(expr: Expr) -> String {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr { expr, alias: None }],
        ..simple_query()
    };
    render(&stmt)
}

#[test]
fn pg_power_renders_caret() {
    let e = Expr::Binary {
        left: Box::new(Expr::Value(Value::Int(2))),
        op: qcraft_core::ast::expr::BinaryOp::Power,
        right: Box::new(Expr::Value(Value::Int(3))),
    };
    assert_eq!(render_expr_pg(e), "SELECT 2 ^ 3");
}

#[test]
fn pg_bitwise_xor_renders_hash() {
    let e = Expr::Binary {
        left: Box::new(Expr::field("t", "a")),
        op: qcraft_core::ast::expr::BinaryOp::BitwiseXor,
        right: Box::new(Expr::field("t", "b")),
    };
    assert_eq!(render_expr_pg(e), r#"SELECT "t"."a" # "t"."b""#);
}

#[test]
fn pg_xor_with_subquery_operand_is_allowed() {
    // PG renders once — no double-execution, so subquery operands are fine.
    let sub = Expr::SubQuery(Box::new(simple_query()));
    let e = Expr::Binary {
        left: Box::new(sub),
        op: qcraft_core::ast::expr::BinaryOp::BitwiseXor,
        right: Box::new(Expr::field("t", "b")),
    };
    // Just assert it renders without error and contains the # operator.
    let sql = render_expr_pg(e);
    assert!(sql.contains(" # "), "got: {sql}");
}
```

Confirm `Value` literals render inline as `2`/`3` in PG DQL (they do in the default non-`Percent`/parameterized path used by `render`); if `render` parameterizes values, assert `$1 ^ $2` instead — check an existing value-in-SELECT test in this file and match its expectation.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qcraft-postgres pg_power_renders_caret pg_bitwise_xor_renders_hash pg_xor_with_subquery`
Expected: FAIL — the expr renders `unsupported: BinaryOp — Power/BitwiseXor rendering not yet implemented` (panics on `.unwrap()` in the `render` helper).

- [ ] **Step 3: Replace the temp branch with real infix rendering**

In `crates/qcraft-postgres/src/lib.rs`, delete the temporary `BinaryOp::Power | BinaryOp::BitwiseXor => { return Err(...); }` branch, and move the two operators into the inner keyword match (replacing the `unreachable!()` entry):

```rust
BinaryOp::Concat => "||",
BinaryOp::Power => "^",
BinaryOp::BitwiseXor => "#",
BinaryOp::Custom(_) => unreachable!(),
```

The outer `match op` now has just `Custom(custom) => ...` and `_ => { ctx.keyword(match op { ... }) }`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p qcraft-postgres`
Expected: PASS — the 3 new tests pass; no regressions.

- [ ] **Step 5: Commit**

```bash
git add crates/qcraft-postgres/src/lib.rs crates/qcraft-postgres/tests/dql.rs
git commit -m "feat(postgres): render Power as ^ and BitwiseXor as #"
```

---

## Task 5: SQLite — render `Power` as `power(l, r)`

**Files:**
- Modify: `crates/qcraft-sqlite/src/lib.rs` (restructure Binary arm; replace Task 3 temp for `Power`)
- Test: `crates/qcraft-sqlite/tests/dql.rs`

**Interfaces:**
- Consumes: `BinaryOp::Power` (Task 3).
- Produces: SQLite renders `Power` as `power(l, r)` — operands rendered once, works in any param mode. This task also restructures the Binary arm so `left` is no longer pre-rendered before the op branch (needed for `Power`/`BitwiseXor` to control their own layout).

- [ ] **Step 1: Write the failing tests**

Add to `crates/qcraft-sqlite/tests/dql.rs` (mirror the file's `render`/`render_with_params` helpers; construct a one-column SELECT). Inline helper:

```rust
fn render_expr_sqlite(expr: Expr) -> String {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr { expr, alias: None }],
        ..simple_query()
    };
    render(&stmt) // existing helper returning SQL string
}

#[test]
fn sqlite_power_renders_power_function() {
    let e = Expr::Binary {
        left: Box::new(Expr::field("t", "a")),
        op: BinaryOp::Power,
        right: Box::new(Expr::Value(Value::Int(2))),
    };
    // Value literals in DQL are parameterized (QMark) → the 2 becomes ?.
    let sql = render_expr_sqlite(e);
    assert_eq!(sql, r#"SELECT power("t"."a", ?)"#);
}

#[test]
fn sqlite_power_with_grouped_left_operand() {
    // Caller-supplied grouping via Tuple: (a + b) ** 2
    let inner = Expr::Binary {
        left: Box::new(Expr::field("t", "a")),
        op: BinaryOp::Add,
        right: Box::new(Expr::field("t", "b")),
    };
    let e = Expr::Binary {
        left: Box::new(Expr::Tuple(vec![inner])),
        op: BinaryOp::Power,
        right: Box::new(Expr::Value(Value::Int(2))),
    };
    let sql = render_expr_sqlite(e);
    assert_eq!(sql, r#"SELECT power(("t"."a" + "t"."b"), ?)"#);
}
```

Verify against an existing SQLite DQL test whether an `Expr::Value` in a SELECT column renders as `?` (parameterized) — `render_query_stmt` uses `.with_parameterize(true)`, so yes. If the exact quoting/format differs, align the expected string with the file's existing field-render format.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qcraft-sqlite sqlite_power_renders_power_function sqlite_power_with_grouped_left`
Expected: FAIL — `unsupported: BinaryOp — Power/BitwiseXor rendering not yet implemented`.

- [ ] **Step 3: Restructure the Binary arm and add the `Power` branch**

In `crates/qcraft-sqlite/src/lib.rs`, replace the entire `Expr::Binary { left, op, right } => { ... }` arm (469-495) with a `match op` that does **not** pre-render `left`:

```rust
Expr::Binary { left, op, right } => match op {
    BinaryOp::Custom(_) => Err(RenderError::unsupported(
        "CustomBinaryOp",
        "SQLite does not support custom binary operators.",
    )),

    // power(l, r) — operands rendered once; works in any param mode.
    BinaryOp::Power => {
        ctx.keyword("power").write("(");
        self.render_expr(left, ctx)?;
        ctx.comma();
        self.render_expr(right, ctx)?;
        ctx.paren_close();
        Ok(())
    }

    // TEMP (Task 6 replaces): composite XOR with guards.
    BinaryOp::BitwiseXor => Err(RenderError::unsupported(
        "BitwiseXor",
        "SQLite XOR rendering not yet implemented",
    )),

    // Everything else stays infix.
    _ => {
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
            BinaryOp::Power | BinaryOp::BitwiseXor => unreachable!(),
            BinaryOp::Custom(_) => unreachable!(),
        });
        self.render_expr(right, ctx)
    }
},
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p qcraft-sqlite`
Expected: PASS — the 2 Power tests pass; existing infix binary tests still pass (the restructure is behavior-preserving for them).

- [ ] **Step 5: Commit**

```bash
git add crates/qcraft-sqlite/src/lib.rs crates/qcraft-sqlite/tests/dql.rs
git commit -m "feat(sqlite): render Power as power() and restructure Binary arm"
```

---

## Task 6: SQLite — render `BitwiseXor` composite with guards (both param modes)

**Files:**
- Modify: `crates/qcraft-sqlite/src/lib.rs` (replace Task 5 temp `BitwiseXor` branch)
- Test: `crates/qcraft-sqlite/tests/dql.rs` (string-assert), `crates/qcraft-sqlite/tests/integration_dql.rs` (rusqlite semantic)

**Interfaces:**
- Consumes: `BinaryOp::BitwiseXor` (Task 3), `RenderCtx::capture` + `param_style()` (Task 1), `with_param_style` (Task 2), `contains_unbound_param` + `contains_subquery` (Task 3).
- Produces: SQLite renders `BitwiseXor` as `(((L) | (R)) - ((L) & (R)))`; guards reject subquery operands (both modes) and unbound `Param` operands (non-numbered mode); numbered mode reuses each operand's params once.

- [ ] **Step 1: Write the failing string-assert tests**

Add to `crates/qcraft-sqlite/tests/dql.rs`. Reuse `render_expr_sqlite` from Task 5 and add a `render_err`-style helper if the file has one (it has `render_err` at line ~20 returning the error string). Also add a numbered-mode renderer helper:

```rust
fn render_expr_sqlite_numbered(expr: Expr) -> (String, Vec<Value>) {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr { expr, alias: None }],
        ..simple_query()
    };
    let renderer = SqliteRenderer::new().with_param_style(ParamStyle::QMarkNumbered);
    renderer.render_query_stmt(&stmt).unwrap()
}

#[test]
fn sqlite_xor_qmark_fields_canonical_form() {
    let e = Expr::Binary {
        left: Box::new(Expr::field("t", "a")),
        op: BinaryOp::BitwiseXor,
        right: Box::new(Expr::field("t", "b")),
    };
    assert_eq!(
        render_expr_sqlite(e),
        r#"SELECT ((("t"."a") | ("t"."b")) - (("t"."a") & ("t"."b")))"#
    );
}

#[test]
fn sqlite_xor_inside_parent_expr_is_isolated() {
    // 1 + (a ^ b) — outer parens must isolate the composite from +.
    let xor = Expr::Binary {
        left: Box::new(Expr::field("t", "a")),
        op: BinaryOp::BitwiseXor,
        right: Box::new(Expr::field("t", "b")),
    };
    let e = Expr::Binary {
        left: Box::new(Expr::Value(Value::Int(1))),
        op: BinaryOp::Add,
        right: Box::new(xor),
    };
    assert_eq!(
        render_expr_sqlite(e),
        r#"SELECT ? + ((("t"."a") | ("t"."b")) - (("t"."a") & ("t"."b")))"#
    );
}

#[test]
fn sqlite_xor_subquery_operand_rejected() {
    let e = Expr::Binary {
        left: Box::new(Expr::SubQuery(Box::new(simple_query()))),
        op: BinaryOp::BitwiseXor,
        right: Box::new(Expr::field("t", "b")),
    };
    let err = render_err(&QueryStmt {
        columns: vec![SelectColumn::Expr { expr: e, alias: None }],
        ..simple_query()
    });
    assert!(err.contains("BitwiseXor"), "got: {err}");
}

#[test]
fn sqlite_xor_unbound_param_rejected_in_qmark_mode() {
    let e = Expr::Binary {
        left: Box::new(Expr::Param { type_hint: None }),
        op: BinaryOp::BitwiseXor,
        right: Box::new(Expr::field("t", "b")),
    };
    let err = render_err(&QueryStmt {
        columns: vec![SelectColumn::Expr { expr: e, alias: None }],
        ..simple_query()
    });
    assert!(err.contains("BitwiseXor"), "got: {err}");
}

#[test]
fn sqlite_xor_numbered_complex_operand_binds_once() {
    // (x + y) ^ z, all unbound params, numbered mode → 3 params, reuse.
    // Left is a raw Binary (x + y); the composite wraps each operand itself,
    // so no caller Tuple is needed (a Tuple would add a second paren layer).
    let left = Expr::Binary {
        left: Box::new(Expr::Param { type_hint: None }),
        op: BinaryOp::Add,
        right: Box::new(Expr::Param { type_hint: None }),
    };
    let e = Expr::Binary {
        left: Box::new(left),
        op: BinaryOp::BitwiseXor,
        right: Box::new(Expr::Param { type_hint: None }),
    };
    let (sql, _params) = render_expr_sqlite_numbered(e);
    // ?1,?2 (from x+y) reused; ?3 from z. Max index 3 → 3 bound values per row.
    assert_eq!(
        sql,
        "SELECT (((?1 + ?2) | (?3)) - ((?1 + ?2) & (?3)))"
    );
}
```

Check `render_err`'s exact signature in the file (it builds a renderer and returns `unwrap_err().to_string()`); the default renderer is `QMark`, which is what the two rejection tests want. If `render_err` uses a different construction, mirror it.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qcraft-sqlite sqlite_xor_`
Expected: FAIL — currently `unsupported: BitwiseXor — SQLite XOR rendering not yet implemented` (the two rejection tests may already pass by coincidence of the temp message containing "BitwiseXor"; the canonical-form and numbered tests fail on wrong/`unwrap` output).

- [ ] **Step 3: Replace the temp `BitwiseXor` branch with the real implementation**

In `crates/qcraft-sqlite/src/lib.rs`, replace the temporary `BinaryOp::BitwiseXor => Err(...)` branch (from Task 5) with:

```rust
BinaryOp::BitwiseXor => {
    // Guard 1: a subquery operand would be executed twice (the composite
    // duplicates operand text). Reject in any param mode.
    if left.contains_subquery() || right.contains_subquery() {
        return Err(RenderError::unsupported(
            "BitwiseXor",
            "SQLite XOR is emulated by a composite that duplicates operands; \
             a subquery operand would execute twice — not supported",
        ));
    }
    if ctx.param_style() == ParamStyle::QMarkNumbered {
        // render-once, splice: (((L) | (R)) - ((L) & (R)))
        let l = ctx.capture(|c| self.render_expr(left, c))?;
        let r = ctx.capture(|c| self.render_expr(right, c))?;
        ctx.paren_open();
        ctx.write("((").write(&l).write(") | (").write(&r).write("))");
        ctx.write(" - ");
        ctx.write("((").write(&l).write(") & (").write(&r).write("))");
        ctx.paren_close();
        Ok(())
    } else {
        // Guard 2: unbound Param would corrupt positional binding under
        // double-render; require QMarkNumbered for that case.
        if left.contains_unbound_param() || right.contains_unbound_param() {
            return Err(RenderError::unsupported(
                "BitwiseXor",
                "SQLite XOR duplicates operands in non-numbered mode; an unbound \
                 Param would corrupt positional binding — use ParamStyle::QMarkNumbered",
            ));
        }
        ctx.paren_open();
        ctx.write("((");
        self.render_expr(left, ctx)?;
        ctx.write(") | (");
        self.render_expr(right, ctx)?;
        ctx.write("))");
        ctx.write(" - ");
        ctx.write("((");
        self.render_expr(left, ctx)?;
        ctx.write(") & (");
        self.render_expr(right, ctx)?;
        ctx.write("))");
        ctx.paren_close();
        Ok(())
    }
}
```

Note on spacing: the non-numbered path uses `ctx.write("((")` then `render_expr` — `render_expr`'s first `space_if_needed` sees the buffer ending in `(`, so no stray space is inserted. The numbered path splices already-trimmed fragments. If a test shows an unexpected space, adjust with the observed exact string (the assertions above are the intended output).

- [ ] **Step 4: Run string-assert tests to verify they pass**

Run: `cargo test -p qcraft-sqlite sqlite_xor_`
Expected: PASS — canonical form, parent-isolation, both rejections, and the numbered complex-operand reuse.

- [ ] **Step 5: Write the failing semantic (rusqlite) test**

Add to `crates/qcraft-sqlite/tests/integration_dql.rs` (imports `SqliteRenderer`, `Connection`, `Value`, `mod common`). This proves the rendered SQL computes a real XOR — catching any precedence bug:

```rust
#[test]
fn sqlite_xor_computes_real_xor_via_rusqlite() {
    use qcraft_core::render::ctx::ParamStyle;

    let conn = Connection::open_in_memory().unwrap();

    // Build:  SELECT (a ^ b)  with a, b as unbound params, numbered mode.
    let expr = Expr::Binary {
        left: Box::new(Expr::Param { type_hint: None }),
        op: BinaryOp::BitwiseXor,
        right: Box::new(Expr::Param { type_hint: None }),
    };
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr { expr, alias: None }],
        ..QueryStmt::default() // or the file's simple-query builder if no Default
    };
    let renderer = SqliteRenderer::new().with_param_style(ParamStyle::QMarkNumbered);
    let (sql, _params) = renderer.render_query_stmt(&stmt).unwrap();

    // Bind a,b per row and compare to Rust's a ^ b.
    for (a, b) in [(6_i64, 3_i64), (12, 10), (255, 0), (255, 255), (1024, 1)] {
        let got: i64 = conn
            .query_row(&sql, rusqlite::params![a, b], |row| row.get(0))
            .unwrap();
        assert_eq!(got, a ^ b, "sql={sql}");
    }
}
```

If `QueryStmt` has no `Default`, replicate the minimal-`QueryStmt` construction used elsewhere in `integration_dql.rs`. The renderer produces `SELECT (((?1) | (?2)) - ((?1) & (?2)))`; rusqlite binds `?1=a`, `?2=b`.

- [ ] **Step 6: Run the semantic test**

Run: `cargo test -p qcraft-sqlite sqlite_xor_computes_real_xor_via_rusqlite`
Expected: PASS — every pair matches `a ^ b`.

- [ ] **Step 7: Run the whole SQLite suite**

Run: `cargo test -p qcraft-sqlite`
Expected: PASS — no regressions.

- [ ] **Step 8: Commit**

```bash
git add crates/qcraft-sqlite/src/lib.rs crates/qcraft-sqlite/tests/dql.rs crates/qcraft-sqlite/tests/integration_dql.rs
git commit -m "feat(sqlite): render BitwiseXor composite with subquery/param guards"
```

---

## Task 7: Version bump + CHANGELOG

**Files:**
- Modify: `Cargo.toml` (`[workspace.package]` version, line 12)
- Modify: `CHANGELOG.md` (top)

**Interfaces:**
- Consumes: the completed feature.
- Produces: released version metadata. No code/tests.

- [ ] **Step 1: Bump the workspace version**

In `Cargo.toml`, change line 12 under `[workspace.package]`:

```toml
version = "2.5.0"
```

- [ ] **Step 2: Add the CHANGELOG entry**

Insert at the top of `CHANGELOG.md`, above `## 2.4.1`:

```markdown
## 2.5.0

### Added
- `BinaryOp::Power` (`**`) and `BinaryOp::BitwiseXor` (`^`). Postgres renders them as native infix `^` (exponentiation) and `#` (bitwise XOR). SQLite renders `Power` as `power(l, r)` and `BitwiseXor` as the composite `((l | r) - (l & r))`.
- `ParamStyle::QMarkNumbered` (`?1`, `?2`) — numbered SQLite placeholders, enabling operand reuse so the XOR composite binds each logical operand once (supports `executemany`).
- `SqliteRenderer::with_param_style` to select the placeholder style.

### Notes
- SQLite `BitwiseXor` rejects subquery operands (they would execute twice) and, in non-numbered mode, unbound-parameter operands. Use `QMarkNumbered` for unbound-parameter XOR. SQLite `power()` requires the math extension (default in SQLite ≥ 3.35).
```

- [ ] **Step 3: Verify the whole workspace builds and tests pass**

Run: `cargo build && cargo test`
Expected: PASS across all crates.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release 2.5.0 — Power and BitwiseXor operators"
```

---

## Notes / out of scope

- **Downstream amsdal-glue binding** (separate repo) — after publishing qcraft 2.5.0: map `"**" => BinaryOp::Power`, `"^" => BinaryOp::BitwiseXor` in `extract.rs`'s `"Combined"` arm (drop from the error branch); enable `ParamStyle::QMarkNumbered` on the SQLite connection and pass one value per logical param in the batch path; add golden tests. Closes finding I1 of the sql-rust migration review.
- **Volatile scalar function operands** (`random()`, etc.) in SQLite XOR execute twice — not blocked (undetectable without a volatility table); operand determinism is the caller's responsibility.
- No generic "operator renders as a function" framework (YAGNI). No SQLite math-extension emulation.
