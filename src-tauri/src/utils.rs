use std::path::PathBuf;

/// Returns the path to the dbt DuckDB database file: ~/.tdwh/db/dbt.db
pub fn dbt_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".tdwh")
        .join("db")
        .join("dbt.db")
}

/// Expands a leading `~/` to the user's home directory.
/// Returns the path unchanged if it does not start with `~/`.
pub fn expand_home_path(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped).to_string_lossy().to_string();
        }
    }
    path.to_string()
}
