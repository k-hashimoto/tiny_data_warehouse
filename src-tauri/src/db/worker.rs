use std::sync::mpsc;
use std::thread;
use std::time::Instant;
use duckdb::Connection;
use crate::db::connection::row_value_to_json;
use crate::db::sql_util;
use crate::db::types::{QueryResult, TableInfo, SchemaResult, ColumnInfo, CsvImportOptions, CsvPreviewResult, JsonImportOptions, JsonPreviewResult, TableMeta, ColumnMeta};
use crate::error::AppError;
use crate::file_io;
use crate::scheduler::{JobType, ScheduledJob};

pub enum WorkerCmd {
    // === クエリ実行 ===
    Query {
        sql: String,
        tx: tokio::sync::oneshot::Sender<Result<QueryResult, String>>,
    },

    // === スキーマ探索 ===
    ListTables {
        tx: tokio::sync::oneshot::Sender<Result<Vec<TableInfo>, String>>,
    },
    ListDbtTables {
        dbt_path: String,
        tx: tokio::sync::oneshot::Sender<Result<Vec<TableInfo>, String>>,
    },
    ListSchemas {
        tx: tokio::sync::oneshot::Sender<Result<Vec<String>, String>>,
    },
    GetSchema {
        schema_name: String,
        table_name: String,
        tx: tokio::sync::oneshot::Sender<Result<SchemaResult, String>>,
    },
    GetDbtSchema {
        dbt_path: String,
        schema_name: String,
        table_name: String,
        tx: tokio::sync::oneshot::Sender<Result<SchemaResult, String>>,
    },
    PreviewTable {
        table_name: String,
        limit: i64,
        tx: tokio::sync::oneshot::Sender<Result<QueryResult, String>>,
    },
    PreviewDbtTable {
        dbt_path: String,
        schema_name: String,
        table_name: String,
        limit: i64,
        tx: tokio::sync::oneshot::Sender<Result<QueryResult, String>>,
    },
    AttachDbt {
        dbt_path: String,
        tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    DetachDbt {
        tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    DropDbtTable {
        dbt_path: String,
        schema_name: String,
        table_name: String,
        tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    DropDbtSchema {
        dbt_path: String,
        schema_name: String,
        tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    },

    // === データインポート ===
    PreviewCsv {
        opts: CsvImportOptions,
        tx: tokio::sync::oneshot::Sender<Result<CsvPreviewResult, String>>,
    },
    ImportCsv {
        opts: CsvImportOptions,
        tx: tokio::sync::oneshot::Sender<Result<TableInfo, String>>,
    },
    ReimportCsv {
        schema_name: String,
        table_name: String,
        tx: tokio::sync::oneshot::Sender<Result<TableInfo, String>>,
    },
    PreviewJson {
        opts: JsonImportOptions,
        tx: tokio::sync::oneshot::Sender<Result<JsonPreviewResult, String>>,
    },
    ImportJson {
        opts: JsonImportOptions,
        tx: tokio::sync::oneshot::Sender<Result<TableInfo, String>>,
    },

    // === メタデータ管理 ===
    GetTableMeta {
        schema_name: String,
        table_name: String,
        tx: tokio::sync::oneshot::Sender<Result<TableMeta, String>>,
    },
    GetDbtTableMeta {
        dbt_path: String,
        schema_name: String,
        table_name: String,
        tx: tokio::sync::oneshot::Sender<Result<TableMeta, String>>,
    },
    SetTableComment {
        schema_name: String,
        table_name: String,
        comment: String,
        tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    SetColumnComment {
        schema_name: String,
        table_name: String,
        column_name: String,
        comment: String,
        tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    TouchTableTimestamp {
        schema_name: String,
        table_name: String,
        source: String,
        is_new: bool,
        tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    TouchDbtTimestamps {
        tables: Vec<(String, String)>, // (schema_name, table_name)
        tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    },

    // === スケジューラ ===
    ListScheduledJobs {
        tx: tokio::sync::oneshot::Sender<Result<Vec<ScheduledJob>, String>>,
    },
    SaveScheduledJob {
        job: ScheduledJob,
        tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    DeleteScheduledJob {
        id: String,
        tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
}

/// DuckDB をシングルスレッドで動かすアクター
#[derive(Clone)]
pub struct DbWorker {
    tx: mpsc::SyncSender<WorkerCmd>,
}

impl DbWorker {
    pub fn new(db_path: &str, dbt_db_path: &str) -> Self {
        let db_path = db_path.to_string();
        let dbt_db_path = dbt_db_path.to_string();
        let (tx, rx) = mpsc::sync_channel::<WorkerCmd>(64);

        thread::spawn(move || {
            let mut conn = Connection::open(&db_path)
                .expect("Failed to open DuckDB connection");
            // Limit DuckDB internal threads to avoid macOS thread conflicts
            let _ = conn.execute("SET threads=2", []);
            // Disable automatic checkpoints to prevent SIGBUS on macOS (memory-mapped page reclaim)
            let _ = conn.execute("SET checkpoint_threshold='1TB'", []);
            // Set dbt_db variable so users can ATTACH with: ATTACH getvariable('dbt_db') AS dbt
            let set_var = format!("SET VARIABLE dbt_db = {}", sql_util::literal(&dbt_db_path));
            let _ = conn.execute_batch(&set_var);
            // Initialize internal metadata schema for CSV source tracking and timestamps
            let _ = conn.execute_batch(
                "CREATE SCHEMA IF NOT EXISTS _tdw;
                 CREATE TABLE IF NOT EXISTS _tdw.csv_sources (
                     schema_name VARCHAR NOT NULL,
                     table_name  VARCHAR NOT NULL,
                     file_path   VARCHAR NOT NULL,
                     delimiter   VARCHAR NOT NULL,
                     encoding    VARCHAR NOT NULL,
                     has_header  BOOLEAN NOT NULL,
                     PRIMARY KEY (schema_name, table_name)
                 );
                 CREATE TABLE IF NOT EXISTS _tdw.table_timestamps (
                     schema_name VARCHAR NOT NULL,
                     table_name  VARCHAR NOT NULL,
                     source      VARCHAR NOT NULL,
                     created_at  VARCHAR NOT NULL,
                     updated_at  VARCHAR NOT NULL,
                     PRIMARY KEY (schema_name, table_name, source)
                 );
                 CREATE TABLE IF NOT EXISTS _tdw.scheduled_jobs (
                     id         VARCHAR PRIMARY KEY,
                     name       VARCHAR NOT NULL,
                     job_type   VARCHAR NOT NULL,
                     target_id  VARCHAR NOT NULL,
                     cron_expr  VARCHAR NOT NULL,
                     enabled    BOOLEAN DEFAULT TRUE,
                     created_at TIMESTAMP DEFAULT now(),
                     last_run_at TIMESTAMP
                 );"
            );

            // Track dbt attachment state to restore it after reconnection
            let mut dbt_attached_path: Option<String> = None;

            for cmd in rx {
                match cmd {
                    // === クエリ実行 ===
                    WorkerCmd::Query { sql, tx } => {
                        let _ = tx.send(exec_query(&conn, &sql).map_err(|e| e.to_string()));
                        // Reconnect after each user query to reset DuckDB's HTTP connection pool.
                        // This prevents stale keep-alive connections from causing JSON parse errors
                        // when using read_json_auto with external HTTP URLs.
                        // Skip for in-memory databases (used in tests): a new in-memory connection
                        // would lose all previously created tables.
                        if db_path != ":memory:" {
                            if let Ok(new_conn) = Connection::open(&db_path) {
                                let _ = new_conn.execute("SET threads=2", []);
                                let _ = new_conn.execute("SET checkpoint_threshold='1TB'", []);
                                let set_var = format!("SET VARIABLE dbt_db = {}", sql_util::literal(&dbt_db_path));
                                let _ = new_conn.execute_batch(&set_var);
                                if let Some(ref path) = dbt_attached_path {
                                    let attach_sql = format!("ATTACH {} AS dbt (READ_ONLY)", sql_util::literal(path));
                                    let _ = new_conn.execute(&attach_sql, []);
                                }
                                conn = new_conn;
                            }
                        }
                    }

                    // === スキーマ探索 ===
                    WorkerCmd::ListTables { tx } => {
                        let _ = tx.send(exec_list_tables(&conn).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::ListDbtTables { dbt_path, tx } => {
                        let _ = tx.send(exec_list_dbt_tables(&dbt_path).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::ListSchemas { tx } => {
                        let _ = tx.send(exec_list_schemas(&conn).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::GetSchema { schema_name, table_name, tx } => {
                        let _ = tx.send(exec_get_schema(&conn, &schema_name, &table_name).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::GetDbtSchema { dbt_path, schema_name, table_name, tx } => {
                        let _ = tx.send(exec_get_dbt_schema(&dbt_path, &schema_name, &table_name).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::PreviewTable { table_name, limit, tx } => {
                        let sql = format!("SELECT * FROM {} LIMIT {}", sql_util::ident(&table_name), limit);
                        let _ = tx.send(exec_query(&conn, &sql).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::PreviewDbtTable { dbt_path, schema_name, table_name, limit, tx } => {
                        let _ = tx.send(exec_preview_dbt_table(&dbt_path, &schema_name, &table_name, limit).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::AttachDbt { dbt_path, tx } => {
                        let result = exec_attach_dbt(&conn, &dbt_path);
                        if result.is_ok() {
                            dbt_attached_path = Some(dbt_path);
                        }
                        let _ = tx.send(result.map_err(|e| e.to_string()));
                    }
                    WorkerCmd::DetachDbt { tx } => {
                        let result = exec_detach_dbt(&conn);
                        if result.is_ok() {
                            dbt_attached_path = None;
                        }
                        let _ = tx.send(result.map_err(|e| e.to_string()));
                    }
                    WorkerCmd::DropDbtTable { dbt_path, schema_name, table_name, tx } => {
                        let _ = tx.send(exec_drop_dbt_table(&dbt_path, &schema_name, &table_name).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::DropDbtSchema { dbt_path, schema_name, tx } => {
                        let _ = tx.send(exec_drop_dbt_schema(&dbt_path, &schema_name).map_err(|e| e.to_string()));
                    }

                    // === データインポート ===
                    WorkerCmd::PreviewCsv { opts, tx } => {
                        let _ = tx.send(exec_preview_csv(&conn, &opts).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::ImportCsv { opts, tx } => {
                        let _ = tx.send(exec_import_csv(&conn, &opts).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::ReimportCsv { schema_name, table_name, tx } => {
                        let _ = tx.send(exec_reimport_csv(&conn, &schema_name, &table_name).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::PreviewJson { opts, tx } => {
                        let _ = tx.send(exec_preview_json(&conn, &opts).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::ImportJson { opts, tx } => {
                        let _ = tx.send(exec_import_json(&conn, &opts).map_err(|e| e.to_string()));
                    }

                    // === メタデータ管理 ===
                    WorkerCmd::GetTableMeta { schema_name, table_name, tx } => {
                        let _ = tx.send(exec_get_table_meta(&conn, &schema_name, &table_name).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::GetDbtTableMeta { dbt_path, schema_name, table_name, tx } => {
                        let _ = tx.send(exec_get_dbt_table_meta(&dbt_path, &schema_name, &table_name).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::SetTableComment { schema_name, table_name, comment, tx } => {
                        let _ = tx.send(exec_set_table_comment(&conn, &schema_name, &table_name, &comment).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::SetColumnComment { schema_name, table_name, column_name, comment, tx } => {
                        let _ = tx.send(exec_set_column_comment(&conn, &schema_name, &table_name, &column_name, &comment).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::TouchTableTimestamp { schema_name, table_name, source, is_new, tx } => {
                        let _ = tx.send(exec_touch_table_timestamp(&conn, &schema_name, &table_name, &source, is_new).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::TouchDbtTimestamps { tables, tx } => {
                        let result = tables.iter().try_for_each(|(schema, table)| {
                            exec_touch_table_timestamp(&conn, schema, table, "dbt", false)
                        });
                        let _ = tx.send(result.map_err(|e| e.to_string()));
                    }

                    // === スケジューラ ===
                    WorkerCmd::ListScheduledJobs { tx } => {
                        let _ = tx.send(exec_list_scheduled_jobs(&conn).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::SaveScheduledJob { job, tx } => {
                        let _ = tx.send(exec_save_scheduled_job(&conn, &job).map_err(|e| e.to_string()));
                    }
                    WorkerCmd::DeleteScheduledJob { id, tx } => {
                        let _ = tx.send(exec_delete_scheduled_job(&conn, &id).map_err(|e| e.to_string()));
                    }
                }
            }
        });

        Self { tx }
    }

    pub async fn query(&self, sql: String) -> Result<QueryResult, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::Query { sql, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn list_tables(&self) -> Result<Vec<TableInfo>, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::ListTables { tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn list_dbt_tables(&self, dbt_path: String) -> Result<Vec<TableInfo>, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::ListDbtTables { dbt_path, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn attach_dbt(&self, dbt_path: String) -> Result<(), String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::AttachDbt { dbt_path, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn detach_dbt(&self) -> Result<(), String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::DetachDbt { tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn get_dbt_schema(&self, dbt_path: String, schema_name: String, table_name: String) -> Result<SchemaResult, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::GetDbtSchema { dbt_path, schema_name, table_name, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn drop_dbt_table(&self, dbt_path: String, schema_name: String, table_name: String) -> Result<(), String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::DropDbtTable { dbt_path, schema_name, table_name, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn drop_dbt_schema(&self, dbt_path: String, schema_name: String) -> Result<(), String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::DropDbtSchema { dbt_path, schema_name, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn preview_dbt_table(&self, dbt_path: String, schema_name: String, table_name: String, limit: i64) -> Result<QueryResult, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::PreviewDbtTable { dbt_path, schema_name, table_name, limit, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn get_schema(&self, schema_name: String, table_name: String) -> Result<SchemaResult, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::GetSchema { schema_name, table_name, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn list_schemas(&self) -> Result<Vec<String>, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::ListSchemas { tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn preview_table(&self, table_name: String, limit: i64) -> Result<QueryResult, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::PreviewTable { table_name, limit, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn preview_csv(&self, opts: CsvImportOptions) -> Result<CsvPreviewResult, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::PreviewCsv { opts, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn import_csv(&self, opts: CsvImportOptions) -> Result<TableInfo, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::ImportCsv { opts, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn get_table_meta(&self, schema_name: String, table_name: String) -> Result<TableMeta, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::GetTableMeta { schema_name, table_name, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn get_dbt_table_meta(&self, dbt_path: String, schema_name: String, table_name: String) -> Result<TableMeta, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::GetDbtTableMeta { dbt_path, schema_name, table_name, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn set_table_comment(&self, schema_name: String, table_name: String, comment: String) -> Result<(), String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::SetTableComment { schema_name, table_name, comment, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn set_column_comment(&self, schema_name: String, table_name: String, column_name: String, comment: String) -> Result<(), String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::SetColumnComment { schema_name, table_name, column_name, comment, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn reimport_csv(&self, schema_name: String, table_name: String) -> Result<TableInfo, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::ReimportCsv { schema_name, table_name, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn touch_table_timestamp(&self, schema_name: String, table_name: String, source: String, is_new: bool) -> Result<(), String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::TouchTableTimestamp { schema_name, table_name, source, is_new, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn touch_dbt_timestamps(&self, tables: Vec<(String, String)>) -> Result<(), String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::TouchDbtTimestamps { tables, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn preview_json(&self, opts: JsonImportOptions) -> Result<JsonPreviewResult, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::PreviewJson { opts, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn import_json(&self, opts: JsonImportOptions) -> Result<TableInfo, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::ImportJson { opts, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn list_scheduled_jobs(&self) -> Result<Vec<ScheduledJob>, String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::ListScheduledJobs { tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn save_scheduled_job(&self, job: ScheduledJob) -> Result<(), String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::SaveScheduledJob { job, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    pub async fn delete_scheduled_job(&self, id: String) -> Result<(), String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx.send(WorkerCmd::DeleteScheduledJob { id, tx: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }
}

const RESULT_ROW_LIMIT: usize = 1000;

fn exec_query(conn: &Connection, sql: &str) -> Result<QueryResult, AppError> {
    let start = Instant::now();
    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query([])?;

    let column_count = rows.as_ref().map(|s| s.column_count()).unwrap_or(0);
    let columns: Vec<String> = (0..column_count)
        .map(|i| {
            rows.as_ref()
                .and_then(|s| s.column_name(i).ok())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("col{}", i))
        })
        .collect();

    let mut result_rows: Vec<Vec<serde_json::Value>> = Vec::new();
    while let Some(row) = rows.next()? {
        let row_vals: Vec<serde_json::Value> =
            (0..column_count).map(|i| row_value_to_json(row, i)).collect();
        result_rows.push(row_vals);
        if result_rows.len() >= RESULT_ROW_LIMIT {
            // Drain remaining rows to check if there are more
            let truncated = rows.next()?.is_some();
            let row_count = result_rows.len();
            let elapsed_ms = start.elapsed().as_millis();
            return Ok(QueryResult { columns, rows: result_rows, row_count, elapsed_ms, truncated });
        }
    }

    let row_count = result_rows.len();
    let elapsed_ms = start.elapsed().as_millis();
    Ok(QueryResult { columns, rows: result_rows, row_count, elapsed_ms, truncated: false })
}

/// Shared inner loop for listing tables from a connection.
/// `list_sql` must SELECT schema_name, table_name.
/// `exclude_dbt_db`: when true, adds `AND database_name != 'dbt'` to the column count query
///   (needed for the main connection that may have the dbt DB attached).
fn list_tables_from(conn: &Connection, list_sql: &str, exclude_dbt_db: bool) -> Result<Vec<TableInfo>, AppError> {
    let mut stmt = conn.prepare(list_sql)?;
    let mut rows = stmt.query([])?;

    let mut tables = Vec::new();
    while let Some(row) = rows.next()? {
        let schema_name: String = row.get(0)?;
        let name: String = row.get(1)?;
        let table_type: String = row.get(2).unwrap_or_else(|_| "table".to_string());
        let row_count: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {}", sql_util::qualified(&schema_name, &name)),
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let db_filter = if exclude_dbt_db { " AND database_name != 'dbt'" } else { "" };
        let column_count: i64 = conn
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM duckdb_columns() WHERE{} schema_name = {} AND table_name = {}",
                    db_filter,
                    sql_util::literal(&schema_name),
                    sql_util::literal(&name)
                ),
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        tables.push(TableInfo { name, schema_name, row_count, column_count, csv_source_path: None, table_type });
    }
    Ok(tables)
}

fn exec_list_tables(conn: &Connection) -> Result<Vec<TableInfo>, AppError> {
    let list_sql = "\
        SELECT schema_name, table_name, 'table' AS table_type FROM duckdb_tables() \
        WHERE schema_name NOT IN ('information_schema', 'pg_catalog', '_tdw') AND database_name != 'dbt' \
        UNION ALL \
        SELECT schema_name, view_name, 'view' AS table_type FROM duckdb_views() \
        WHERE schema_name NOT IN ('information_schema', 'pg_catalog', '_tdw') AND database_name != 'dbt' AND internal = false \
        ORDER BY schema_name, table_name";
    let mut tables = list_tables_from(conn, list_sql, true)?;
    for t in &mut tables {
        if t.table_type == "table" {
            t.csv_source_path = conn
                .query_row(
                    "SELECT file_path FROM _tdw.csv_sources WHERE schema_name = ? AND table_name = ?",
                    [&t.schema_name, &t.name],
                    |r| r.get(0),
                )
                .ok();
        }
    }
    Ok(tables)
}

fn exec_list_dbt_tables(dbt_path: &str) -> Result<Vec<TableInfo>, AppError> {
    // Open a fresh read-only connection — released immediately when this function returns,
    // so dbt can write to the file at any other time without conflicts.
    let config = duckdb::Config::default()
        .access_mode(duckdb::AccessMode::ReadOnly)?;
    let conn = match Connection::open_with_flags(dbt_path, config) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };
    let list_sql = "SELECT schema_name, table_name, 'table' AS table_type FROM duckdb_tables() WHERE schema_name NOT IN ('information_schema', 'pg_catalog') ORDER BY schema_name, table_name";
    list_tables_from(&conn, list_sql, false)
}

/// Shared schema lookup used by both the main and dbt connections.
fn get_schema_from(conn: &Connection, schema_name: &str, table_name: &str) -> Result<SchemaResult, AppError> {
    let sql = format!(
        "SELECT column_name, data_type, is_nullable FROM duckdb_columns() WHERE schema_name = {} AND table_name = {} ORDER BY column_index",
        sql_util::literal(schema_name),
        sql_util::literal(table_name)
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;
    let mut columns = Vec::new();
    while let Some(row) = rows.next()? {
        let name: String = row.get(0)?;
        let column_type: String = row.get(1)?;
        // is_nullable can be BOOLEAN or VARCHAR depending on DuckDB version
        let nullable = row.get::<_, bool>(2)
            .unwrap_or_else(|_| row.get::<_, String>(2).map(|s| s == "YES").unwrap_or(false));
        columns.push(ColumnInfo { name, column_type, nullable });
    }
    Ok(SchemaResult { table_name: table_name.to_string(), columns })
}

fn exec_get_schema(conn: &Connection, schema_name: &str, table_name: &str) -> Result<SchemaResult, AppError> {
    get_schema_from(conn, schema_name, table_name)
}

fn exec_preview_csv(conn: &Connection, opts: &CsvImportOptions) -> Result<CsvPreviewResult, AppError> {
    let csv_expr = file_io::csv::build_read_expr(opts);
    let sql = format!("SELECT * FROM {} LIMIT 10", csv_expr);
    let preview = exec_query(conn, &sql)?;

    // Suggest table name from file stem
    let suggested_table_name = std::path::Path::new(&opts.file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported_table")
        .to_string();

    Ok(CsvPreviewResult { preview, suggested_table_name })
}

/// CSV/JSON インポートの共通パラメータ
struct ImportParams<'a> {
    schema_name: &'a str,
    table_name: &'a str,
    /// read_csv(...) または read_json(...) の SQL 式
    read_sql: &'a str,
    if_exists: &'a str,
    /// インポート後に TableInfo.csv_source_path として返すパス（CSV のみ Some）
    csv_source_path: Option<String>,
}

/// CSV/JSON インポート後処理の共通ヘルパー
///
/// テーブル作成 SQL の実行・タイムスタンプ記録・行数・列数取得を統一して行う。
/// CSV ソース情報の記録は呼び出し元で行う（CSV 固有処理のため）。
fn exec_import_common(conn: &Connection, params: ImportParams) -> Result<TableInfo, AppError> {
    let qualified = sql_util::qualified(params.schema_name, params.table_name);

    let sql = match params.if_exists {
        "append" => format!("INSERT INTO {} SELECT * FROM {}", qualified, params.read_sql),
        "replace" => format!("CREATE OR REPLACE TABLE {} AS SELECT * FROM {}", qualified, params.read_sql),
        _ => format!("CREATE TABLE {} AS SELECT * FROM {}", qualified, params.read_sql),
    };

    conn.execute(&sql, [])?;

    // タイムスタンプ記録（append 以外）
    if params.if_exists != "append" {
        let is_new = params.if_exists != "replace";
        let _ = exec_touch_table_timestamp(conn, params.schema_name, params.table_name, "adhoc", is_new);
    }

    let row_count: i64 = conn
        .query_row(&format!("SELECT COUNT(*) FROM {}", qualified), [], |r| r.get(0))
        .unwrap_or(0);
    let column_count: i64 = conn
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM duckdb_columns() WHERE schema_name = {} AND table_name = {}",
                sql_util::literal(params.schema_name),
                sql_util::literal(params.table_name)
            ),
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    Ok(TableInfo {
        name: params.table_name.to_string(),
        schema_name: params.schema_name.to_string(),
        row_count,
        column_count,
        csv_source_path: params.csv_source_path,
        table_type: "table".to_string(),
    })
}

fn exec_import_csv(conn: &Connection, opts: &CsvImportOptions) -> Result<TableInfo, AppError> {
    let csv_expr = file_io::csv::build_read_expr(opts);

    // Save CSV source info for re-import (not for append, where the source mapping is ambiguous)
    if opts.if_exists != "append" {
        let upsert = format!(
            "INSERT INTO _tdw.csv_sources (schema_name, table_name, file_path, delimiter, encoding, has_header)
             VALUES ({}, {}, {}, {}, {}, {})
             ON CONFLICT (schema_name, table_name) DO UPDATE SET
                 file_path = EXCLUDED.file_path,
                 delimiter = EXCLUDED.delimiter,
                 encoding  = EXCLUDED.encoding,
                 has_header = EXCLUDED.has_header",
            sql_util::literal(&opts.schema_name),
            sql_util::literal(&opts.table_name),
            sql_util::literal(&opts.file_path),
            sql_util::literal(&opts.delimiter),
            sql_util::literal(&opts.encoding),
            if opts.has_header { "true" } else { "false" },
        );
        let _ = conn.execute(&upsert, []);
    }

    exec_import_common(conn, ImportParams {
        schema_name: &opts.schema_name,
        table_name: &opts.table_name,
        read_sql: &csv_expr,
        if_exists: &opts.if_exists,
        csv_source_path: Some(opts.file_path.clone()),
    })
}

fn exec_preview_dbt_table(dbt_path: &str, schema_name: &str, table_name: &str, limit: i64) -> Result<QueryResult, AppError> {
    let config = duckdb::Config::default()
        .access_mode(duckdb::AccessMode::ReadOnly)?;
    let conn = Connection::open_with_flags(dbt_path, config)?;
    let sql = format!("SELECT * FROM {} LIMIT {}", sql_util::qualified(schema_name, table_name), limit);
    exec_query(&conn, &sql)
}

fn exec_list_schemas(conn: &Connection) -> Result<Vec<String>, AppError> {
    let mut stmt = conn
        .prepare("SELECT schema_name FROM duckdb_schemas() WHERE schema_name NOT IN ('information_schema', 'pg_catalog', '_tdw') AND database_name != 'dbt' ORDER BY schema_name")?;
    let mut rows = stmt.query([])?;
    let mut schemas = Vec::new();
    while let Some(row) = rows.next()? {
        let name: String = row.get(0)?;
        schemas.push(name);
    }
    Ok(schemas)
}

fn exec_attach_dbt(conn: &Connection, dbt_path: &str) -> Result<(), AppError> {
    // Detach first if already attached (ignore error if not attached)
    let _ = conn.execute("DETACH dbt", []);
    let sql = format!("ATTACH {} AS dbt (READ_ONLY)", sql_util::literal(dbt_path));
    conn.execute(&sql, [])?;
    Ok(())
}

fn exec_detach_dbt(conn: &Connection) -> Result<(), AppError> {
    conn.execute("DETACH dbt", [])?;
    Ok(())
}

fn exec_get_dbt_schema(dbt_path: &str, schema_name: &str, table_name: &str) -> Result<SchemaResult, AppError> {
    let config = duckdb::Config::default()
        .access_mode(duckdb::AccessMode::ReadOnly)?;
    let conn = Connection::open_with_flags(dbt_path, config)?;
    get_schema_from(&conn, schema_name, table_name)
}

fn exec_drop_dbt_table(dbt_path: &str, schema_name: &str, table_name: &str) -> Result<(), AppError> {
    let conn = Connection::open(dbt_path)?;
    let sql = format!("DROP TABLE IF EXISTS {}", sql_util::qualified(schema_name, table_name));
    conn.execute(&sql, [])?;
    Ok(())
}

fn exec_drop_dbt_schema(dbt_path: &str, schema_name: &str) -> Result<(), AppError> {
    let conn = Connection::open(dbt_path)?;
    let sql = format!("DROP SCHEMA IF EXISTS {} CASCADE", sql_util::ident(schema_name));
    conn.execute(&sql, [])?;
    Ok(())
}

fn exec_touch_table_timestamp(conn: &Connection, schema_name: &str, table_name: &str, source: &str, is_new: bool) -> Result<(), AppError> {
    let now_sql = "strftime(now()::TIMESTAMP, '%Y-%m-%dT%H:%M:%SZ')";
    let _ = is_new; // both branches produce identical SQL; field reserved for future use
    let sql = format!(
        "INSERT INTO _tdw.table_timestamps (schema_name, table_name, source, created_at, updated_at) \
         VALUES ({}, {}, {}, {}, {}) \
         ON CONFLICT (schema_name, table_name, source) DO UPDATE SET updated_at = {}",
        sql_util::literal(schema_name), sql_util::literal(table_name), sql_util::literal(source),
        now_sql, now_sql, now_sql
    );
    conn.execute(&sql, [])?;
    Ok(())
}

fn fetch_table_meta(
    conn: &Connection,
    schema_name: &str,
    table_name: &str,
    exclude_db: Option<&str>,
) -> Result<TableMeta, AppError> {
    let db_filter = exclude_db
        .map(|db| format!(" AND database_name != {}", sql_util::literal(db)))
        .unwrap_or_default();

    let comment: Option<String> = conn.query_row(
        &format!(
            "SELECT comment FROM duckdb_tables() WHERE schema_name = {} AND table_name = {}{}",
            sql_util::literal(schema_name),
            sql_util::literal(table_name),
            db_filter
        ),
        [],
        |r| r.get::<_, String>(0),
    ).ok();

    let sql = format!(
        "SELECT column_name, data_type, comment FROM duckdb_columns() WHERE schema_name = {} AND table_name = {}{} ORDER BY column_index",
        sql_util::literal(schema_name),
        sql_util::literal(table_name),
        db_filter
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;
    let mut columns = Vec::new();
    while let Some(row) = rows.next()? {
        let name: String = row.get(0)?;
        let data_type: String = row.get(1)?;
        let col_comment: Option<String> = row.get::<_, String>(2).ok();
        columns.push(ColumnMeta { name, data_type, comment: col_comment });
    }

    let ts_sql = format!(
        "SELECT created_at, updated_at FROM _tdw.table_timestamps WHERE schema_name = {} AND table_name = {} AND source = 'adhoc'",
        sql_util::literal(schema_name),
        sql_util::literal(table_name)
    );
    let (created_at, updated_at) = conn.query_row(&ts_sql, [], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    }).map(|(c, u)| (Some(c), Some(u))).unwrap_or((None, None));

    Ok(TableMeta { schema_name: schema_name.to_string(), table_name: table_name.to_string(), comment, columns, created_at, updated_at })
}

fn exec_get_table_meta(conn: &Connection, schema_name: &str, table_name: &str) -> Result<TableMeta, AppError> {
    fetch_table_meta(conn, schema_name, table_name, Some("dbt"))
}

fn exec_get_dbt_table_meta(dbt_path: &str, schema_name: &str, table_name: &str) -> Result<TableMeta, AppError> {
    let config = duckdb::Config::default()
        .access_mode(duckdb::AccessMode::ReadOnly)?;
    let conn = Connection::open_with_flags(dbt_path, config)?;
    fetch_table_meta(&conn, schema_name, table_name, None)
}

fn exec_set_table_comment(conn: &Connection, schema_name: &str, table_name: &str, comment: &str) -> Result<(), AppError> {
    let qualified = sql_util::qualified(schema_name, table_name);
    let sql = if comment.is_empty() {
        format!("COMMENT ON TABLE {} IS NULL", qualified)
    } else {
        format!("COMMENT ON TABLE {} IS {}", qualified, sql_util::literal(comment))
    };
    conn.execute(&sql, [])?;
    Ok(())
}

fn exec_set_column_comment(conn: &Connection, schema_name: &str, table_name: &str, column_name: &str, comment: &str) -> Result<(), AppError> {
    let table_ref = sql_util::qualified(schema_name, table_name);
    let col_ref = sql_util::ident(column_name);
    let sql = if comment.is_empty() {
        format!("COMMENT ON COLUMN {}.{} IS NULL", table_ref, col_ref)
    } else {
        format!("COMMENT ON COLUMN {}.{} IS {}", table_ref, col_ref, sql_util::literal(comment))
    };
    conn.execute(&sql, [])?;
    Ok(())
}

fn exec_reimport_csv(conn: &Connection, schema_name: &str, table_name: &str) -> Result<TableInfo, AppError> {
    let result: Result<(String, String, String, bool), _> = conn.query_row(
        "SELECT file_path, delimiter, encoding, has_header FROM _tdw.csv_sources WHERE schema_name = ? AND table_name = ?",
        [schema_name, table_name],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    );
    let (file_path, delimiter, encoding, has_header) = result
        .map_err(|_| AppError::Other(format!("CSV source not found for {}.{}", schema_name, table_name)))?;

    let opts = CsvImportOptions {
        file_path,
        table_name: table_name.to_string(),
        schema_name: schema_name.to_string(),
        has_header,
        delimiter,
        encoding,
        if_exists: "replace".to_string(),
    };
    exec_import_csv(conn, &opts)
}

fn exec_preview_json(conn: &Connection, opts: &JsonImportOptions) -> Result<JsonPreviewResult, AppError> {
    let json_expr = file_io::json::build_read_expr(opts);
    let sql = format!("SELECT * FROM {} LIMIT 10", json_expr);
    let preview = exec_query(conn, &sql)?;
    let suggested_table_name = std::path::Path::new(&opts.file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported_table")
        .to_string();
    Ok(JsonPreviewResult { preview, suggested_table_name })
}

fn exec_import_json(conn: &Connection, opts: &JsonImportOptions) -> Result<TableInfo, AppError> {
    let json_expr = file_io::json::build_read_expr(opts);

    exec_import_common(conn, ImportParams {
        schema_name: &opts.schema_name,
        table_name: &opts.table_name,
        read_sql: &json_expr,
        if_exists: &opts.if_exists,
        csv_source_path: None,
    })
}

fn exec_list_scheduled_jobs(conn: &Connection) -> Result<Vec<ScheduledJob>, AppError> {
    let sql = "SELECT id, name, job_type, target_id, cron_expr, enabled, \
               CAST(created_at AS VARCHAR), CAST(last_run_at AS VARCHAR) \
               FROM _tdw.scheduled_jobs ORDER BY created_at";
    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query([])?;
    let mut jobs = Vec::new();
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let job_type_str: String = row.get(2)?;
        let target_id: String = row.get(3)?;
        let cron_expr: String = row.get(4)?;
        let enabled: bool = row.get(5)?;
        let created_at: String = row.get(6)?;
        let last_run_at: Option<String> = row.get(7)?;
        let job_type = job_type_str.parse::<JobType>()
            .map_err(AppError::Other)?;
        jobs.push(ScheduledJob { id, name, job_type, target_id, cron_expr, enabled, created_at, last_run_at });
    }
    Ok(jobs)
}

fn exec_save_scheduled_job(conn: &Connection, job: &ScheduledJob) -> Result<(), AppError> {
    let sql = "INSERT OR REPLACE INTO _tdw.scheduled_jobs \
               (id, name, job_type, target_id, cron_expr, enabled, created_at, last_run_at) \
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)";
    conn.execute(sql, duckdb::params![
        job.id,
        job.name,
        job.job_type.as_str(),
        job.target_id,
        job.cron_expr,
        job.enabled,
        job.created_at,
        job.last_run_at,
    ])?;
    Ok(())
}

fn exec_delete_scheduled_job(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM _tdw.scheduled_jobs WHERE id = ?", duckdb::params![id])?;
    Ok(())
}
