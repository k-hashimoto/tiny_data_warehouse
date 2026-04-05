use tauri::State;
use crate::db::worker::DbWorker;
use crate::db::types::{JsonImportOptions, JsonPreviewResult, TableInfo};
use crate::mcp::McpLock;
use crate::commands::ensure_not_mcp_locked;

#[tauri::command]
pub async fn preview_json(opts: JsonImportOptions, db: State<'_, DbWorker>, lock: State<'_, McpLock>) -> Result<JsonPreviewResult, String> {
    ensure_not_mcp_locked(&lock)?;
    db.preview_json(opts).await
}

#[tauri::command]
pub async fn import_json(opts: JsonImportOptions, db: State<'_, DbWorker>, lock: State<'_, McpLock>) -> Result<TableInfo, String> {
    ensure_not_mcp_locked(&lock)?;
    db.import_json(opts).await
}
