use anyhow::Result;
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

pub fn row_value_to_json(row: &Row, idx: usize) -> serde_json::Value {
    // Try each type in order; DuckDB will return a type error if the type doesn't match
    if let Ok(v) = row.get::<_, i64>(idx) {
        return serde_json::Value::Number(v.into());
    }
    if let Ok(v) = row.get::<_, f64>(idx) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.get::<_, bool>(idx) {
        return serde_json::Value::Bool(v);
    }
    if let Ok(v) = row.get::<_, String>(idx) {
        return serde_json::Value::String(v);
    }
    serde_json::Value::Null
}
