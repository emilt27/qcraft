# Integration Testing Plan

## Мета

Перевірити що згенерований SQL коректно виконується на реальних БД
і повертає очікувані результати. Юніт-тести перевіряють текст SQL,
інтеграційні — що він працює правильно end-to-end.

## Що перевіряємо

### DDL
- Створити таблицю → перевірити структуру через метадані БД:
  - Імена колонок, типи, NOT NULL, DEFAULT значення
  - PRIMARY KEY, UNIQUE, CHECK constraints
  - FOREIGN KEY (ref table, ref columns, ON DELETE/UPDATE)
  - Індекси (колонки, unique, partial)
- ALTER TABLE → перевірити що структура змінилась
- DROP TABLE → перевірити що таблиця зникла

### DML
- INSERT з параметрами → SELECT назад → значення і порядок збігаються
- INSERT з DEFAULT/expression → SELECT → правильне обчислене значення
- UPDATE з WHERE і параметрами → SELECT → змінились тільки потрібні рядки
- DELETE з WHERE → SELECT → видалились тільки потрібні рядки
- UPSERT (ON CONFLICT) → SELECT → правильна поведінка при конфлікті

### DQL
- SELECT колонок → перевірити імена і значення в результаті
- WHERE з параметрами → повернулись тільки відфільтровані рядки
- JOIN → правильне зʼєднання таблиць
- ORDER BY → порядок рядків відповідає очікуваному
- LIMIT/OFFSET → правильна кількість і зміщення
- GROUP BY + агрегати → правильні групи і значення
- HAVING → відфільтровані групи
- DISTINCT → без дублікатів
- Subqueries → правильний результат вкладеного запиту
- CTE → правильний результат
- Параметри: `VALUES (?, ?)` з `[1, "hello"]` → отримуємо `1, "hello"`, а не навпаки

### TCL
- BEGIN + INSERT + COMMIT → дані збереглись
- BEGIN + INSERT + ROLLBACK → дані не збереглись
- SAVEPOINT + RELEASE/ROLLBACK TO → часткове скасування

## Інфраструктура

### SQLite
- `rusqlite` crate, in-memory БД (`Connection::open_in_memory()`)
- Метадані: `PRAGMA table_info(table)`, `PRAGMA index_list(table)`, `PRAGMA foreign_key_list(table)`
- Без Docker, працює скрізь, запускається завжди

### PostgreSQL
- `testcontainers` crate — піднімає Docker контейнер з PostgreSQL на рандомному порті
- `tokio-postgres` або `postgres` crate для зʼєднання
- Метадані: `information_schema.columns`, `information_schema.table_constraints`, `pg_indexes`
- За feature flag `integration-pg`, не запускається без Docker

## Структура файлів

```
crates/rquery-sqlite/tests/
  integration_ddl.rs    — DDL на реальній SQLite
  integration_dml.rs    — DML на реальній SQLite
  integration_dql.rs    — DQL на реальній SQLite
  integration_tcl.rs    — TCL на реальній SQLite

crates/rquery-postgres/tests/
  integration_ddl.rs    — DDL на реальній PostgreSQL
  integration_dml.rs    — DML на реальній PostgreSQL
  integration_dql.rs    — DQL на реальній PostgreSQL
  integration_tcl.rs    — TCL на реальній PostgreSQL
```

## Патерн тесту

```rust
// SQLite приклад
#[test]
fn insert_params_order() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE t (a INTEGER, b TEXT)", []).unwrap();

    // Генеруємо INSERT через AST + renderer
    let stmt = insert_ast("t", &["a", "b"], vec![Value::Int(42), Value::Str("hello".into())]);
    let (sql, params) = SqliteRenderer::new().render_mutation_stmt(&stmt).unwrap();

    // Виконуємо на реальній БД
    conn.execute(&sql, to_rusqlite_params(&params)).unwrap();

    // Перевіряємо результат
    let (a, b): (i64, String) = conn.query_row("SELECT a, b FROM t", [], |row| {
        Ok((row.get(0)?, row.get(1)?))
    }).unwrap();

    assert_eq!(a, 42);
    assert_eq!(b, "hello");
}
```

## Конвертація параметрів

Потрібен хелпер для конвертації `Vec<Value>` → формат драйвера:
- SQLite: `Value` → `rusqlite::types::ToSql`
- PostgreSQL: `Value` → `postgres::types::ToSql`

Це може жити в самих інтеграційних тестах або в окремому test-utils модулі.

## Запуск

```bash
# Тільки SQLite інтеграційні тести (завжди працює)
cargo test -p rquery-sqlite --test integration_ddl
cargo test -p rquery-sqlite --test integration_dml
cargo test -p rquery-sqlite --test integration_dql
cargo test -p rquery-sqlite --test integration_tcl

# PostgreSQL (потрібен Docker)
cargo test -p rquery-postgres --test integration_ddl --features integration-pg
cargo test -p rquery-postgres --test integration_dml --features integration-pg
cargo test -p rquery-postgres --test integration_dql --features integration-pg
cargo test -p rquery-postgres --test integration_tcl --features integration-pg
```
