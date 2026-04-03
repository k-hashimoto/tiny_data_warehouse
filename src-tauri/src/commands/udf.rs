use tauri::State;
use serde::Serialize;
use crate::db::worker::DbWorker;
use crate::db::sql_util;

#[derive(Serialize)]
pub struct UdfInfo {
    pub name: String,
    pub params: String,
    pub definition: String,
}

fn col_idx(columns: &[String], name: &str) -> Option<usize> {
    columns.iter().position(|c| c == name)
}

fn row_str(row: &[serde_json::Value], idx: usize) -> String {
    match row.get(idx) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
        None => String::new(),
    }
}

#[tauri::command]
pub async fn list_udfs(db: State<'_, DbWorker>) -> Result<Vec<UdfInfo>, String> {
    let sql = "SELECT function_name, \
               array_to_string(parameters, ', ') AS params, \
               macro_definition \
               FROM duckdb_functions() \
               WHERE function_type IN ('macro', 'table_macro') \
               AND schema_name = 'main' \
               AND internal = false \
               ORDER BY function_name".to_string();
    let result = db.query(sql).await?;
    let name_idx = col_idx(&result.columns, "function_name").ok_or("column not found")?;
    let params_idx = col_idx(&result.columns, "params").ok_or("column not found")?;
    let def_idx = col_idx(&result.columns, "macro_definition").ok_or("column not found")?;
    let udfs = result.rows.iter().map(|row| UdfInfo {
        name: row_str(row, name_idx),
        params: row_str(row, params_idx),
        definition: row_str(row, def_idx),
    }).collect();
    Ok(udfs)
}

/// Reconstruct the CREATE OR REPLACE MACRO SQL for a given UDF name.
#[tauri::command]
pub async fn get_udf_sql(name: String, db: State<'_, DbWorker>) -> Result<String, String> {
    let sql = format!(
        "SELECT array_to_string(parameters, ', ') AS params, macro_definition, function_type \
         FROM duckdb_functions() \
         WHERE function_name = {} \
         AND schema_name = 'main' \
         AND function_type IN ('macro', 'table_macro') \
         AND internal = false \
         LIMIT 1",
        sql_util::literal(&name)
    );
    let result = db.query(sql).await?;
    let row = result.rows.into_iter().next().ok_or_else(|| format!("UDF '{}' not found", name))?;
    let params_idx = col_idx(&result.columns, "params").ok_or("column not found")?;
    let def_idx = col_idx(&result.columns, "macro_definition").ok_or("column not found")?;
    let params = row_str(&row, params_idx);
    let definition = row_str(&row, def_idx);
    Ok(format!(
        "CREATE OR REPLACE MACRO {}({}) AS {};",
        sql_util::ident(&name),
        params,
        definition
    ))
}

/// Execute arbitrary SQL to register a UDF.
/// Automatically upgrades `CREATE MACRO` to `CREATE OR REPLACE MACRO` so that
/// overwriting an existing macro does not require manual edits.
#[tauri::command]
pub async fn save_udf(sql: String, db: State<'_, DbWorker>) -> Result<(), String> {
    let sql_trimmed = sql.trim().to_string();
    let sql_upper = sql_trimmed.to_uppercase();
    let normalized = if sql_upper.starts_with("CREATE MACRO ")
        && !sql_upper.starts_with("CREATE OR REPLACE ")
    {
        // Insert "OR REPLACE " after "CREATE "
        format!("CREATE OR REPLACE {}", &sql_trimmed["CREATE ".len()..])
    } else {
        sql_trimmed
    };
    db.query(normalized).await.map(|_| ())
}

#[tauri::command]
pub async fn delete_udf(name: String, db: State<'_, DbWorker>) -> Result<(), String> {
    let sql = format!("DROP MACRO IF EXISTS {}", sql_util::ident(&name));
    db.query(sql).await.map(|_| ())
}

#[tauri::command]
pub async fn rename_udf(old_name: String, new_name: String, db: State<'_, DbWorker>) -> Result<(), String> {
    // Get current definition
    let fetch_sql = format!(
        "SELECT array_to_string(parameters, ', ') AS params, macro_definition \
         FROM duckdb_functions() \
         WHERE function_name = {} \
         AND schema_name = 'main' \
         AND function_type IN ('macro', 'table_macro') \
         AND internal = false \
         LIMIT 1",
        sql_util::literal(&old_name)
    );
    let result = db.query(fetch_sql).await?;
    let row = result.rows.into_iter().next()
        .ok_or_else(|| format!("UDF '{}' not found", old_name))?;
    let params_idx = col_idx(&result.columns, "params").ok_or("column not found")?;
    let def_idx = col_idx(&result.columns, "macro_definition").ok_or("column not found")?;
    let params = row_str(&row, params_idx);
    let definition = row_str(&row, def_idx);

    // Create with new name
    let create_sql = format!(
        "CREATE OR REPLACE MACRO {}({}) AS {}",
        sql_util::ident(&new_name),
        params,
        definition
    );
    db.query(create_sql).await?;

    // Drop old name
    let drop_sql = format!("DROP MACRO IF EXISTS {}", sql_util::ident(&old_name));
    db.query(drop_sql).await.map(|_| ())
}
