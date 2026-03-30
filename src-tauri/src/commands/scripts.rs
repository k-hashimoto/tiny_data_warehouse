use std::path::{Path, PathBuf};
use tauri::{Manager, State};
use crate::db::worker::DbWorker;
use crate::db::types::QueryResult;

fn scripts_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let dir = home.join(".tdwh").join("scripts");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// All path components must be Normal (no `..`, no absolute, no root).
fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Script name must not be empty".into());
    }
    let p = Path::new(name);
    let mut count = 0usize;
    for component in p.components() {
        match component {
            std::path::Component::Normal(_) => { count += 1; }
            _ => return Err("Invalid script name".into()),
        }
    }
    if count == 0 {
        return Err("Script name must not be empty".into());
    }
    Ok(())
}

fn collect_scripts(dir: &Path, base: &Path, names: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            collect_scripts(&path, base, names);
        } else if path.extension().and_then(|s| s.to_str()) == Some("sql") {
            if let Ok(rel) = path.strip_prefix(base) {
                if let Some(s) = rel.with_extension("").to_str() {
                    names.push(s.replace('\\', "/"));
                }
            }
        }
    }
}

const WELCOME_SQL: &str = "\
SELECT\n\
  'Tiny Data Warehouse' as item,\n\
  'https://github.com/k-hashimoto/tiny_data_warehouse' as link\n\
UNION ALL\n\
SELECT\n\
  '|- Sidebar Guide' as item,\n\
  'https://github.com/k-hashimoto/tiny_data_warehouse/blob/main/docs/sidebar.md' as link\n\
UNION ALL\n\
SELECT\n\
  '|- dbt Guide' as item,\n\
  'https://github.com/k-hashimoto/tiny_data_warehouse/blob/main/docs/dbt-integration.md' as link\n\
UNION ALL\n\
SELECT\n\
  'An introduction to SQL(DuckDB SQL)' as item,\n\
  'https://duckdb.org/docs/stable/sql/introduction' as link\n";

const SAAS_GENERATE_SQL: &str = include_str!("../../../scripts/samples/saas_marketing/01_generate.sql");
const SAAS_SUMMARY_SQL: &str  = include_str!("../../../scripts/samples/saas_marketing/02_summary.sql");

/// Seed default scripts. welcome.sql is always overwritten to stay up to date.
/// Sample scripts are created only if they don't already exist.
pub fn seed_default_scripts(app: &tauri::AppHandle) {
    let Ok(dir) = scripts_dir(app) else { return };

    // main/welcome.sql — always overwrite
    let main_dir = dir.join("main");
    if std::fs::create_dir_all(&main_dir).is_err() { return; }
    let _ = std::fs::write(main_dir.join("welcome.sql"), WELCOME_SQL);

    // samples/saas_marketing — create only if absent
    let saas_dir = dir.join("samples").join("saas_marketing");
    if std::fs::create_dir_all(&saas_dir).is_ok() {
        let gen_path = saas_dir.join("01_generate.sql");
        if !gen_path.exists() {
            let _ = std::fs::write(&gen_path, SAAS_GENERATE_SQL);
        }
        let sum_path = saas_dir.join("02_summary.sql");
        if !sum_path.exists() {
            let _ = std::fs::write(&sum_path, SAAS_SUMMARY_SQL);
        }
    }
}

#[tauri::command]
pub fn list_scripts(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let dir = scripts_dir(&app)?;
    let mut names = Vec::new();
    collect_scripts(&dir, &dir, &mut names);
    names.sort();
    Ok(names)
}

#[tauri::command]
pub fn read_script(app: tauri::AppHandle, name: String) -> Result<String, String> {
    validate_name(&name)?;
    let path = scripts_dir(&app)?.join(format!("{}.sql", name));
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_script(app: tauri::AppHandle, name: String, content: String) -> Result<(), String> {
    validate_name(&name)?;
    let path = scripts_dir(&app)?.join(format!("{}.sql", name));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_script(app: tauri::AppHandle, name: String) -> Result<(), String> {
    validate_name(&name)?;
    let path = scripts_dir(&app)?.join(format!("{}.sql", name));
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_script(app: tauri::AppHandle, old_name: String, new_name: String) -> Result<(), String> {
    validate_name(&old_name)?;
    validate_name(&new_name)?;
    let dir = scripts_dir(&app)?;
    let old_path = dir.join(format!("{}.sql", old_name));
    let new_path = dir.join(format!("{}.sql", new_name));
    if new_path.exists() {
        return Err(format!("'{}' already exists", new_name));
    }
    if let Some(parent) = new_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&old_path, &new_path).map_err(|e| e.to_string())
}

/// Execute a saved query by name. Used by the scheduler to run saved queries by reference.
#[tauri::command]
pub async fn execute_saved_query(app: tauri::AppHandle, name: String, db: State<'_, DbWorker>) -> Result<QueryResult, String> {
    let sql = read_script(app, name)?;
    db.query(sql).await
}
