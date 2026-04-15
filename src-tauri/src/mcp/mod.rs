use axum::{
    extract::State as AxumState,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;

use crate::db::worker::DbWorker;
use crate::utils;
use crate::cron_engine::read_log_entries;
use crate::commands::scripts::validate_name;
use crate::scheduler::{JobType, ScheduledJob};
use uuid::Uuid;

/// Shared lock: true while an MCP tool call is in progress.
/// Tauri commands check this and reject requests when true.
pub struct McpLock(pub Arc<AtomicBool>);

pub const MCP_PORT: u16 = 7741;

/// Handle for stopping the MCP server gracefully.
/// Wraps an optional oneshot sender; calling stop() consumes it.
pub struct McpServerHandle(pub tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>);

impl McpServerHandle {
    pub fn empty() -> Self {
        Self(tokio::sync::Mutex::new(None))
    }

    /// Send shutdown signal if the server is running.
    pub async fn stop(&self) {
        let mut guard = self.0.lock().await;
        if let Some(tx) = guard.take() {
            let _ = tx.send(());
        }
    }

    /// Returns true if the server is currently running.
    pub async fn is_running(&self) -> bool {
        self.0.lock().await.is_some()
    }
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct McpServerState {
    db: DbWorker,
    active: Arc<AtomicBool>,
    app_handle: tauri::AppHandle,
    log_path: std::path::PathBuf,
    home_dir: String,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Start the MCP server and register a shutdown handle.
/// Calling `handle.stop()` will gracefully shut it down.
pub async fn run_mcp_server(
    db: DbWorker,
    active: Arc<AtomicBool>,
    home_dir: String,
    app_handle: tauri::AppHandle,
    handle: Arc<McpServerHandle>,
) {
    let log_path = std::path::PathBuf::from(&home_dir)
        .join(".tdwh")
        .join("logs")
        .join("mcp_access.log");
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    write_log(
        &log_path,
        &format!("MCP server started (Streamable HTTP on port {MCP_PORT})"),
    );

    let state = McpServerState {
        db,
        active,
        app_handle: app_handle.clone(),
        log_path: log_path.clone(),
        home_dir: home_dir.clone(),
    };

    let router = Router::new()
        // Streamable HTTP: single POST endpoint for all JSON-RPC traffic
        .route("/mcp", post(mcp_handler))
        // Health check
        .route("/mcp", get(health_handler))
        .with_state(state);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], MCP_PORT));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            let msg = format!("port {MCP_PORT} bind failed: {e}");
            write_log(&log_path, &format!("ERROR {msg}"));
            let _ = app_handle.emit("mcp-server-error", msg);
            return;
        }
    };

    // Register shutdown channel
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    {
        let mut guard = handle.0.lock().await;
        *guard = Some(shutdown_tx);
    }

    write_log(
        &log_path,
        &format!("MCP server ready on http://localhost:{MCP_PORT}/mcp"),
    );
    let _ = app_handle.emit("mcp-server-ready", MCP_PORT);

    let result = axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            shutdown_rx.await.ok();
        })
        .await;

    write_log(&log_path, "MCP server stopped");
    let _ = app_handle.emit("mcp-server-stopped", ());

    if let Err(e) = result {
        let msg = format!("server error: {e}");
        write_log(&log_path, &format!("ERROR {msg}"));
        let _ = app_handle.emit("mcp-server-error", msg);
    }
}

// ---------------------------------------------------------------------------
// GET /mcp  — health check
// ---------------------------------------------------------------------------

async fn health_handler() -> impl IntoResponse {
    Json(json!({ "status": "ok", "server": "tiny-data-warehouse", "transport": "streamable-http" }))
}

// ---------------------------------------------------------------------------
// POST /mcp  — Streamable HTTP MCP endpoint (2025-03-26 spec)
// ---------------------------------------------------------------------------

async fn mcp_handler(
    AxumState(state): AxumState<McpServerState>,
    _headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let req: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": "Parse error" }
                })),
            );
        }
    };

    let response =
        handle_request(&req, &state.db, &state.active, &state.app_handle, &state.log_path, &state.home_dir).await;

    match response {
        Some(resp) => (StatusCode::OK, Json(resp)),
        // Notifications (no id) → 202 Accepted with empty body, but axum needs a response
        None => (StatusCode::ACCEPTED, Json(json!({}))),
    }
}

// ---------------------------------------------------------------------------
// JSON-RPC request handler
// ---------------------------------------------------------------------------

async fn handle_request(
    req: &Value,
    db: &DbWorker,
    active: &Arc<AtomicBool>,
    app_handle: &tauri::AppHandle,
    log_path: &std::path::Path,
    home_dir: &str,
) -> Option<Value> {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method")?.as_str()?;

    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2025-03-26",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "tiny-data-warehouse",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        })),

        "notifications/initialized" => None,

        "ping" => Some(json!({ "jsonrpc": "2.0", "id": id, "result": {} })),

        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    {
                        "name": "echo",
                        "description": "MCPサーバーの疎通確認用。受け取ったメッセージをそのまま返す。",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "message": { "type": "string", "description": "返してほしいメッセージ" }
                            },
                            "required": ["message"]
                        }
                    },
                    {
                        "name": "run_query",
                        "description": "Execute a SQL query on DuckDB and return results as JSON.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sql": { "type": "string", "description": "SQL query to execute" }
                            },
                            "required": ["sql"]
                        }
                    },
                    {
                        "name": "list_tables",
                        "description": "List all tables in the database with row/column counts. Set include_dbt=true to also include dbt tables.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "include_dbt": { "type": "boolean", "description": "If true, also include dbt tables (default: false)" }
                            }
                        }
                    },
                    {
                        "name": "get_schema",
                        "description": "Get column definitions of a table. Use database=\"dbt\" to get schema from dbt tables.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "schema_name": { "type": "string" },
                                "table_name":  { "type": "string" },
                                "database":    { "type": "string", "description": "\"main\" (default) or \"dbt\"" }
                            },
                            "required": ["schema_name", "table_name"]
                        }
                    },
                    {
                        "name": "reimport_csv",
                        "description": "Re-import a table from its original CSV source file.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "schema_name": { "type": "string" },
                                "table_name":  { "type": "string" }
                            },
                            "required": ["schema_name", "table_name"]
                        }
                    },
                    {
                        "name": "export_query_csv",
                        "description": "Export SQL query results to a CSV file. Returns the saved file path.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sql":        { "type": "string", "description": "SQL query to export" },
                                "export_dir": { "type": "string", "description": "Directory to save (supports ~/)" },
                                "filename":   { "type": "string", "description": "Output filename e.g. result.csv" }
                            },
                            "required": ["sql", "export_dir", "filename"]
                        }
                    },
                    {
                        "name": "list_saved_queries",
                        "description": "~/.tdwh/scripts/ 以下の .sql ファイル一覧をスクリプト名（拡張子なし）の配列で返す。",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "get_saved_query",
                        "description": "指定したスクリプト名の SQL テキストを返す。",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string", "description": "スクリプト名（拡張子なし、サブディレクトリ区切りは /）" }
                            },
                            "required": ["name"]
                        }
                    },
                    {
                        "name": "save_query",
                        "description": "スクリプト名と SQL テキストを受け取り ~/.tdwh/scripts/<name>.sql に保存する。",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "sql":  { "type": "string" }
                            },
                            "required": ["name", "sql"]
                        }
                    },
                    {
                        "name": "list_scheduled_jobs",
                        "description": "_tdw.scheduled_jobs テーブルの全ジョブを返す。",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "get_scheduled_job",
                        "description": "指定 ID のジョブ詳細を返す。",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string" }
                            },
                            "required": ["id"]
                        }
                    },
                    {
                        "name": "save_scheduled_job",
                        "description": "ジョブを作成または更新する（id 省略時は新規作成として UUID を自動生成）。",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "id":         { "type": "string", "description": "省略または空文字で新規作成" },
                                "name":       { "type": "string" },
                                "job_type":   { "type": "string", "description": "\"Query\" または \"Import\"" },
                                "target_id":  { "type": "string" },
                                "cron_expr":  { "type": "string" },
                                "timezone":   { "type": "string", "description": "省略時は UTC" },
                                "enabled":    { "type": "boolean" },
                                "created_at": { "type": "string", "description": "省略時は現在時刻を自動設定" },
                                "last_run_at":{ "type": "string" }
                            },
                            "required": ["name", "job_type", "target_id", "cron_expr", "enabled"]
                        }
                    },
                    {
                        "name": "delete_scheduled_job",
                        "description": "指定 ID のジョブを削除する。",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string" }
                            },
                            "required": ["id"]
                        }
                    },
                    {
                        "name": "get_scheduler_logs",
                        "description": "指定スクリプト名のスケジューラログを返す。date は YYYYMMDD 形式（省略時は UTC 当日）。",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "script_name": { "type": "string" },
                                "date":        { "type": "string", "description": "YYYYMMDD 形式。省略時は UTC 当日" }
                            },
                            "required": ["script_name"]
                        }
                    }
                ]
            }
        })),

        "tools/call" => {
            let params = req.get("params")?;
            let tool_name = params.get("name")?.as_str()?;
            let args = params.get("arguments").cloned().unwrap_or(json!({}));

            active.store(true, Ordering::SeqCst);
            let _ = app_handle.emit("mcp-active", ());

            write_log(log_path, &format!("CALL  {} args={}", tool_name, args));
            let start = std::time::Instant::now();

            let result = call_tool(tool_name, &args, db, home_dir, app_handle).await;
            let elapsed_ms = start.elapsed().as_millis();

            active.store(false, Ordering::SeqCst);
            let _ = app_handle.emit("mcp-idle", ());

            match result {
                Ok(text) => {
                    write_log(log_path, &format!("OK    {} | {}ms", tool_name, elapsed_ms));
                    Some(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{ "type": "text", "text": text }]
                        }
                    }))
                }
                Err(e) => {
                    write_log(
                        log_path,
                        &format!("ERROR {} | {}ms | {}", tool_name, elapsed_ms, e),
                    );
                    Some(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{ "type": "text", "text": format!("Error: {e}") }],
                            "isError": true
                        }
                    }))
                }
            }
        }

        _ => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": "Method not found" }
        })),
    }
}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

async fn call_tool(
    name: &str,
    args: &Value,
    db: &DbWorker,
    home_dir: &str,
    app_handle: &tauri::AppHandle,
) -> Result<String, String> {
    match name {
        "echo" => {
            let message = args
                .get("message")
                .and_then(|v| v.as_str())
                .ok_or("Missing argument: message")?;
            Ok(message.to_string())
        }

        "run_query" => {
            let sql = args
                .get("sql")
                .and_then(|v| v.as_str())
                .ok_or("Missing argument: sql")?;
            let result = db.query(sql.to_string()).await?;
            serde_json::to_string(&result).map_err(|e| e.to_string())
        }

        "list_tables" => {
            let include_dbt = args
                .get("include_dbt")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let mut tables = db.list_tables().await?;
            if include_dbt {
                let dbt_path = utils::dbt_db_path().to_str().unwrap_or("").to_string();
                // dbt.db may not exist yet — ignore errors
                if let Ok(dbt_tables) = db.list_dbt_tables(dbt_path).await {
                    tables.extend(dbt_tables);
                }
            }
            serde_json::to_string(&tables).map_err(|e| e.to_string())
        }

        "get_schema" => {
            let schema_name = args
                .get("schema_name")
                .and_then(|v| v.as_str())
                .ok_or("Missing argument: schema_name")?;
            let table_name = args
                .get("table_name")
                .and_then(|v| v.as_str())
                .ok_or("Missing argument: table_name")?;
            let database = args
                .get("database")
                .and_then(|v| v.as_str())
                .unwrap_or("main");
            let schema = if database == "dbt" {
                let dbt_path = utils::dbt_db_path().to_str().unwrap_or("").to_string();
                db.get_dbt_schema(dbt_path, schema_name.to_string(), table_name.to_string())
                    .await?
            } else {
                db.get_schema(schema_name.to_string(), table_name.to_string())
                    .await?
            };
            serde_json::to_string(&schema).map_err(|e| e.to_string())
        }

        "reimport_csv" => {
            let schema_name = args
                .get("schema_name")
                .and_then(|v| v.as_str())
                .ok_or("Missing argument: schema_name")?;
            let table_name = args
                .get("table_name")
                .and_then(|v| v.as_str())
                .ok_or("Missing argument: table_name")?;
            let table = db
                .reimport_csv(schema_name.to_string(), table_name.to_string())
                .await?;
            serde_json::to_string(&table).map_err(|e| e.to_string())
        }

        "export_query_csv" => {
            let sql = args
                .get("sql")
                .and_then(|v| v.as_str())
                .ok_or("Missing argument: sql")?;
            let export_dir = args
                .get("export_dir")
                .and_then(|v| v.as_str())
                .ok_or("Missing argument: export_dir")?;
            let filename = args
                .get("filename")
                .and_then(|v| v.as_str())
                .ok_or("Missing argument: filename")?;

            let resolved_dir = std::path::PathBuf::from(utils::expand_home_path(export_dir));

            std::fs::create_dir_all(&resolved_dir).map_err(|e| e.to_string())?;
            let out_path = resolved_dir.join(filename);
            let out_path_str = out_path.to_str().ok_or("Invalid path")?.replace('\'', "''");

            let copy_sql = format!(
                "COPY ({}) TO '{}' (HEADER, DELIMITER ',')",
                sql.trim_end_matches(';'),
                out_path_str
            );
            db.query(copy_sql).await?;
            Ok(out_path.to_str().unwrap_or("").to_string())
        }

        "list_saved_queries" => {
            let scripts_dir = std::path::PathBuf::from(home_dir)
                .join(".tdwh")
                .join("scripts");
            let mut names = Vec::new();
            collect_scripts_mcp(&scripts_dir, &scripts_dir, &mut names);
            names.sort();
            serde_json::to_string(&names).map_err(|e| e.to_string())
        }

        "get_saved_query" => {
            let name = args.get("name").and_then(|v| v.as_str()).ok_or("Missing argument: name")?;
            validate_name(name)?;
            let path = std::path::PathBuf::from(home_dir)
                .join(".tdwh")
                .join("scripts")
                .join(format!("{name}.sql"));
            std::fs::read_to_string(&path).map_err(|e| format!("Failed to read script: {e}"))
        }

        "save_query" => {
            let name = args.get("name").and_then(|v| v.as_str()).ok_or("Missing argument: name")?;
            let sql = args.get("sql").and_then(|v| v.as_str()).ok_or("Missing argument: sql")?;
            validate_name(name)?;
            let path = std::path::PathBuf::from(home_dir)
                .join(".tdwh")
                .join("scripts")
                .join(format!("{name}.sql"));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {e}"))?;
            }
            std::fs::write(&path, sql).map_err(|e| format!("Failed to write script: {e}"))?;
            Ok(format!("Saved: {name}.sql"))
        }

        "list_scheduled_jobs" => {
            let jobs = db.list_scheduled_jobs().await?;
            serde_json::to_string(&jobs).map_err(|e| e.to_string())
        }

        "get_scheduled_job" => {
            let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing argument: id")?;
            let jobs = db.list_scheduled_jobs().await?;
            let job = jobs.into_iter().find(|j| j.id == id)
                .ok_or_else(|| format!("Job not found: {id}"))?;
            serde_json::to_string(&job).map_err(|e| e.to_string())
        }

        "save_scheduled_job" => {
            let original_id = args.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let is_new = original_id.is_empty();
            let id = if is_new { Uuid::new_v4().to_string() } else { original_id };
            let name = args.get("name").and_then(|v| v.as_str()).ok_or("Missing argument: name")?.to_string();
            let job_type_str = args.get("job_type").and_then(|v| v.as_str()).ok_or("Missing argument: job_type")?;
            let job_type: JobType = job_type_str.parse()?;
            let target_id = args.get("target_id").and_then(|v| v.as_str()).ok_or("Missing argument: target_id")?.to_string();
            let cron_expr = args.get("cron_expr").and_then(|v| v.as_str()).ok_or("Missing argument: cron_expr")?.to_string();
            let timezone = args.get("timezone").and_then(|v| v.as_str()).unwrap_or("UTC").to_string();
            let enabled = args.get("enabled").and_then(|v| v.as_bool()).ok_or("Missing argument: enabled")?;
            let created_at = args.get("created_at").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    let secs = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let (y, mo, d, h, mi, s) = epoch_to_datetime(secs);
                    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
                });
            let last_run_at = args.get("last_run_at").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string());
            let job = ScheduledJob { id, name, job_type, target_id, cron_expr, timezone, enabled, created_at, last_run_at };
            db.save_scheduled_job(job).await?;
            if is_new {
                let _ = app_handle.emit("scheduled-jobs-changed", ());
            }
            Ok("Saved".to_string())
        }

        "delete_scheduled_job" => {
            let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing argument: id")?.to_string();
            db.delete_scheduled_job(id).await?;
            let _ = app_handle.emit("scheduled-jobs-changed", ());
            Ok("Deleted".to_string())
        }

        "get_scheduler_logs" => {
            let script_name = args.get("script_name").and_then(|v| v.as_str()).ok_or("Missing argument: script_name")?;
            let date_str = args.get("date").and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    let secs = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let (y, mo, d, _, _, _) = epoch_to_datetime(secs);
                    format!("{y:04}{mo:02}{d:02}")
                });
            let entries = read_log_entries(home_dir, script_name, &date_str);
            serde_json::to_string(&entries).map_err(|e| e.to_string())
        }

        _ => Err(format!("Unknown tool: {name}")),
    }
}

// ---------------------------------------------------------------------------
// Logging helpers
// ---------------------------------------------------------------------------

fn write_log(log_path: &std::path::Path, message: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();
        let (y, mo, d, h, mi, s) = epoch_to_datetime(secs);
        let _ = writeln!(
            f,
            "[{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z] {message}"
        );
    }
}

fn collect_scripts_mcp(dir: &std::path::Path, base: &std::path::Path, names: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            collect_scripts_mcp(&path, base, names);
        } else if path.extension().and_then(|s| s.to_str()) == Some("sql") {
            if let Ok(rel) = path.strip_prefix(base) {
                if let Some(s) = rel.with_extension("").to_str() {
                    names.push(s.replace('\\', "/"));
                }
            }
        }
    }
}

fn epoch_to_datetime(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    let mins = secs / 60;
    let mi = mins % 60;
    let hours = mins / 60;
    let h = hours % 24;
    let days = hours / 24;
    let mut y = 1970u64;
    let mut remaining = days;
    loop {
        let leap = (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400);
        let dy = if leap { 366 } else { 365 };
        if remaining < dy {
            break;
        }
        remaining -= dy;
        y += 1;
    }
    let leap = (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400);
    let month_days: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut mo = 1u64;
    for &md in &month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        mo += 1;
    }
    (y, mo, remaining + 1, h, mi, s)
}
