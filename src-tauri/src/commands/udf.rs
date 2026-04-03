use std::path::{Path, PathBuf};
use tauri::Manager;
use tauri::State;
use crate::db::worker::DbWorker;

fn udfs_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let dir = home.join(".tdwh").join("udfs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("UDF name must not be empty".into());
    }
    let p = Path::new(name);
    for component in p.components() {
        match component {
            std::path::Component::Normal(_) => {}
            _ => return Err("Invalid UDF name".into()),
        }
    }
    Ok(())
}

#[tauri::command]
pub fn list_udfs(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let dir = udfs_dir(&app)?;
    let mut names = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(names);
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("sql") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

#[tauri::command]
pub fn read_udf(app: tauri::AppHandle, name: String) -> Result<String, String> {
    validate_name(&name)?;
    let path = udfs_dir(&app)?.join(format!("{}.sql", name));
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_udf(
    app: tauri::AppHandle,
    name: String,
    sql: String,
    db: State<'_, DbWorker>,
) -> Result<(), String> {
    validate_name(&name)?;
    let path = udfs_dir(&app)?.join(format!("{}.sql", name));
    std::fs::write(&path, &sql).map_err(|e| e.to_string())?;
    db.query(sql).await.map(|_| ())
}

#[tauri::command]
pub fn delete_udf(app: tauri::AppHandle, name: String) -> Result<(), String> {
    validate_name(&name)?;
    let path = udfs_dir(&app)?.join(format!("{}.sql", name));
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}

/// Startup: load all saved UDF definitions into DuckDB.
pub async fn load_all_udfs(app: &tauri::AppHandle, db: &DbWorker) {
    let Ok(dir) = udfs_dir(app) else { return };
    let Ok(entries) = std::fs::read_dir(&dir) else { return };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("sql") {
            if let Ok(sql) = std::fs::read_to_string(&path) {
                let _ = db.query(sql).await;
            }
        }
    }
}
