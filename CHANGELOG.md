# Changelog

## 3.2.0

### Fixed
- **Operator operands are now parenthesized when their own structure requires it.** Renderers emitted no grouping, so an operand that was itself an operator expression got re-associated by the engine's precedence rules — two different ASTs rendered to the same SQL. `Cast(JsonPathText(data, 'age'), "bigint")` produced `data->>'age'::bigint`, which PostgreSQL reads as `data ->> ('age'::bigint)` and rejects with `invalid input syntax for type bigint`. Worse, `Binary(Binary(1, +, 2), *, 3)` produced `1 + 2 * 3` and silently evaluated to 7 instead of 9, with no error from the database. Operands of `Binary`, `Unary`, `Cast`, `Collate`, `JsonPathText` and of both sides of a comparison are now bracketed when they are themselves `Binary`, `Unary`, `Collate`, `JsonPathText` or `Window`.

  Bracketing is structural rather than driven by a precedence table, because precedence is dialect-specific: SQLite binds `||` tighter than `*`, PostgreSQL binds it looser than `+`, so the same AST needs different brackets per dialect.

### Added
- `Expr::Paren(Box<Expr>)` and the `Expr::paren(expr)` constructor — explicit grouping. Operator operands are bracketed automatically, so this is only needed to group an opaque `Raw` / `Custom` expression or to force brackets for readability.
- `Expr::needs_operand_parens()` and the `Renderer::render_operand()` default method, which together implement the rule above.

### Changed
- Generated SQL now carries brackets where operand structure demands them (values are unchanged; only the SQL text differs). Snapshot tests that compare rendered SQL byte-for-byte may need updating — for example a `COLLATE` operand in a comparison now renders as `("users"."name" COLLATE "C") = $1`.
- Unary operators render via `keyword()` instead of `write()`, so `SELECT- x` is now `SELECT - x`.

### Notes
- `Expr::Raw` and `Expr::Custom` are **never** bracketed automatically: their contents are opaque and need not be an expression at all. Wrap them in `Expr::Paren` when they need grouping — e.g. `Expr::cast(Expr::paren(Expr::raw("a + b")), "text")` renders `(a + b)::text`, while the unwrapped form renders `a + b::text`.
- Adding the `Expr::Paren` variant is source-breaking for exhaustive `match` over `Expr`.

## 3.1.0

### Added
- `FieldType::Decimal { precision: Option<u32>, scale: Option<u32> }` and the `FieldType::decimal(precision, scale)` constructor. Renders `NUMERIC` / `NUMERIC(p)` / `NUMERIC(p, s)` on PostgreSQL and `DECIMAL_TEXT` / `DECIMAL_TEXT(p)` / `DECIMAL_TEXT(p, s)` on SQLite. The SQLite name is deliberately TEXT-affinity so decimal strings are stored without the precision loss that `NUMERIC`/`DECIMAL` (NUMERIC affinity → float) would cause; SQLite does not enforce `(p, s)`. A `Decimal` with `scale` but no `precision` is a `RenderError`.

### Changed
- SQLite now renders an inline (non-parameterized) `Value::Decimal` as a quoted string literal (e.g. a column `DEFAULT ('10.234')`) instead of a bare numeric literal (`10.234`), so inlined decimals land in TEXT storage and preserve precision. Parameterized rendering and PostgreSQL are unchanged.

## 3.0.0

### Changed
- **BREAKING:** `CompareOp::IsNull` is now value-driven. Its `right` operand must be a boolean: `Value::Bool(true)` renders `IS NULL`, `Value::Bool(false)` renders `IS NOT NULL`. Any other `right` (including `Value::Null`) is a `RenderError`. `negate` / `negated` remain pure `NOT (...)` wrappers, orthogonal to null polarity.
- `Conditions::is_not_null` now emits a native `IS NOT NULL` token instead of `NOT (field IS NULL)`. `Conditions::is_null` / `is_not_null` construct a boolean `right` with `negate: false`.

### Migration
- Hand-built `Comparison { op: CompareOp::IsNull, right: Expr::Value(Value::Null), .. }` must pass `Value::Bool(true)` (for `IS NULL`) or `Value::Bool(false)` (for `IS NOT NULL`). The `is_null()` / `is_not_null()` constructors already produce the correct form.

## 2.5.0

### Added
- `BinaryOp::Power` (`**`) and `BinaryOp::BitwiseXor` (`^`). Postgres renders them as native infix `^` (exponentiation) and `#` (bitwise XOR). SQLite renders `Power` as `power(l, r)` and `BitwiseXor` as the composite `(((l) | (r)) - ((l) & (r)))`.
- `ParamStyle::QMarkNumbered` (`?1`, `?2`) — numbered SQLite placeholders, enabling operand reuse so the XOR composite binds each logical operand once (supports `executemany`).
- `SqliteRenderer::with_param_style` to select the placeholder style.

### Notes
- SQLite `BitwiseXor` rejects subquery operands (they would execute twice) and, in non-numbered mode, unbound-parameter operands. Use `QMarkNumbered` for unbound-parameter XOR. SQLite `power()` requires the math extension (default in SQLite ≥ 3.35).

## 2.4.1

### Fixed
- PostgreSQL: missing space between `CACHE` and its value in identity column rendering (`.write()` → `.keyword()`)

## 2.4.0

### Added
- `Expr::CurrentTimestamp`, `Expr::CurrentDate`, `Expr::CurrentTime` — SQL standard datetime keywords rendered without parentheses (unlike `Expr::Now` which renders as `now()` / `datetime('now')`)

### Changed
- SQLite: `DEFAULT` values are now always wrapped in parentheses (`DEFAULT (expr)`) for consistency — parentheses are required for expression defaults, and are valid for literal defaults too

## 2.3.0

### Changed
- `TableSource::Values` columns are now mandatory
- Fixed SQLite compatibility issues

## 2.2.0

### Added
- `Expr::Tuple` for row/tuple constructor support
- `Expr::Param` for explicit parameter placeholders

### Fixed
- Non-btree index methods (GIN, GiST, etc.) no longer render ASC/DESC

## 2.1.2

### Fixed
- Non-btree index methods rendering ASC/DESC incorrectly

## 2.1.1

### Fixed
- SQLite renderer converting `TimeDelta` and `Array` params instead of passing as-is
- CTE body wrapping `SetOperation` in unnecessary `SELECT * FROM (...)`
- Partial unique constraint generating duplicate names

## 2.1.0

### Added
- `JsonPathText` (`->>`) expression
- Consolidated postgres integration tests into single binary

### Fixed
- Empty `table_name` and generated column qualifiers
