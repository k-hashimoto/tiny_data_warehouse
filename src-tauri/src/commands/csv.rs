use std::sync::atomic::Ordering;
use tauri::{Manager, State};
use crate::db::worker::DbWorker;
use crate::db::types::{CsvImportOptions, CsvPreviewResult, TableInfo};
use crate::mcp::McpLock;

#[tauri::command]
pub async fn reimport_csv(schema_name: String, table_name: String, db: State<'_, DbWorker>, lock: State<'_, McpLock>) -> Result<TableInfo, String> {
    if lock.0.load(Ordering::SeqCst) {
        return Err("AI操作中です。しばらくお待ちください。".into());
    }
    db.reimport_csv(schema_name, table_name).await
}

#[tauri::command]
pub async fn preview_csv(opts: CsvImportOptions, db: State<'_, DbWorker>, lock: State<'_, McpLock>) -> Result<CsvPreviewResult, String> {
    if lock.0.load(Ordering::SeqCst) {
        return Err("AI操作中です。しばらくお待ちください。".into());
    }
    db.preview_csv(opts).await
}

#[tauri::command]
pub async fn import_csv(opts: CsvImportOptions, db: State<'_, DbWorker>, lock: State<'_, McpLock>) -> Result<TableInfo, String> {
    if lock.0.load(Ordering::SeqCst) {
        return Err("AI操作中です。しばらくお待ちください。".into());
    }
    db.import_csv(opts).await
}

#[tauri::command]
pub async fn export_csv(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

/// Export all rows of a query directly to CSV via DuckDB's COPY TO (no row limit).
/// Returns the full path of the saved file.
#[tauri::command]
pub async fn export_query_csv(
    app: tauri::AppHandle,
    sql: String,
    export_dir: String,
    filename: String,
    db: State<'_, DbWorker>,
    lock: State<'_, McpLock>,
) -> Result<String, String> {
    if lock.0.load(Ordering::SeqCst) {
        return Err("AI操作中です。しばらくお待ちください。".into());
    }
    // Resolve ~ to home directory
    let resolved_dir = if export_dir.starts_with("~/") {
        let home = app.path().home_dir().map_err(|e| e.to_string())?;
        home.join(&export_dir[2..])
    } else {
        std::path::PathBuf::from(&export_dir)
    };

    std::fs::create_dir_all(&resolved_dir).map_err(|e| e.to_string())?;

    let out_path = resolved_dir.join(&filename);
    let out_path_str = out_path.to_str().ok_or("Invalid path")?.replace('\'', "''");

    // Use DuckDB COPY TO for full export without row limit
    let copy_sql = format!(
        "COPY ({}) TO '{}' (HEADER, DELIMITER ',')",
        sql.trim_end_matches(';'),
        out_path_str
    );

    db.query(copy_sql).await?;

    Ok(out_path.to_str().unwrap_or("").to_string())
}
