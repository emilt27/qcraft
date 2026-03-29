# Changelog

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
