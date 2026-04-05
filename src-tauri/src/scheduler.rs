use serde::{Deserialize, Serialize};

/// The kind of job a scheduled task runs.
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobKind {
    /// Run a saved SQL query by name.
    SavedQuery { name: String },
    /// Re-import a CSV file into a table.
    CsvImport { path: String },
    /// Execute a named workflow (future: KHA-10 #4).
    Workflow { id: String },
}

/// A single scheduled job entry.
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScheduledJob {
    pub id: String,
    pub kind: JobKind,
    /// Cron expression (e.g. "0 * * * *" for every hour).
    pub cron: String,
    pub enabled: bool,
}
