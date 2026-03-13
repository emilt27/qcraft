use qcraft_core::ast::value::Value;
use rusqlite::types::ToSql as RusqliteToSql;

pub fn to_sqlite_params(values: &[Value]) -> Vec<Box<dyn RusqliteToSql>> {
    values
        .iter()
        .map(|v| -> Box<dyn RusqliteToSql> {
            match v {
                Value::Null => Box::new(rusqlite::types::Null),
                Value::Bool(b) => Box::new(*b),
                Value::Int(n) | Value::BigInt(n) => Box::new(*n),
                Value::Float(f) => Box::new(*f),
                Value::Str(s) => Box::new(s.clone()),
                Value::Bytes(b) => Box::new(b.clone()),
                Value::Date(s) | Value::DateTime(s) | Value::Time(s) => Box::new(s.clone()),
                Value::Decimal(s) => Box::new(s.clone()),
                Value::Uuid(s) => Box::new(s.clone()),
                Value::Json(s) | Value::Jsonb(s) => Box::new(s.clone()),
                Value::IpNetwork(s) => Box::new(s.clone()),
                _ => Box::new(format!("{:?}", v)),
            }
        })
        .collect()
}

pub fn as_sqlite_params(boxed: &[Box<dyn RusqliteToSql>]) -> Vec<&dyn RusqliteToSql> {
    boxed.iter().map(|b| b.as_ref()).collect()
}
