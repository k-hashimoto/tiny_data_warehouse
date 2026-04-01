use std::sync::mpsc;
use std::thread;
use std::time::Instant;
use duckdb::Connection;
use crate::db::connection::row_value_to_json;
use crate::db::sql_util;
use crate::db::types::{QueryResult, TableInfo, SchemaResult, ColumnInfo, CsvImportOptions, CsvPreviewResult, JsonImportOptions, JsonPreviewResult, TableMeta, ColumnMeta};
use crate::file_io;

pub enum WorkerCmd {
    Query {
        sql: String,
        tx: tokio::sync::oneshot::Sender<Result<QueryResult, String>>,
    },
    ListTables {
        tx: tokio::sync::oneshot::Sender<Result<Vec<TableInfo>, String>>,
    },
    ListDbtTables {
        dbt_path: String,
        tx: tokio::sync::oneshot::Sender<Result<Vec<TableInfo>, String>>,
    },
    PreviewDbtTable {
        dbt_path: String,
        schema_name: String,
        table_name: String,
        limit: i64,
        tx: tokio::sync::oneshot::Sender<Result<QueryResult, String>>,
    },
    GetSchema {
        schema_name: String,
        table_name: String,
        tx: tokio::sync::oneshot::Sender<Result<SchemaResult, String>>,
    },
    PreviewTable {
        table_name: String,
        limit: i64,
        tx: tokio::sync::oneshot::Sender<Result<QueryResult, String>>,
    },
    PreviewCsv {
        opts: CsvImportOptions,
        tx: tokio::sync::oneshot::Sender<Result<CsvPreviewResult, String>>,
    },
    ImportCsv {
        opts: CsvImportOptions,
        tx: tokio::sync::oneshot::Sender<Result<TableInfo, String>>,
    },
    ListSchemas {
        tx: tokio::sync::oneshot::Sender<Result<Vec<String>, String>>,
    },
    AttachDbt {
        dbt_path: String,
        tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    DetachDbt {
        tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    GetDbtSchema {
        dbt_path: String,
        schema_name: String,
        table_name: String,
        tx: tokio::sync::oneshot::Sender<Result<SchemaResult, String>>,
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
            let conn = Connection::open(&db_path)
                .expect("Failed to open DuckDB connection");
            // Limit DuckDB internal threads to avoid macOS thread conflicts
            let _ = conn.execute("SET threads=2", []);
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
                 );"
            );

            for cmd in rx {
                match cmd {
                    WorkerCmd::Query { sql, tx } => {
                        let _ = tx.send(exec_query(&conn, &sql));
                    }
                    WorkerCmd::ListTables { tx } => {
                        let _ = tx.send(exec_list_tables(&conn));
                    }
                    WorkerCmd::ListDbtTables { dbt_path, tx } => {
                        let _ = tx.send(exec_list_dbt_tables(&dbt_path));
                    }
                    WorkerCmd::PreviewDbtTable { dbt_path, schema_name, table_name, limit, tx } => {
                        let _ = tx.send(exec_preview_dbt_table(&dbt_path, &schema_name, &table_name, limit));
                    }
                    WorkerCmd::GetSchema { schema_name, table_name, tx } => {
                        let _ = tx.send(exec_get_schema(&conn, &schema_name, &table_name));
                    }
                    WorkerCmd::PreviewTable { table_name, limit, tx } => {
                        let sql = format!("SELECT * FROM {} LIMIT {}", sql_util::ident(&table_name), limit);
                        let _ = tx.send(exec_query(&conn, &sql));
                    }
                    WorkerCmd::PreviewCsv { opts, tx } => {
                        let _ = tx.send(exec_preview_csv(&conn, &opts));
                    }
                    WorkerCmd::ImportCsv { opts, tx } => {
                        let _ = tx.send(exec_import_csv(&conn, &opts));
                    }
                    WorkerCmd::ListSchemas { tx } => {
                        let _ = tx.send(exec_list_schemas(&conn));
                    }
                    WorkerCmd::AttachDbt { dbt_path, tx } => {
                        let _ = tx.send(exec_attach_dbt(&conn, &dbt_path));
                    }
                    WorkerCmd::DetachDbt { tx } => {
                        let _ = tx.send(exec_detach_dbt(&conn));
                    }
                    WorkerCmd::GetDbtSchema { dbt_path, schema_name, table_name, tx } => {
                        let _ = tx.send(exec_get_dbt_schema(&dbt_path, &schema_name, &table_name));
                    }
                    WorkerCmd::DropDbtTable { dbt_path, schema_name, table_name, tx } => {
                        let _ = tx.send(exec_drop_dbt_table(&dbt_path, &schema_name, &table_name));
                    }
                    WorkerCmd::DropDbtSchema { dbt_path, schema_name, tx } => {
                        let _ = tx.send(exec_drop_dbt_schema(&dbt_path, &schema_name));
                    }
                    WorkerCmd::GetTableMeta { schema_name, table_name, tx } => {
                        let _ = tx.send(exec_get_table_meta(&conn, &schema_name, &table_name));
                    }
                    WorkerCmd::GetDbtTableMeta { dbt_path, schema_name, table_name, tx } => {
                        let _ = tx.send(exec_get_dbt_table_meta(&dbt_path, &schema_name, &table_name));
                    }
                    WorkerCmd::SetTableComment { schema_name, table_name, comment, tx } => {
                        let _ = tx.send(exec_set_table_comment(&conn, &schema_name, &table_name, &comment));
                    }
                    WorkerCmd::SetColumnComment { schema_name, table_name, column_name, comment, tx } => {
                        let _ = tx.send(exec_set_column_comment(&conn, &schema_name, &table_name, &column_name, &comment));
                    }
                    WorkerCmd::ReimportCsv { schema_name, table_name, tx } => {
                        let _ = tx.send(exec_reimport_csv(&conn, &schema_name, &table_name));
                    }
                    WorkerCmd::TouchTableTimestamp { schema_name, table_name, source, is_new, tx } => {
                        let _ = tx.send(exec_touch_table_timestamp(&conn, &schema_name, &table_name, &source, is_new));
                    }
                    WorkerCmd::TouchDbtTimestamps { tables, tx } => {
                        let result = tables.iter().try_for_each(|(schema, table)| {
                            exec_touch_table_timestamp(&conn, schema, table, "dbt", false)
                        });
                        let _ = tx.send(result);
                    }
                    WorkerCmd::PreviewJson { opts, tx } => {
                        let _ = tx.send(exec_preview_json(&conn, &opts));
                    }
                    WorkerCmd::ImportJson { opts, tx } => {
                        let _ = tx.send(exec_import_json(&conn, &opts));
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
}

const RESULT_ROW_LIMIT: usize = 1000;

fn exec_query(conn: &Connection, sql: &str) -> Result<QueryResult, String> {
    let start = Instant::now();
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;

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
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let row_vals: Vec<serde_json::Value> =
            (0..column_count).map(|i| row_value_to_json(row, i)).collect();
        result_rows.push(row_vals);
        if result_rows.len() >= RESULT_ROW_LIMIT {
            // Drain remaining rows to check if there are more
            let truncated = rows.next().map_err(|e| e.to_string())?.is_some();
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
fn list_tables_from(conn: &Connection, list_sql: &str, exclude_dbt_db: bool) -> Result<Vec<TableInfo>, String> {
    let mut stmt = conn.prepare(list_sql).map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;

    let mut tables = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let schema_name: String = row.get(0).map_err(|e| e.to_string())?;
        let name: String = row.get(1).map_err(|e| e.to_string())?;
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

fn exec_list_tables(conn: &Connection) -> Result<Vec<TableInfo>, String> {
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

fn exec_list_dbt_tables(dbt_path: &str) -> Result<Vec<TableInfo>, String> {
    // Open a fresh read-only connection — released immediately when this function returns,
    // so dbt can write to the file at any other time without conflicts.
    let config = duckdb::Config::default()
        .access_mode(duckdb::AccessMode::ReadOnly)
        .map_err(|e| e.to_string())?;
    let conn = match Connection::open_with_flags(dbt_path, config) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };
    let list_sql = "SELECT schema_name, table_name, 'table' AS table_type FROM duckdb_tables() WHERE schema_name NOT IN ('information_schema', 'pg_catalog') ORDER BY schema_name, table_name";
    list_tables_from(&conn, list_sql, false)
}

/// Shared schema lookup used by both the main and dbt connections.
fn get_schema_from(conn: &Connection, schema_name: &str, table_name: &str) -> Result<SchemaResult, String> {
    let sql = format!(
        "SELECT column_name, data_type, is_nullable FROM duckdb_columns() WHERE schema_name = {} AND table_name = {} ORDER BY column_index",
        sql_util::literal(schema_name),
        sql_util::literal(table_name)
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let mut columns = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let name: String = row.get(0).map_err(|e| e.to_string())?;
        let column_type: String = row.get(1).map_err(|e| e.to_string())?;
        // is_nullable can be BOOLEAN or VARCHAR depending on DuckDB version
        let nullable = row.get::<_, bool>(2)
            .unwrap_or_else(|_| row.get::<_, String>(2).map(|s| s == "YES").unwrap_or(false));
        columns.push(ColumnInfo { name, column_type, nullable });
    }
    Ok(SchemaResult { table_name: table_name.to_string(), columns })
}

fn exec_get_schema(conn: &Connection, schema_name: &str, table_name: &str) -> Result<SchemaResult, String> {
    get_schema_from(conn, schema_name, table_name)
}

fn exec_preview_csv(conn: &Connection, opts: &CsvImportOptions) -> Result<CsvPreviewResult, String> {
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

fn exec_import_csv(conn: &Connection, opts: &CsvImportOptions) -> Result<TableInfo, String> {
    let csv_expr = file_io::csv::build_read_expr(opts);
    let qualified = sql_util::qualified(&opts.schema_name, &opts.table_name);

    let sql = match opts.if_exists.as_str() {
        "append" => format!("INSERT INTO {} SELECT * FROM {}", qualified, csv_expr),
        "replace" => format!("CREATE OR REPLACE TABLE {} AS SELECT * FROM {}", qualified, csv_expr),
        _ => format!("CREATE TABLE {} AS SELECT * FROM {}", qualified, csv_expr),
    };

    conn.execute(&sql, []).map_err(|e| e.to_string())?;

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
        // CSV import時にタイムスタンプを記録（replace = 新規扱いで created_at をリセット）
        let is_new = opts.if_exists != "replace";
        let _ = exec_touch_table_timestamp(conn, &opts.schema_name, &opts.table_name, "adhoc", is_new);
    }

    let row_count: i64 = conn
        .query_row(&format!("SELECT COUNT(*) FROM {}", qualified), [], |r| r.get(0))
        .unwrap_or(0);
    let column_count: i64 = conn
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM duckdb_columns() WHERE schema_name = {} AND table_name = {}",
                sql_util::literal(&opts.schema_name),
                sql_util::literal(&opts.table_name)
            ),
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    Ok(TableInfo {
        name: opts.table_name.clone(),
        schema_name: opts.schema_name.clone(),
        row_count,
        column_count,
        csv_source_path: Some(opts.file_path.clone()),
        table_type: "table".to_string(),
    })
}

fn exec_preview_dbt_table(dbt_path: &str, schema_name: &str, table_name: &str, limit: i64) -> Result<QueryResult, String> {
    let config = duckdb::Config::default()
        .access_mode(duckdb::AccessMode::ReadOnly)
        .map_err(|e| e.to_string())?;
    let conn = Connection::open_with_flags(dbt_path, config).map_err(|e| e.to_string())?;
    let sql = format!("SELECT * FROM {} LIMIT {}", sql_util::qualified(schema_name, table_name), limit);
    exec_query(&conn, &sql)
}

fn exec_list_schemas(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT schema_name FROM duckdb_schemas() WHERE schema_name NOT IN ('information_schema', 'pg_catalog', '_tdw') AND database_name != 'dbt' ORDER BY schema_name")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let mut schemas = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let name: String = row.get(0).map_err(|e| e.to_string())?;
        schemas.push(name);
    }
    Ok(schemas)
}

fn exec_attach_dbt(conn: &Connection, dbt_path: &str) -> Result<(), String> {
    // Detach first if already attached (ignore error if not attached)
    let _ = conn.execute("DETACH dbt", []);
    let sql = format!("ATTACH {} AS dbt (READ_ONLY)", sql_util::literal(dbt_path));
    conn.execute(&sql, []).map_err(|e| e.to_string())?;
    Ok(())
}

fn exec_detach_dbt(conn: &Connection) -> Result<(), String> {
    conn.execute("DETACH dbt", []).map_err(|e| e.to_string())?;
    Ok(())
}

fn exec_get_dbt_schema(dbt_path: &str, schema_name: &str, table_name: &str) -> Result<SchemaResult, String> {
    let config = duckdb::Config::default()
        .access_mode(duckdb::AccessMode::ReadOnly)
        .map_err(|e| e.to_string())?;
    let conn = Connection::open_with_flags(dbt_path, config).map_err(|e| e.to_string())?;
    get_schema_from(&conn, schema_name, table_name)
}

fn exec_drop_dbt_table(dbt_path: &str, schema_name: &str, table_name: &str) -> Result<(), String> {
    let conn = Connection::open(dbt_path).map_err(|e| e.to_string())?;
    let sql = format!("DROP TABLE IF EXISTS {}", sql_util::qualified(schema_name, table_name));
    conn.execute(&sql, []).map_err(|e| e.to_string())?;
    Ok(())
}

fn exec_drop_dbt_schema(dbt_path: &str, schema_name: &str) -> Result<(), String> {
    let conn = Connection::open(dbt_path).map_err(|e| e.to_string())?;
    let sql = format!("DROP SCHEMA IF EXISTS {} CASCADE", sql_util::ident(schema_name));
    conn.execute(&sql, []).map_err(|e| e.to_string())?;
    Ok(())
}

fn exec_touch_table_timestamp(conn: &Connection, schema_name: &str, table_name: &str, source: &str, is_new: bool) -> Result<(), String> {
    let now_sql = "strftime(now(), '%Y-%m-%dT%H:%M:%SZ')";
    let sql = if is_new {
        format!(
            "INSERT INTO _tdw.table_timestamps (schema_name, table_name, source, created_at, updated_at) \
             VALUES ({}, {}, {}, {}, {}) \
             ON CONFLICT (schema_name, table_name, source) DO UPDATE SET updated_at = {}",
            sql_util::literal(schema_name), sql_util::literal(table_name), sql_util::literal(source),
            now_sql, now_sql, now_sql
        )
    } else {
        format!(
            "INSERT INTO _tdw.table_timestamps (schema_name, table_name, source, created_at, updated_at) \
             VALUES ({}, {}, {}, {}, {}) \
             ON CONFLICT (schema_name, table_name, source) DO UPDATE SET updated_at = {}",
            sql_util::literal(schema_name), sql_util::literal(table_name), sql_util::literal(source),
            now_sql, now_sql, now_sql
        )
    };
    conn.execute(&sql, []).map_err(|e| e.to_string())?;
    Ok(())
}

fn exec_get_table_meta(conn: &Connection, schema_name: &str, table_name: &str) -> Result<TableMeta, String> {
    let comment: Option<String> = conn.query_row(
        &format!(
            "SELECT comment FROM duckdb_tables() WHERE database_name != 'dbt' AND schema_name = {} AND table_name = {}",
            sql_util::literal(schema_name),
            sql_util::literal(table_name)
        ),
        [],
        |r| r.get::<_, String>(0),
    ).ok();

    let sql = format!(
        "SELECT column_name, data_type, comment FROM duckdb_columns() WHERE database_name != 'dbt' AND schema_name = {} AND table_name = {} ORDER BY column_index",
        sql_util::literal(schema_name),
        sql_util::literal(table_name)
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let mut columns = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let name: String = row.get(0).map_err(|e| e.to_string())?;
        let data_type: String = row.get(1).map_err(|e| e.to_string())?;
        let col_comment: Option<String> = row.get::<_, String>(2).ok();
        columns.push(ColumnMeta { name, data_type, comment: col_comment });
    }

    // タイムスタンプを取得
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

fn exec_get_dbt_table_meta(dbt_path: &str, schema_name: &str, table_name: &str) -> Result<TableMeta, String> {
    let config = duckdb::Config::default()
        .access_mode(duckdb::AccessMode::ReadOnly)
        .map_err(|e| e.to_string())?;
    let conn = Connection::open_with_flags(dbt_path, config).map_err(|e| e.to_string())?;

    let comment: Option<String> = conn.query_row(
        &format!(
            "SELECT comment FROM duckdb_tables() WHERE schema_name = {} AND table_name = {}",
            sql_util::literal(schema_name),
            sql_util::literal(table_name)
        ),
        [],
        |r| r.get::<_, String>(0),
    ).ok();

    let sql = format!(
        "SELECT column_name, data_type, comment FROM duckdb_columns() WHERE schema_name = {} AND table_name = {} ORDER BY column_index",
        sql_util::literal(schema_name),
        sql_util::literal(table_name)
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let mut columns = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let name: String = row.get(0).map_err(|e| e.to_string())?;
        let data_type: String = row.get(1).map_err(|e| e.to_string())?;
        let col_comment: Option<String> = row.get::<_, String>(2).ok();
        columns.push(ColumnMeta { name, data_type, comment: col_comment });
    }
    Ok(TableMeta { schema_name: schema_name.to_string(), table_name: table_name.to_string(), comment, columns, created_at: None, updated_at: None })
}

fn exec_set_table_comment(conn: &Connection, schema_name: &str, table_name: &str, comment: &str) -> Result<(), String> {
    let qualified = sql_util::qualified(schema_name, table_name);
    let sql = if comment.is_empty() {
        format!("COMMENT ON TABLE {} IS NULL", qualified)
    } else {
        format!("COMMENT ON TABLE {} IS {}", qualified, sql_util::literal(comment))
    };
    conn.execute(&sql, []).map_err(|e| e.to_string())?;
    Ok(())
}

fn exec_set_column_comment(conn: &Connection, schema_name: &str, table_name: &str, column_name: &str, comment: &str) -> Result<(), String> {
    let table_ref = sql_util::qualified(schema_name, table_name);
    let col_ref = sql_util::ident(column_name);
    let sql = if comment.is_empty() {
        format!("COMMENT ON COLUMN {}.{} IS NULL", table_ref, col_ref)
    } else {
        format!("COMMENT ON COLUMN {}.{} IS {}", table_ref, col_ref, sql_util::literal(comment))
    };
    conn.execute(&sql, []).map_err(|e| e.to_string())?;
    Ok(())
}

fn exec_reimport_csv(conn: &Connection, schema_name: &str, table_name: &str) -> Result<TableInfo, String> {
    let result: Result<(String, String, String, bool), _> = conn.query_row(
        "SELECT file_path, delimiter, encoding, has_header FROM _tdw.csv_sources WHERE schema_name = ? AND table_name = ?",
        [schema_name, table_name],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    );
    let (file_path, delimiter, encoding, has_header) = result
        .map_err(|_| format!("CSV source not found for {}.{}", schema_name, table_name))?;

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

fn exec_preview_json(conn: &Connection, opts: &JsonImportOptions) -> Result<JsonPreviewResult, String> {
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

fn exec_import_json(conn: &Connection, opts: &JsonImportOptions) -> Result<TableInfo, String> {
    let json_expr = file_io::json::build_read_expr(opts);
    let qualified = sql_util::qualified(&opts.schema_name, &opts.table_name);

    let sql = match opts.if_exists.as_str() {
        "append" => format!("INSERT INTO {} SELECT * FROM {}", qualified, json_expr),
        "replace" => format!("CREATE OR REPLACE TABLE {} AS SELECT * FROM {}", qualified, json_expr),
        _ => format!("CREATE TABLE {} AS SELECT * FROM {}", qualified, json_expr),
    };

    conn.execute(&sql, []).map_err(|e| e.to_string())?;
    let is_new = opts.if_exists != "replace";
    let _ = exec_touch_table_timestamp(conn, &opts.schema_name, &opts.table_name, "adhoc", is_new);

    let row_count: i64 = conn
        .query_row(&format!("SELECT COUNT(*) FROM {}", qualified), [], |r| r.get(0))
        .unwrap_or(0);
    let column_count: i64 = conn
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM duckdb_columns() WHERE schema_name = {} AND table_name = {}",
                sql_util::literal(&opts.schema_name),
                sql_util::literal(&opts.table_name)
            ),
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    Ok(TableInfo {
        name: opts.table_name.clone(),
        schema_name: opts.schema_name.clone(),
        row_count,
        column_count,
        csv_source_path: None,
        table_type: "table".to_string(),
    })
}
