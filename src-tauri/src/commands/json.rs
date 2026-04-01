use std::sync::atomic::Ordering;
use tauri::State;
use crate::db::worker::DbWorker;
use crate::db::types::{JsonImportOptions, JsonPreviewResult, TableInfo};
use crate::mcp::McpLock;

#[tauri::command]
pub async fn preview_json(opts: JsonImportOptions, db: State<'_, DbWorker>, lock: State<'_, McpLock>) -> Result<JsonPreviewResult, String> {
    if lock.0.load(Ordering::SeqCst) {
        return Err("AI操作中です。しばらくお待ちください。".into());
    }
    db.preview_json(opts).await
}

#[tauri::command]
pub async fn import_json(opts: JsonImportOptions, db: State<'_, DbWorker>, lock: State<'_, McpLock>) -> Result<TableInfo, String> {
    if lock.0.load(Ordering::SeqCst) {
        return Err("AI操作中です。しばらくお待ちください。".into());
    }
    db.import_json(opts).await
}
