use crate::db::worker::DbWorker;
use std::path::{Path, PathBuf};
use tauri::Manager;

fn macros_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let dir = home.join(".tdwh").join("macros");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Macro name must not be empty".into());
    }
    let p = Path::new(name);
    for component in p.components() {
        match component {
            std::path::Component::Normal(_) => {}
            _ => return Err("Invalid macro name".into()),
        }
    }
    Ok(())
}

fn scan_macros(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("sql"))
        .filter_map(|e| {
            e.path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .collect();
    names.sort();
    names
}

/// Load all macro SQL files from the macros directory and execute them against DuckDB.
/// Called on app startup.
pub async fn load_macros_on_startup(dir: &Path, db: &DbWorker) {
    let names = scan_macros(dir);
    for name in &names {
        let path = dir.join(format!("{}.sql", name));
        if let Ok(sql) = std::fs::read_to_string(&path) {
            let _ = db.query(sql).await;
        }
    }
}

#[tauri::command]
pub fn list_macros(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let dir = macros_dir(&app)?;
    Ok(scan_macros(&dir))
}

#[tauri::command]
pub fn read_macro(app: tauri::AppHandle, name: String) -> Result<String, String> {
    validate_name(&name)?;
    let path = macros_dir(&app)?.join(format!("{}.sql", name));
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_macro(app: tauri::AppHandle, name: String, content: String) -> Result<(), String> {
    validate_name(&name)?;
    let path = macros_dir(&app)?.join(format!("{}.sql", name));
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

/// Re-execute all macro SQL files against DuckDB.
/// Returns list of successfully loaded macro names, or an error with details.
#[tauri::command]
pub async fn reload_macros(
    app: tauri::AppHandle,
    db: tauri::State<'_, DbWorker>,
) -> Result<Vec<String>, String> {
    let dir = macros_dir(&app)?;
    let names = scan_macros(&dir);
    let mut loaded = Vec::new();
    let mut errors = Vec::new();

    for name in &names {
        let path = dir.join(format!("{}.sql", name));
        match std::fs::read_to_string(&path) {
            Ok(sql) => match db.query(sql).await {
                Ok(_) => loaded.push(name.clone()),
                Err(e) => errors.push(format!("{}: {}", name, e)),
            },
            Err(e) => errors.push(format!("{}: {}", name, e)),
        }
    }

    if errors.is_empty() {
        Ok(loaded)
    } else {
        Err(errors.join("\n"))
    }
}
