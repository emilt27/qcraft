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
    List(Vec<Value>),
    Decimal(String),
    Uuid(String),
    TimeDelta {
        days: i64,
        seconds: i64,
        microseconds: i64,
    },
}
