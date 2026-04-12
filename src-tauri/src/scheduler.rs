use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// The type of job a scheduled task runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobType {
    Query,
    Import,
}

impl JobType {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobType::Query => "Query",
            JobType::Import => "Import",
        }
    }
}

impl FromStr for JobType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Query" => Ok(JobType::Query),
            "Import" => Ok(JobType::Import),
            other => Err(format!("Unknown job_type: {}", other)),
        }
    }
}

fn default_timezone() -> String {
    "UTC".to_string()
}

/// A single scheduled job entry stored in _tdw.scheduled_jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    /// UUID identifying this job.
    pub id: String,
    pub name: String,
    pub job_type: JobType,
    /// Script path (for Query) or table name (for Import).
    pub target_id: String,
    /// Cron expression, e.g. "0 * * * *".
    pub cron_expr: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    pub enabled: bool,
    pub created_at: String,
    pub last_run_at: Option<String>,
}
