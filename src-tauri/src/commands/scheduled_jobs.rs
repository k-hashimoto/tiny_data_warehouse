use crate::db::worker::DbWorker;
use crate::scheduler::ScheduledJob;
use tauri::Emitter;

#[tauri::command]
pub async fn list_scheduled_jobs(
    db: tauri::State<'_, DbWorker>,
) -> Result<Vec<ScheduledJob>, String> {
    db.list_scheduled_jobs().await
}

#[tauri::command]
pub async fn save_scheduled_job(
    app: tauri::AppHandle,
    db: tauri::State<'_, DbWorker>,
    job: ScheduledJob,
) -> Result<(), String> {
    db.save_scheduled_job(job).await?;
    let _ = app.emit("scheduled-jobs-changed", ());
    Ok(())
}

#[tauri::command]
pub async fn delete_scheduled_job(
    app: tauri::AppHandle,
    db: tauri::State<'_, DbWorker>,
    id: String,
) -> Result<(), String> {
    db.delete_scheduled_job(id).await?;
    let _ = app.emit("scheduled-jobs-changed", ());
    Ok(())
}
