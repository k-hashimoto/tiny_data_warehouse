use anyhow::Result;
use duckdb::types::{TimeUnit, Value};
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
        Value::Decimal(d) => serde_json::Value::String(d.to_string()),
        Value::Text(s) => serde_json::Value::String(s),
        Value::Blob(b) => serde_json::Value::String(format!("{:?}", b)),
        Value::Date32(days) => serde_json::Value::String(days_to_date_str(days)),
        Value::Timestamp(unit, ts) => serde_json::Value::String(timestamp_to_str(&unit, ts)),
        Value::Time64(unit, t) => serde_json::Value::String(time64_to_str(&unit, t)),
        Value::Interval {
            months,
            days,
            nanos,
        } => serde_json::Value::String(interval_to_str(months, days, nanos)),
        Value::Enum(s) => serde_json::Value::String(s),
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
        Value::Map(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, val)| {
                    let key = match k {
                        Value::Text(s) => s.clone(),
                        other => format!("{:?}", other),
                    };
                    (key, duckdb_value_to_json(val.clone()))
                })
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

// ---------------------------------------------------------------------------
// Date / time conversion helpers (no external dependencies)
// ---------------------------------------------------------------------------

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Convert days-since-1970-01-01 to "YYYY-MM-DD".
fn days_to_date_str(days: i32) -> String {
    let mut d = days as i64;
    let mut year: i64 = 1970;

    if d >= 0 {
        loop {
            let dy = if is_leap(year) { 366 } else { 365 };
            if d < dy {
                break;
            }
            d -= dy;
            year += 1;
        }
    } else {
        loop {
            year -= 1;
            let dy = if is_leap(year) { 366 } else { 365 };
            d += dy;
            if d >= 0 {
                break;
            }
        }
    }

    let month_days: [i64; 12] = [
        31,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1i64;
    for &md in &month_days {
        if d < md {
            break;
        }
        d -= md;
        month += 1;
    }
    format!("{:04}-{:02}-{:02}", year, month, d + 1)
}

fn micros_to_hms(micros: i64) -> (i64, i64, i64, i64) {
    let secs = micros.div_euclid(1_000_000);
    let sub = micros.rem_euclid(1_000_000);
    let h = secs.div_euclid(3600);
    let m = secs.rem_euclid(3600) / 60;
    let s = secs.rem_euclid(60);
    (h, m, s, sub)
}

fn unit_to_micros(unit: &TimeUnit, t: i64) -> i64 {
    match unit {
        TimeUnit::Second => t.saturating_mul(1_000_000),
        TimeUnit::Millisecond => t.saturating_mul(1_000),
        TimeUnit::Microsecond => t,
        TimeUnit::Nanosecond => t / 1_000,
    }
}

/// Convert a DuckDB Timestamp to "YYYY-MM-DD HH:MM:SS[.ffffff]".
fn timestamp_to_str(unit: &TimeUnit, ts: i64) -> String {
    let micros = unit_to_micros(unit, ts);
    let day_micros = 86_400i64 * 1_000_000;
    let days = micros.div_euclid(day_micros) as i32;
    let time_micros = micros.rem_euclid(day_micros);
    let (h, m, s, sub) = micros_to_hms(time_micros);
    let date = days_to_date_str(days);
    if sub != 0 {
        format!("{} {:02}:{:02}:{:02}.{:06}", date, h, m, s, sub)
    } else {
        format!("{} {:02}:{:02}:{:02}", date, h, m, s)
    }
}

/// Convert a DuckDB Time64 to "HH:MM:SS[.ffffff]".
fn time64_to_str(unit: &TimeUnit, t: i64) -> String {
    let micros = unit_to_micros(unit, t);
    let (h, m, s, sub) = micros_to_hms(micros);
    if sub != 0 {
        format!("{:02}:{:02}:{:02}.{:06}", h, m, s, sub)
    } else {
        format!("{:02}:{:02}:{:02}", h, m, s)
    }
}

/// Convert a DuckDB Interval to a human-readable string.
fn interval_to_str(months: i32, days: i32, nanos: i64) -> String {
    let mut parts: Vec<String> = Vec::new();
    if months != 0 {
        let y = months / 12;
        let mo = months % 12;
        if y != 0 {
            parts.push(format!("{} year{}", y, if y.abs() != 1 { "s" } else { "" }));
        }
        if mo != 0 {
            parts.push(format!(
                "{} month{}",
                mo,
                if mo.abs() != 1 { "s" } else { "" }
            ));
        }
    }
    if days != 0 {
        parts.push(format!(
            "{} day{}",
            days,
            if days.abs() != 1 { "s" } else { "" }
        ));
    }
    if nanos != 0 {
        let micros = nanos / 1_000;
        let (h, m, s, sub) = micros_to_hms(micros);
        if sub != 0 {
            parts.push(format!("{:02}:{:02}:{:02}.{:06}", h, m, s, sub));
        } else {
            parts.push(format!("{:02}:{:02}:{:02}", h, m, s));
        }
    }
    if parts.is_empty() {
        "0 seconds".to_string()
    } else {
        parts.join(" ")
    }
}
