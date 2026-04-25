use crate::commands::ensure_not_mcp_locked;
use crate::db::types::QueryResult;
use crate::db::worker::DbWorker;
use crate::mcp::McpLock;
use tauri::State;

#[tauri::command]
pub async fn run_query(
    sql: String,
    db: State<'_, DbWorker>,
    lock: State<'_, McpLock>,
) -> Result<QueryResult, String> {
    ensure_not_mcp_locked(&lock)?;
    db.query(sql).await
}
