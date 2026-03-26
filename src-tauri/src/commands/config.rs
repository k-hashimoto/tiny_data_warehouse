use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize)]
pub struct EditorConfig {
    pub line_numbers: bool,
    pub tab_size: u32,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_export_dir")]
    pub export_dir: String,
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_export_dir() -> String {
    "~/Downloads".to_string()
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            line_numbers: true,
            tab_size: 4,
            theme: default_theme(),
            export_dir: default_export_dir(),
        }
    }
}

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    Ok(home.join(".tdwh").join("config").join("editor.yml"))
}

#[tauri::command]
pub fn get_editor_config(app: tauri::AppHandle) -> Result<EditorConfig, String> {
    read_editor_config(&config_path(&app)?)
}

#[tauri::command]
pub fn save_editor_config(app: tauri::AppHandle, config: EditorConfig) -> Result<(), String> {
    let path = config_path(&app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let contents = serde_yaml::to_string(&config).map_err(|e| e.to_string())?;
    std::fs::write(&path, contents).map_err(|e| e.to_string())
}

fn read_editor_config(path: &PathBuf) -> Result<EditorConfig, String> {
    if !path.exists() {
        return Ok(EditorConfig::default());
    }
    let contents = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_yaml::from_str(&contents).map_err(|e| e.to_string())
}
