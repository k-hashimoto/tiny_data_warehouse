use crate::db::types::{QueryResult, SchemaResult, TableInfo};
use crate::db::worker::DbWorker;
use crate::utils;
use tauri::State;

#[tauri::command]
pub async fn list_tables(db: State<'_, DbWorker>) -> Result<Vec<TableInfo>, String> {
    db.list_tables().await
}

#[tauri::command]
pub async fn get_schema(
    schema_name: String,
    table_name: String,
    db: State<'_, DbWorker>,
) -> Result<SchemaResult, String> {
    db.get_schema(schema_name, table_name).await
}

#[tauri::command]
pub async fn preview_table(
    table_name: String,
    limit: Option<i64>,
    db: State<'_, DbWorker>,
) -> Result<QueryResult, String> {
    db.preview_table(table_name, limit.unwrap_or(100)).await
}

#[tauri::command]
pub async fn list_schemas(db: State<'_, DbWorker>) -> Result<Vec<String>, String> {
    db.list_schemas().await
}

#[tauri::command]
pub async fn list_dbt_tables(db: State<'_, DbWorker>) -> Result<Vec<TableInfo>, String> {
    let path = utils::dbt_db_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    db.list_dbt_tables(path.to_str().unwrap_or("").to_string())
        .await
}

#[tauri::command]
pub async fn preview_dbt_table(
    db: State<'_, DbWorker>,
    schema_name: String,
    table_name: String,
    limit: Option<i64>,
) -> Result<QueryResult, String> {
    let path = utils::dbt_db_path();
    if !path.exists() {
        return Err("dbt.db not found".into());
    }
    db.preview_dbt_table(
        path.to_str().unwrap_or("").to_string(),
        schema_name,
        table_name,
        limit.unwrap_or(100),
    )
    .await
}

#[tauri::command]
pub async fn get_dbt_schema(
    db: State<'_, DbWorker>,
    schema_name: String,
    table_name: String,
) -> Result<SchemaResult, String> {
    let path = utils::dbt_db_path();
    if !path.exists() {
        return Err("dbt.db not found".into());
    }
    db.get_dbt_schema(
        path.to_str().unwrap_or("").to_string(),
        schema_name,
        table_name,
    )
    .await
}

#[tauri::command]
pub async fn drop_dbt_table(
    db: State<'_, DbWorker>,
    schema_name: String,
    table_name: String,
) -> Result<(), String> {
    let path = utils::dbt_db_path();
    if !path.exists() {
        return Err("dbt.db not found".into());
    }
    db.drop_dbt_table(
        path.to_str().unwrap_or("").to_string(),
        schema_name,
        table_name,
    )
    .await
}

#[tauri::command]
pub async fn drop_dbt_schema(db: State<'_, DbWorker>, schema_name: String) -> Result<(), String> {
    let path = utils::dbt_db_path();
    if !path.exists() {
        return Err("dbt.db not found".into());
    }
    db.drop_dbt_schema(path.to_str().unwrap_or("").to_string(), schema_name)
        .await
}

#[tauri::command]
pub async fn attach_dbt(db: State<'_, DbWorker>) -> Result<(), String> {
    let path = utils::dbt_db_path();
    if !path.exists() {
        return Err("dbt.db not found".into());
    }
    db.attach_dbt(path.to_str().unwrap_or("").to_string()).await
}

#[tauri::command]
pub async fn detach_dbt(db: State<'_, DbWorker>) -> Result<(), String> {
    db.detach_dbt().await
}
