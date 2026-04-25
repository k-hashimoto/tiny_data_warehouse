use crate::commands::ensure_not_mcp_locked;
use crate::db::types::{CsvImportOptions, CsvPreviewResult, TableInfo};
use crate::db::worker::DbWorker;
use crate::error::AppError;
use crate::mcp::McpLock;
use crate::utils;
use tauri::State;

#[tauri::command]
pub async fn reimport_csv(
    schema_name: String,
    table_name: String,
    db: State<'_, DbWorker>,
    lock: State<'_, McpLock>,
) -> Result<TableInfo, String> {
    ensure_not_mcp_locked(&lock)?;
    db.reimport_csv(schema_name, table_name).await
}

#[tauri::command]
pub async fn preview_csv(
    opts: CsvImportOptions,
    db: State<'_, DbWorker>,
    lock: State<'_, McpLock>,
) -> Result<CsvPreviewResult, String> {
    ensure_not_mcp_locked(&lock)?;
    db.preview_csv(opts).await
}

#[tauri::command]
pub async fn import_csv(
    opts: CsvImportOptions,
    db: State<'_, DbWorker>,
    lock: State<'_, McpLock>,
) -> Result<TableInfo, String> {
    ensure_not_mcp_locked(&lock)?;
    db.import_csv(opts).await
}

#[tauri::command]
pub async fn export_csv(path: String, content: String) -> Result<(), String> {
    let write = || -> Result<(), AppError> {
        std::fs::write(&path, content)?;
        Ok(())
    };
    write().map_err(|e| e.to_string())
}

/// Export all rows of a query directly to CSV via DuckDB's COPY TO (no row limit).
/// Returns the full path of the saved file.
#[tauri::command]
pub async fn export_query_csv(
    sql: String,
    export_dir: String,
    filename: String,
    db: State<'_, DbWorker>,
    lock: State<'_, McpLock>,
) -> Result<String, String> {
    ensure_not_mcp_locked(&lock)?;
    let inner = || -> Result<String, AppError> {
        // Resolve ~ to home directory
        let resolved_dir = std::path::PathBuf::from(utils::expand_home_path(&export_dir));
        std::fs::create_dir_all(&resolved_dir)?;
        let out_path = resolved_dir.join(&filename);
        let out_path_str = out_path
            .to_str()
            .ok_or_else(|| AppError::Other("Invalid path".to_string()))?
            .replace('\'', "''");
        Ok(out_path_str)
    };
    let out_path_str = inner().map_err(|e| e.to_string())?;

    // Reconstruct out_path for the return value
    let resolved_dir = std::path::PathBuf::from(utils::expand_home_path(&export_dir));
    let out_path = resolved_dir.join(&filename);

    // Use DuckDB COPY TO for full export without row limit
    let copy_sql = format!(
        "COPY ({}) TO '{}' (HEADER, DELIMITER ',')",
        sql.trim_end_matches(';'),
        out_path_str
    );

    db.query(copy_sql).await?;

    Ok(out_path.to_str().unwrap_or("").to_string())
}
