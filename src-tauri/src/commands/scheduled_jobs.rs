use crate::db::worker::DbWorker;
use crate::scheduler::ScheduledJob;

#[tauri::command]
pub async fn list_scheduled_jobs(
    db: tauri::State<'_, DbWorker>,
) -> Result<Vec<ScheduledJob>, String> {
    db.list_scheduled_jobs().await
}

#[tauri::command]
pub async fn save_scheduled_job(
    db: tauri::State<'_, DbWorker>,
    job: ScheduledJob,
) -> Result<(), String> {
    db.save_scheduled_job(job).await
}

#[tauri::command]
pub async fn delete_scheduled_job(
    db: tauri::State<'_, DbWorker>,
    id: String,
) -> Result<(), String> {
    db.delete_scheduled_job(id).await
}
