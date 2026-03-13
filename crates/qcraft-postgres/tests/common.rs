use postgres::types::ToSql;
use qcraft_core::ast::value::Value;

pub fn to_pg_params(values: &[Value]) -> Vec<Box<dyn ToSql + Sync>> {
    values
        .iter()
        .map(|v| -> Box<dyn ToSql + Sync> {
            match v {
                Value::Null => Box::new(Option::<String>::None),
                Value::Bool(b) => Box::new(*b),
                Value::Int(n) => match i32::try_from(*n) {
                    Ok(i) => Box::new(i),
                    Err(_) => Box::new(*n),
                },
                Value::BigInt(n) => Box::new(*n),
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

pub fn as_pg_params(boxed: &[Box<dyn ToSql + Sync>]) -> Vec<&(dyn ToSql + Sync)> {
    boxed.iter().map(|b| b.as_ref()).collect()
}
