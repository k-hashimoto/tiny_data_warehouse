use anyhow::Result;
use duckdb::types::Value;
use duckdb::{Connection, Row};
use std::sync::Mutex;

pub struct DbConnection {
    pub conn: Mutex<Connection>,
    pub path: Mutex<String>,
}

impl DbConnection {
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: Mutex::new(":memory:".to_string()),
        })
    }

    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: Mutex::new(path.to_string()),
        })
    }

    pub fn current_path(&self) -> String {
        self.path.lock().unwrap().clone()
    }
}

fn duckdb_value_to_json(v: Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(b),
        Value::TinyInt(n) => serde_json::json!(n),
        Value::SmallInt(n) => serde_json::json!(n),
        Value::Int(n) => serde_json::json!(n),
        Value::BigInt(n) => serde_json::json!(n),
        Value::HugeInt(n) => serde_json::Value::String(n.to_string()),
        Value::UTinyInt(n) => serde_json::json!(n),
        Value::USmallInt(n) => serde_json::json!(n),
        Value::UInt(n) => serde_json::json!(n),
        Value::UBigInt(n) => serde_json::json!(n),
        Value::Float(f) => serde_json::json!(f),
        Value::Double(f) => serde_json::json!(f),
        Value::Text(s) => serde_json::Value::String(s),
        Value::Blob(b) => serde_json::Value::String(format!("{:?}", b)),
        Value::List(items) | Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(duckdb_value_to_json).collect())
        }
        Value::Struct(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, val)| (k.clone(), duckdb_value_to_json(val.clone())))
                .collect();
            serde_json::Value::Object(obj)
        }
        other => serde_json::Value::String(format!("{:?}", other)),
    }
}

pub fn row_value_to_json(row: &Row, idx: usize) -> serde_json::Value {
    match row.get::<_, Value>(idx) {
        Ok(v) => duckdb_value_to_json(v),
        Err(_) => serde_json::Value::Null,
    }
}
