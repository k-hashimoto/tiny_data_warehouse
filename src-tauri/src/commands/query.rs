use tauri::State;
use crate::db::worker::DbWorker;
use crate::db::types::QueryResult;
use crate::mcp::McpLock;
use crate::commands::ensure_not_mcp_locked;

#[tauri::command]
pub async fn run_query(sql: String, db: State<'_, DbWorker>, lock: State<'_, McpLock>) -> Result<QueryResult, String> {
    ensure_not_mcp_locked(&lock)?;
    db.query(sql).await
}
