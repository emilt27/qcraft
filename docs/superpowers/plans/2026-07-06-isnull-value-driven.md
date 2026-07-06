# Value-driven IS NULL / IS NOT NULL Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `CompareOp::IsNull` polarity driven by its boolean `right` operand (`Bool(true)` → `IS NULL`, `Bool(false)` → `IS NOT NULL`), keeping `negate` a pure `NOT (...)` wrapper.

**Architecture:** The renderer's `IsNull` arm branches on `right` to pick the keyword; a non-boolean `right` (including `Value::Null`) is a hard `RenderError`. The `is_null`/`is_not_null` constructors emit a boolean `right` with `negate: false`. Two renderers (`qcraft-sqlite`, `qcraft-postgres`) plus the shared core constructors.

**Tech Stack:** Rust workspace (`qcraft-core`, `qcraft-sqlite`, `qcraft-postgres`), `cargo test`, `rusqlite` (sqlite integration), `testcontainers` (postgres integration, needs Docker).

## Global Constraints

- `right` for `CompareOp::IsNull` is **strictly boolean**. `Bool(true)` → `IS NULL`, `Bool(false)` → `IS NOT NULL`, anything else (incl. `Value::Null`) → `RenderError::unsupported`.
- `negate` / `negated` are unchanged — pure generic `NOT (...)` wrappers.
- The boolean `right` is consumed as a keyword selector only; it is **never** parameterized (`IS $1` stays invalid).
- No AI/Claude attribution anywhere in commits, code, or docs.
- Design reference: `docs/superpowers/specs/2026-07-06-isnull-value-driven-design.md`.
- Version bump (major) + CHANGELOG are handled separately at release time — **not** in this plan.

---

### Task 1: SQLite renderer + core constructors — value-driven IsNull

**Files:**
- Modify: `crates/qcraft-core/src/ast/conditions.rs:68-80` (constructors)
- Modify: `crates/qcraft-sqlite/src/lib.rs:970-973` (`IsNull` render arm)
- Test: `crates/qcraft-sqlite/tests/dql.rs` (new exact-SQL unit tests)
- Modify: `crates/qcraft-sqlite/tests/integration_dql.rs:558-608` (fix hand-built `Value::Null` sites)

**Interfaces:**
- Consumes: `Conditions::is_null(FieldRef) -> Conditions`, `Conditions::is_not_null(FieldRef) -> Conditions`, `Conditions::negated(self) -> Self`, `Comparison { left, op, right, negate }`, `CompareOp::IsNull`, `Expr::Value(Value)`, `Value::{Bool, Null}`, `RenderError::unsupported(feature, message)`.
- Produces: after this task `Conditions::is_null` yields `right = Value::Bool(true)` and `Conditions::is_not_null` yields `right = Value::Bool(false)`, both with `negate: false`. Later tasks (postgres) rely on these constructors.

- [ ] **Step 1: Write the failing unit tests**

Append to `crates/qcraft-sqlite/tests/dql.rs` (uses existing `render`, `render_err`, `simple_query` helpers already in that file):

```rust
// ---------------------------------------------------------------------------
// IS NULL / IS NOT NULL (value-driven)
// ---------------------------------------------------------------------------

#[test]
fn where_is_null() {
    let stmt = QueryStmt {
        where_clause: Some(Conditions::is_null(FieldRef::new("users", "email"))),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" WHERE "users"."email" IS NULL"#
    );
}

#[test]
fn where_is_not_null() {
    let stmt = QueryStmt {
        where_clause: Some(Conditions::is_not_null(FieldRef::new("users", "email"))),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" WHERE "users"."email" IS NOT NULL"#
    );
}

#[test]
fn where_is_null_negated() {
    let stmt = QueryStmt {
        where_clause: Some(Conditions::is_null(FieldRef::new("users", "email")).negated()),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" WHERE NOT ("users"."email" IS NULL)"#
    );
}

#[test]
fn where_is_not_null_negated() {
    let stmt = QueryStmt {
        where_clause: Some(Conditions::is_not_null(FieldRef::new("users", "email")).negated()),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" WHERE NOT ("users"."email" IS NOT NULL)"#
    );
}

#[test]
fn is_null_non_boolean_right_errors() {
    let stmt = QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "email")),
                op: CompareOp::IsNull,
                right: Expr::Value(Value::Null),
                negate: false,
            },
        ))])),
        ..simple_query()
    };
    let err = render_err(&stmt);
    assert!(err.contains("IsNull"), "unexpected error: {err}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qcraft-sqlite --test dql -- is_null is_not_null`
Expected: FAIL — `where_is_not_null` (gets `NOT (... IS NULL)`), `where_is_not_null_negated` (gets `NOT (NOT (... IS NULL))`), and `is_null_non_boolean_right_errors` (panics: render returns `Ok`, so `unwrap_err` fails).

- [ ] **Step 3: Update the core constructors**

In `crates/qcraft-core/src/ast/conditions.rs`, replace both constructors (currently lines 67-80):

```rust
    /// `field IS NULL`
    pub fn is_null(field: FieldRef) -> Self {
        Self::comparison(field, CompareOp::IsNull, Expr::Value(Value::Bool(true)))
    }

    /// `field IS NOT NULL`
    pub fn is_not_null(field: FieldRef) -> Self {
        Self::comparison(field, CompareOp::IsNull, Expr::Value(Value::Bool(false)))
    }
```

(`Self::comparison` already sets `negate: false` and `connector: And`.)

- [ ] **Step 4: Update the SQLite renderer IsNull arm**

In `crates/qcraft-sqlite/src/lib.rs`, replace the `CompareOp::IsNull` arm (currently lines 970-973):

```rust
            CompareOp::IsNull => {
                match right {
                    Expr::Value(Value::Bool(true)) => ctx.keyword("IS NULL"),
                    Expr::Value(Value::Bool(false)) => ctx.keyword("IS NOT NULL"),
                    _ => {
                        return Err(RenderError::unsupported(
                            "IsNull",
                            "IsNull right operand must be a boolean",
                        ));
                    }
                };
                return Ok(());
            }
```

- [ ] **Step 5: Fix hand-built SQLite integration tests**

In `crates/qcraft-sqlite/tests/integration_dql.rs`:

- In `where_is_null` (line ~564), change `right: Expr::Value(Value::Null),` to `right: Expr::Value(Value::Bool(true)),` (keep `negate: false`).
- In `where_is_not_null` (lines ~591-593), change `right: Expr::Value(Value::Null),` to `right: Expr::Value(Value::Bool(false)),` **and** `negate: true,` to `negate: false,`. The `assert_eq!(rows.len(), 4)` stays valid (native `IS NOT NULL`).

- [ ] **Step 6: Run the full SQLite crate tests**

Run: `cargo test -p qcraft-sqlite`
Expected: PASS (unit `dql` tests + `integration_dql` in-memory tests all green).

- [ ] **Step 7: Commit**

```bash
git add crates/qcraft-core/src/ast/conditions.rs crates/qcraft-sqlite/src/lib.rs crates/qcraft-sqlite/tests/dql.rs crates/qcraft-sqlite/tests/integration_dql.rs
git commit -m "feat(core,sqlite): value-driven IS NULL / IS NOT NULL via boolean right"
```

---

### Task 2: PostgreSQL renderer — value-driven IsNull

**Files:**
- Modify: `crates/qcraft-postgres/src/lib.rs:1320-1323` (`IsNull` render arm)
- Test: `crates/qcraft-postgres/tests/dql.rs` (new exact-SQL unit tests)
- Modify: `crates/qcraft-postgres/tests/integration/dql.rs:511-534` (fix hand-built `Value::Null` site)

**Interfaces:**
- Consumes: the constructors updated in Task 1 (`Conditions::is_null` → `Bool(true)`, `Conditions::is_not_null` → `Bool(false)`), plus `PostgresRenderer::new()`, `render(&QueryStmt) -> String`, `simple_query()`, `SelectColumn::Star`, `FromItem::table`, `SchemaRef::new`, `RenderError::unsupported`.
- Produces: nothing consumed by later tasks.

- [ ] **Step 1: Write the failing unit tests**

Append to `crates/qcraft-postgres/tests/dql.rs` (uses existing `render` and `simple_query`; note postgres `simple_query()` has empty `columns` and `from: None`, so set them explicitly):

```rust
// ---------------------------------------------------------------------------
// IS NULL / IS NOT NULL (value-driven)
// ---------------------------------------------------------------------------

fn users_from() -> Vec<FromItem> {
    vec![FromItem::table(SchemaRef::new("users"))]
}

#[test]
fn where_is_null() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(users_from()),
        where_clause: Some(Conditions::is_null(FieldRef::new("users", "email"))),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" WHERE "users"."email" IS NULL"#
    );
}

#[test]
fn where_is_not_null() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(users_from()),
        where_clause: Some(Conditions::is_not_null(FieldRef::new("users", "email"))),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" WHERE "users"."email" IS NOT NULL"#
    );
}

#[test]
fn where_is_null_negated() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(users_from()),
        where_clause: Some(Conditions::is_null(FieldRef::new("users", "email")).negated()),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" WHERE NOT ("users"."email" IS NULL)"#
    );
}

#[test]
fn where_is_not_null_negated() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(users_from()),
        where_clause: Some(Conditions::is_not_null(FieldRef::new("users", "email")).negated()),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" WHERE NOT ("users"."email" IS NOT NULL)"#
    );
}

#[test]
fn is_null_non_boolean_right_errors() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(users_from()),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "email")),
                op: CompareOp::IsNull,
                right: Expr::Value(Value::Null),
                negate: false,
            },
        ))])),
        ..simple_query()
    };
    let err = PostgresRenderer::new()
        .render_query_stmt(&stmt)
        .unwrap_err()
        .to_string();
    assert!(err.contains("IsNull"), "unexpected error: {err}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qcraft-postgres --test dql -- is_null is_not_null`
Expected: FAIL — `where_is_not_null` gets `IS NULL` (old renderer ignores `right`), and `is_null_non_boolean_right_errors` panics (render returns `Ok`).

- [ ] **Step 3: Update the PostgreSQL renderer IsNull arm**

In `crates/qcraft-postgres/src/lib.rs`, replace the `CompareOp::IsNull` arm (currently lines 1320-1323):

```rust
            CompareOp::IsNull => {
                match right {
                    Expr::Value(Value::Bool(true)) => ctx.keyword("IS NULL"),
                    Expr::Value(Value::Bool(false)) => ctx.keyword("IS NOT NULL"),
                    _ => {
                        return Err(RenderError::unsupported(
                            "IsNull",
                            "IsNull right operand must be a boolean",
                        ));
                    }
                };
                return Ok(());
            }
```

- [ ] **Step 4: Fix hand-built PostgreSQL integration test**

In `crates/qcraft-postgres/tests/integration/dql.rs`, `where_is_null` (line ~521): change `right: Expr::Value(Value::Null),` to `right: Expr::Value(Value::Bool(true)),` (keep `negate: false`). Row assertion (Eve) stays valid.

- [ ] **Step 5: Run the PostgreSQL unit tests**

Run: `cargo test -p qcraft-postgres --test dql`
Expected: PASS.

> Note: `cargo test -p qcraft-postgres --test integration` requires Docker (testcontainers). Run it if Docker is available; otherwise the `dql` unit tests above are the authoritative fast check for this task.

- [ ] **Step 6: Commit**

```bash
git add crates/qcraft-postgres/src/lib.rs crates/qcraft-postgres/tests/dql.rs crates/qcraft-postgres/tests/integration/dql.rs
git commit -m "feat(postgres): value-driven IS NULL / IS NOT NULL via boolean right"
```

---

### Task 3: Documentation

**Files:**
- Modify: `docs/type-reference.md:65,174`
- Verify: `docs/type-reference.md:274-275`, `docs/select-queries.md:327-337` (already claim `IS NOT NULL` — confirm now correct)

**Interfaces:**
- Consumes: nothing. Produces: nothing.

- [ ] **Step 1: Update the IsNull operator mapping**

In `docs/type-reference.md`, the operator table row (line ~174) currently reads:

```
IsNull          IS NULL
```

Replace with:

```
IsNull          IS NULL / IS NOT NULL   (selected by boolean `right`: true → IS NULL, false → IS NOT NULL)
```

- [ ] **Step 2: Clarify the parameterization note**

In `docs/type-reference.md` (line ~65), replace:

```
The only exception is `CompareOp::IsNull` — it always renders as `IS NULL` because `IS $1` is not valid SQL syntax.
```

with:

```
The exception is `CompareOp::IsNull`: its boolean `right` selects the keyword (`Bool(true)` → `IS NULL`, `Bool(false)` → `IS NOT NULL`) and is never emitted as a bind parameter, since `IS $1` is not valid SQL syntax. A non-boolean `right` is a render error.
```

- [ ] **Step 3: Verify the SELECT docs are now accurate**

Confirm `docs/select-queries.md:327-337` and `docs/type-reference.md:274-275` show `Conditions::is_not_null(...)` → `... IS NOT NULL`. These are now produced by the renderer; no change needed if the text already matches. Adjust only if they still describe the old `NOT (... IS NULL)` output.

- [ ] **Step 4: Commit**

```bash
git add docs/type-reference.md docs/select-queries.md
git commit -m "docs: document value-driven IS NULL / IS NOT NULL"
```

---

## Self-Review

**Spec coverage:**
- Value-driven semantics table → Task 1 Step 4 + Task 2 Step 3 (renderer branch). ✓
- Strictly-boolean `right`, error otherwise → renderer `_ =>` arm + error tests. ✓
- `negate` unchanged → not modified; negated tests assert `NOT (...)` wrapping. ✓
- Constructors emit boolean `right` → Task 1 Step 3. ✓
- TDD failing-first for native `IS NOT NULL` → Task 1/2 Step 1-2. ✓
- Both dialects → Task 1 (sqlite) + Task 2 (postgres). ✓
- Update hand-built `Value::Null` tests → Task 1 Step 5, Task 2 Step 4. ✓
- Docs → Task 3. ✓
- Version/CHANGELOG deferred to release → Global Constraints. ✓

**Placeholder scan:** No TBD/TODO; all code blocks concrete. ✓

**Type consistency:** `RenderError::unsupported(feature, message)`, `Value::Bool(bool)`, `Expr::Value(Value)`, `Conditions::is_null/is_not_null/negated`, `render`/`render_err`/`simple_query` helpers match their source definitions. ✓
