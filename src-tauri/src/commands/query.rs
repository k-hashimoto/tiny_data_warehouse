use std::sync::atomic::Ordering;
use tauri::State;
use crate::db::worker::DbWorker;
use crate::db::types::QueryResult;
use crate::mcp::McpLock;

#[tauri::command]
pub async fn run_query(sql: String, db: State<'_, DbWorker>, lock: State<'_, McpLock>) -> Result<QueryResult, String> {
    if lock.0.load(Ordering::SeqCst) {
        return Err("AI操作中です。しばらくお待ちください。".into());
    }
    db.query(sql).await
}
