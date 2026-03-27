use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    Csv,
    Json,
    Parquet,
}

impl FileFormat {
    /// Returns the file extensions associated with this format.
    pub fn extensions(&self) -> &[&str] {
        match self {
            FileFormat::Csv => &["csv", "tsv", "txt"],
            FileFormat::Json => &["json", "jsonl", "ndjson"],
            FileFormat::Parquet => &["parquet"],
        }
    }
}
