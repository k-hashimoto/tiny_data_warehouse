use tauri::State;
use crate::db::worker::DbWorker;
use crate::db::types::QueryResult;

#[tauri::command]
pub async fn run_query(sql: String, db: State<'_, DbWorker>) -> Result<QueryResult, String> {
    db.query(sql).await
}
