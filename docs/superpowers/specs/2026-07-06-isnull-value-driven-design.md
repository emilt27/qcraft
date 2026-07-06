# Value-driven `IS NULL` / `IS NOT NULL`

**Date:** 2026-07-06
**Status:** Approved (design)

## Problem

`CompareOp::IsNull` conflates two orthogonal concerns:

- **Null polarity** (`IS NULL` vs `IS NOT NULL`) — in Django this is driven by the
  lookup value: `field__isnull=True` → `IS NULL`, `field__isnull=False` → `IS NOT NULL`.
- **Negation** (`~Q(...)`) — a generic `NOT (...)` wrapper around any predicate.

Today the renderer **ignores `right`** for `IsNull` and always emits `IS NULL`
(`qcraft-sqlite/src/lib.rs:970`, `qcraft-postgres/src/lib.rs:1320`). Null polarity is
instead smuggled through the `negate` flag: `is_not_null` is built as
`IsNull + negate:true`, which renders `NOT (field IS NULL)`.

Consequences:

1. **`right` is dead data** for `IsNull` — the Django boolean has nowhere to live.
   A binding that maps `"ISNULL" => CompareOp::IsNull` and drops the boolean silently
   collapses `isnull=False` into `IS NULL` (the root of the observed binding bug).
2. **No native `IS NOT NULL` token** — `is_not_null()` emits `NOT (field IS NULL)`.
3. **Docs are wrong** — `type-reference.md:275` and `select-queries.md:334-336`
   promise `field IS NOT NULL`, which the renderer never produces.
4. **Ugly double negation** — `~Q(field__isnull=False)` becomes
   `NOT (NOT (field IS NULL))`.

## Approach

Make null polarity **value-driven** (Approach A: boolean encoded in `right`), matching
Django 1:1. Keep `negate` as a pure, generic `NOT (...)` wrapper (it already is —
`qcraft-sqlite/src/lib.rs:864-870` has no `IsNull` special-casing).

This was chosen over a dedicated `CompareOp::IsNotNull` variant (Approach B) because it
mirrors Django's "one lookup, boolean rhs" model exactly and makes the PyO3 binding a
pure pass-through of `right` — the strongest fix for the original bug — with a minimal,
mostly backward-compatible code change.

### AST semantics for `CompareOp::IsNull`

The `right` operand encodes the Django `isnull` boolean and is consumed by the renderer
as a **keyword selector** — it is never emitted as a bind parameter, preserving the
existing `IS $1 is not valid SQL` invariant.

| `right`                    | rendered keyword |
| -------------------------- | ---------------- |
| `Expr::Value(Value::Bool(true))`  | `IS NULL`        |
| `Expr::Value(Value::Bool(false))` | `IS NOT NULL`    |
| `Expr::Value(Value::Null)` (legacy) | `IS NULL`      |
| anything else              | `RenderError::unsupported` |

`Comparison.negate` and `Conditions.negated` are unchanged — they wrap the produced
predicate in `NOT (...)`. This yields the full, orthogonal truth table:

| AST (`op=IsNull`)                | SQL                          | ≡           |
| -------------------------------- | ---------------------------- | ----------- |
| `right=Bool(true)`               | `field IS NULL`              |             |
| `right=Bool(false)`              | `field IS NOT NULL`          |             |
| `right=Bool(true), negate`       | `NOT (field IS NULL)`        | `IS NOT NULL` |
| `right=Bool(false), negate`      | `NOT (field IS NOT NULL)`    | `IS NULL`   |

The two redundant spellings (rows 3/4) are intentional and expected — the same
`Eq+negate` vs `Neq` redundancy — and match Django, which also emits `NOT (x IS NULL)`
for `~Q(x__isnull=True)` rather than simplifying.

## Changes

### Renderers (`qcraft-sqlite`, `qcraft-postgres`)

The `CompareOp::IsNull` arm of `render_compare_op` branches on `right`:

```rust
CompareOp::IsNull => {
    match right {
        Expr::Value(Value::Bool(false)) => ctx.keyword("IS NOT NULL"),
        Expr::Value(Value::Bool(true)) | Expr::Value(Value::Null) => ctx.keyword("IS NULL"),
        _ => return Err(RenderError::unsupported(
            "IsNull",
            "IsNull right operand must be a boolean",
        )),
    };
    return Ok(());
}
```

`left` is already rendered before the `match op` block, so this arm only chooses the
trailing keyword. The boolean is not passed to `render_expr`, so it never becomes a
parameter. During implementation, confirm no separate AST walk parameterizes `right`
(`contains_unbound_param` at `conditions.rs:196` treats a `Value::Bool` literal as bound,
so it is safe).

### Constructors (`qcraft-core/src/ast/conditions.rs`)

Update every `is_null` / `is_not_null` constructor (both the `Conditions::*` and any
`Comparison::*` forms):

- `is_null` → `right = Value::Bool(true)`, `negate = false`
- `is_not_null` → `right = Value::Bool(false)`, `negate = false`
  (replacing the current `right = Null` + `negate = true` construction)

### Tests (TDD — write failing first)

Exact-SQL unit tests for **both** dialects:

- `is_null` → `... IS NULL`
- `is_not_null` → `... IS NOT NULL` (fails on current code — proves the fix)
- `is_null().negated()` → `NOT (... IS NULL)`
- `is_not_null().negated()` → `NOT (... IS NOT NULL)`
- `IsNull` with a non-boolean, non-null `right` → `RenderError`
- back-compat: raw `Comparison { op: IsNull, right: Value::Null }` → `... IS NULL`

Update existing integration tests that construct `IsNull` by hand
(`integration_dql.rs:558`, `:585`) to reflect the new constructor output where they
assert SQL; their row-count assertions remain valid.

### Docs

- `type-reference.md:174` — show both `IS NULL` and `IS NOT NULL` for `IsNull`.
- `type-reference.md:65` — clarify that the boolean `right` selects the keyword and is
  never parameterized (the `IS $1` invariant still holds).
- `type-reference.md:275`, `select-queries.md:334-336` — already claim `IS NOT NULL`;
  they become correct, no text change needed (verify).
- Document the `right` boolean semantics for `IsNull`.

## Versioning

`is_not_null()` output changes textually (`NOT (field IS NULL)` → `field IS NOT NULL`),
semantically identical. Treat as a **minor** bump with a CHANGELOG entry; finalize the
version at release time.

## Out of scope

The PyO3 binding (separate `amsdal` repo) is not touched here. After this change its fix
is a trivial pass-through of Django's `isnull` boolean into `right`.
