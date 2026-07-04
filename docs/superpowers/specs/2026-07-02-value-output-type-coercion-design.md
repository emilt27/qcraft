# Design: `Value` `output_type` coercion — where the fix belongs

**Date:** 2026-07-02
**Status:** approved direction — coercion lives in the **Python ORM layer** (amsdal), not in qcraft.
**Related:** problem statement in `docs/todo/value-output-type-coercion.md`.

## 1. Problem (desired end-state)

A Python `Value(value, output_type=<FieldType>)` must **coerce and validate** its raw Python value
to the target type before it is bound as a SQL parameter — matching Django's
`Value(value, output_field=<Field>)` behaviour via `Field.get_db_prep_value`.

This is **only** about the bound parameter value. Neither Django nor qcraft emits a `CAST` from
`output_type`; an explicit `Cast` expression is used for that (qcraft already renders `Cast`
correctly). The generated SQL string does **not** change — only what goes into the params list.

Target behaviour (verified vs Django 6.0):

| Input | Wanted bound param |
|---|---|
| `Value('5', INTEGER)` | `5` (int) — coerced |
| `Value('abc', INTEGER)` | **raise** a typed error at build time |
| `Value(5, INTEGER)` | `5` |
| `Value('5')` (no `output_type`) | `'5'` unchanged |
| `None` / SQL `NULL` | `NULL` regardless of `output_type` |

## 2. Architectural decision

**Coercion is a `Field` responsibility, and `Field`/`output_type` are Python ORM concepts.**
Therefore coercion+validation happens in the **Python ORM layer (amsdal), before the value crosses
the PyO3 boundary**. This is exactly where Django does it.

Reframing the key mistake in the original TODO: it recommended fixing this in the PyO3 binding
because that is "the field/ORM boundary." It is not. The PyO3 binding (`extract.rs` /
`value_conv.rs`) is a **serialization boundary** (Python object → Rust struct). The **ORM/Field
boundary is in Python**. The value that reaches an SQL-AST builder should already be DB-ready and
correctly typed.

### Candidate layers and full pros/cons

#### Layer 1 — Python ORM (before the Rust boundary) — **CHOSEN**

Coerce in Python at query-build time (in `Value` / the field's `get_db_prep_value` equivalent),
before handing the value to the binding. The binding then receives a native `int` / `Decimal` /
`datetime` / … and maps it type-blind, exactly as today.

- **Pros**
  - Exact Django parity: same layer, same objects, identical error types/messages
    (`"expected a number but got 'abc'"`) — trivial because it is native Python.
  - **Custom / Array / Nested / Vector types work.** Their prep logic is knowable *only* in Python.
    Any Rust-side solution is inherently incomplete for these.
  - Zero Rust changes. `qcraft-core` stays a pure SQL-AST engine; the invariant
    "a `qcraft_core::Value` is already the intended DB type" is preserved.
  - Coercion rules can change without rebuilding the Rust crate.
- **Cons**
  - Requires the Python `FieldType` to actually expose a prep hook (Django has one; amsdal may need
    it added).
  - Logic is not "centralized in Rust" — but it should not be.
  - The work lands in the **amsdal repo, not qcraft**.

#### Layer 2 — PyO3 binding (`extract.rs`) — *rejected*

Read `output_type` in the `Value` branch and coerce in Rust at the conversion edge.

- **Pros**: single enforcement point for all Python call sites; fast; no `qcraft-core` API change.
- **Cons**
  - **Incomplete for custom types**: Rust cannot know a Python `CustomType`'s prep logic → still
    needs a Python fallback → split-brain.
  - Django leniency (`'5'`→`5`, `'true'`→`True`, int ranges) must be **re-implemented in Rust**,
    losing native Python `Decimal` / `datetime`.
  - Error type/message parity across the PyO3 boundary is awkward (map a Rust error to a specific
    Python exception).
  - Logic lives inline in a binding crate: poorly isolated, duplicated if another binding appears.

#### Layer 3 — `qcraft-core` pure helper `Value::coerce_to(&FieldType)` — *rejected*

- **Pros**: centralized and unit-tested in Rust; SQL untouched; `Value` enum unchanged.
- **Cons**
  - Pushes Django **ORM semantics** (what counts as a valid bool string, etc.) into a
    **Python-agnostic SQL library** — wrong ownership, pollutes core's contract.
  - The input is an already-typed `Value::Str("5")`; "coerce a raw pre-DB value" is not an operation
    over an AST value, it is a Field operation.
  - Still cannot cover Python custom types.
  - Adds public API to core used **only** by the Python binding → ownership on the wrong layer.

#### Layer 4 — `qcraft-core` `Expr::Value` gains an optional target type — *rejected*

All of Layer 3's cons **plus** an AST + param-pipeline change for a Python-only concern.
Over-engineering.

**Conclusion:** Layer 1 is architecturally correct; Layer 2 is a lossy fallback; Layers 3–4 place
ORM semantics in the wrong crate.

## 3. Responsibility boundary (Layer 1)

| Component | Responsibility | Change |
|---|---|---|
| Python amsdal `Value` / `FieldType` | Owns coercion + validation. When `output_type is not None`, produce a native Python value already of the target type (via the field's `get_db_prep_value` equivalent). Raise a typed error on invalid input. | **New / extended** (amsdal repo) |
| PyO3 binding (`extract.rs`, `value_conv.rs`) | Stays type-blind. Reads the already-coerced `.value`. Maps native Python scalars to `qcraft_core::Value` as today. | **None** (confirm native int/Decimal/datetime mapping is correct) |
| `qcraft-core` | Strongly-typed SQL AST + rendering + param extraction. | **None** |
| `qcraft` docs (this repo) | Track the corrected decision. | **Spec correction only** |

## 4. How it works (mechanics)

- **Trigger:** coercion runs iff `Value.output_type is not None`. When `None`, bind the raw value
  unchanged (today's behaviour).
- **Where:** at query-build / compile time in Python, in `Value` (or delegated to the field's
  `get_db_prep_value`), before the value is handed to the glue/binding.
- **Coercion rules** mirror Django `get_db_prep_value`, keyed by the underlying scalar of the
  Python `FieldType`:
  - integer family (`INTEGER/SMALLINT/BIGINT/SERIAL/…`) → `int`, accepting int, numeric string,
    bool→0/1; invalid → `ValueError("expected a number but got {value!r}")`.
  - float family (`FLOAT/DOUBLE/REAL`) → `float`; invalid → raise.
  - `NUMERIC` → `Decimal`; `BOOLEAN` → `bool` (`1/0/'1'/'0'/'true'/'false'/True/False`);
    text family (`TEXT/VARCHAR/CHAR`) → `str`; `UUID` → uuid; date/time family → parse to canonical
    form; `BYTEA` → bytes; `JSON/JSONB` → JSON text; invalid → raise.
  - **Array / Nested / Vector / Custom** → recurse via the element/field's own prep (natural and
    complete in Python; the decisive advantage of Layer 1).
- **NULL:** `None` / SQL `NULL` passes through as `NULL` regardless of `output_type`.
- **Leniency:** preserve Django's — numeric strings coerce; truly invalid raise. Match Django's
  error type/message shape as closely as practical.
- **Errors surface at build time** (before SQL execution), as Python exceptions.

## 5. Changes in this repo (`qcraft`)

Only the TODO spec is corrected — **no code changes**:

- Rewrite the "Decision needed (pick one)" section of
  `docs/todo/value-output-type-coercion.md`: coercion belongs in the **Python ORM layer**, not the
  binding (A) or core (B). Explain that the PyO3 binding is a serialization boundary, not the
  ORM/Field boundary.
- Fix the `ScalarType` vs `FieldType::Scalar(String)` mismatch note: the coercion table is a
  **Python-level `FieldType`** concern; qcraft-core's `FieldType::Scalar` is stringly-typed and is
  not involved in coercion.
- Point the TODO at this design doc.

## 6. Testing

- **Python unit tests (amsdal):** the acceptance table —
  `Value('5', INTEGER) → 5`; `Value('abc', INTEGER)` raises; `Value(5, INTEGER) → 5`;
  `Value('5')` (no type) → `'5'`; `Value('1.5', FLOAT) → 1.5`; `Value('true', BOOLEAN) → True`;
  `Value('abc', FLOAT)` raises; NULL passthrough; one array/custom-type recursion case.
- **Django parity check:** round-trip against the verified table.
- **No Rust tests** are needed for coercion (Rust is unchanged).

## 7. Downstream impact (amsdal / amsdal-glue)

- Re-baseline the `amsdal-glue-connections` golden `tests/golden/test_output_type_value.py`: params
  for `Value(<str>, output_type=<numeric>)` change from string to the coerced type; invalid-value
  cases become expected raises. (It was temporarily re-baselined to type-blind behaviour during the
  Rust migration.)
- Update the migration cheatsheet note
  (`amsdal-glue` `docs/superpowers/specs/2026-06-30-model-migration-compat-notes.md`):
  `Value(x, output_type=T)` coerces + validates the param (Django parity); an explicit SQL cast
  still needs `Cast(expression=..., to_type=T)`.

## 8. Open questions / dependencies (resolve during amsdal implementation)

- Does amsdal's `FieldType` already expose a `get_db_prep_value` equivalent, or must one be added?
- Exact placement of the coercion call: on `Value` itself vs delegated to the field.
- Confirm the PyO3 binding already maps native `Decimal` / `datetime` / `uuid` to the right
  `qcraft_core::Value` variants (it should; verify while re-baselining the golden).
