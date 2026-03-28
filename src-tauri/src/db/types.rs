use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
    pub elapsed_ms: u128,
    pub truncated: bool,   // true if rows were limited to RESULT_ROW_LIMIT
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TableInfo {
    pub name: String,
    pub schema_name: String,
    pub row_count: i64,
    pub column_count: i64,
    pub csv_source_path: Option<String>,
    pub table_type: String, // "table" or "view"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub column_type: String,
    pub nullable: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SchemaResult {
    pub table_name: String,
    pub columns: Vec<ColumnInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CsvImportOptions {
    pub file_path: String,
    pub table_name: String,
    pub schema_name: String,
    pub has_header: bool,
    pub delimiter: String,
    pub encoding: String,   // "utf8" | "sjis"
    pub if_exists: String,  // "replace" | "append" | "fail"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CsvPreviewResult {
    pub preview: QueryResult,
    pub suggested_table_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColumnMeta {
    pub name: String,
    pub data_type: String,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TableMeta {
    pub schema_name: String,
    pub table_name: String,
    pub comment: Option<String>,
    pub columns: Vec<ColumnMeta>,
}
