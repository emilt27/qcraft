/// A database value used in expressions, parameters, and defaults.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Bytes(Vec<u8>),
    Date(String),
    DateTime(String),
    Time(String),
    Decimal(String),
    Uuid(String),
    Json(String),
    Jsonb(String),
    IpNetwork(String),
    Array(Vec<Value>),
    Vector(Vec<f32>),
    TimeDelta {
        years: i32,
        months: i32,
        days: i64,
        seconds: i64,
        microseconds: i64,
    },
}
