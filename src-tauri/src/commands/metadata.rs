use tauri::{Manager, State};
use crate::db::worker::DbWorker;
use crate::db::types::TableMeta;

fn dbt_db_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    Ok(home.join(".tdwh").join("db").join("dbt.db"))
}

#[tauri::command]
pub async fn get_table_meta(
    schema_name: String,
    table_name: String,
    db: State<'_, DbWorker>,
) -> Result<TableMeta, String> {
    db.get_table_meta(schema_name, table_name).await
}

#[tauri::command]
pub async fn get_dbt_table_meta(
    app: tauri::AppHandle,
    schema_name: String,
    table_name: String,
    db: State<'_, DbWorker>,
) -> Result<TableMeta, String> {
    let path = dbt_db_path(&app)?;
    if !path.exists() {
        return Err("dbt.db not found".into());
    }
    db.get_dbt_table_meta(path.to_str().unwrap_or("").to_string(), schema_name, table_name).await
}

#[tauri::command]
pub async fn set_table_comment(
    schema_name: String,
    table_name: String,
    comment: String,
    db: State<'_, DbWorker>,
) -> Result<(), String> {
    db.set_table_comment(schema_name, table_name, comment).await
}

#[tauri::command]
pub async fn set_column_comment(
    schema_name: String,
    table_name: String,
    column_name: String,
    comment: String,
    db: State<'_, DbWorker>,
) -> Result<(), String> {
    db.set_column_comment(schema_name, table_name, column_name, comment).await
}
