# Changelog

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
