use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde_json::{json, Value};
use tauri::Emitter;

use crate::db::worker::DbWorker;

fn write_log(log_path: &std::path::Path, message: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_path) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        // Format: [YYYY-MM-DDTHH:MM:SSZ] message
        let secs = now.as_secs();
        let (y, mo, d, h, mi, s) = epoch_to_datetime(secs);
        let _ = writeln!(f, "[{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z] {message}");
    }
}

fn epoch_to_datetime(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    let mins = secs / 60;
    let mi = mins % 60;
    let hours = mins / 60;
    let h = hours % 24;
    let days = hours / 24;
    // Gregorian calendar calculation
    let mut y = 1970u64;
    let mut remaining = days;
    loop {
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        let dy = if leap { 366 } else { 365 };
        if remaining < dy { break; }
        remaining -= dy;
        y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let month_days: [u64; 12] = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut mo = 1u64;
    for &md in &month_days {
        if remaining < md { break; }
        remaining -= md;
        mo += 1;
    }
    (y, mo, remaining + 1, h, mi, s)
}

/// Shared lock: true while an MCP tool call is in progress.
/// Tauri commands check this and reject requests when true.
pub struct McpLock(pub Arc<AtomicBool>);

pub async fn run_mcp_server(
    db: DbWorker,
    active: Arc<AtomicBool>,
    home_dir: String,
    app_handle: tauri::AppHandle,
) {
    // ~/.tdwh/logs/mcp_access.log
    let log_path = std::path::PathBuf::from(&home_dir)
        .join(".tdwh")
        .join("logs")
        .join("mcp_access.log");
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    write_log(&log_path, "MCP server started");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut writer = tokio::io::BufWriter::new(stdout);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF — client disconnected
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let req: Value = match serde_json::from_str(trimmed) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let response =
                    handle_request(&req, &db, &active, &home_dir, &app_handle, &log_path).await;
                if let Some(resp) = response {
                    let mut resp_str =
                        serde_json::to_string(&resp).unwrap_or_default();
                    resp_str.push('\n');
                    let _ = writer.write_all(resp_str.as_bytes()).await;
                    let _ = writer.flush().await;
                }
            }
            Err(_) => break,
        }
    }
}

async fn handle_request(
    req: &Value,
    db: &DbWorker,
    active: &Arc<AtomicBool>,
    home_dir: &str,
    app_handle: &tauri::AppHandle,
    log_path: &std::path::Path,
) -> Option<Value> {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method")?.as_str()?;

    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "tiny-data-warehouse",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        })),

        "notifications/initialized" | "ping" => {
            // Notifications have no id — return nothing; ping returns empty result
            if method == "ping" {
                Some(json!({ "jsonrpc": "2.0", "id": id, "result": {} }))
            } else {
                None
            }
        }

        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
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
                        "description": "List all tables in the database with row/column counts.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "get_schema",
                        "description": "Get column definitions of a table.",
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
                        "name": "echo",
                        "description": "MCPサーバーの疎通確認用。受け取ったメッセージをそのまま返す。",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "message": { "type": "string", "description": "返してほしいメッセージ" }
                            },
                            "required": ["message"]
                        }
                    }
                ]
            }
        })),

        "tools/call" => {
            let params = req.get("params")?;
            let tool_name = params.get("name")?.as_str()?;
            let args = params.get("arguments").cloned().unwrap_or(json!({}));

            // Acquire lock & notify frontend
            active.store(true, Ordering::SeqCst);
            let _ = app_handle.emit("mcp-active", ());

            write_log(log_path, &format!("CALL  {} args={}", tool_name, args));
            let start = std::time::Instant::now();

            let result = call_tool(tool_name, &args, db, home_dir).await;

            let elapsed_ms = start.elapsed().as_millis();

            // Release lock & notify frontend
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
                    write_log(log_path, &format!("ERROR {} | {}ms | {}", tool_name, elapsed_ms, e));
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

async fn call_tool(
    name: &str,
    args: &Value,
    db: &DbWorker,
    home_dir: &str,
) -> Result<String, String> {
    match name {
        "run_query" => {
            let sql = args
                .get("sql")
                .and_then(|v| v.as_str())
                .ok_or("Missing argument: sql")?;
            let result = db.query(sql.to_string()).await?;
            serde_json::to_string(&result).map_err(|e| e.to_string())
        }

        "list_tables" => {
            let tables = db.list_tables().await?;
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
            let schema = db
                .get_schema(schema_name.to_string(), table_name.to_string())
                .await?;
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

            let resolved_dir = if export_dir.starts_with("~/") {
                std::path::PathBuf::from(home_dir).join(&export_dir[2..])
            } else {
                std::path::PathBuf::from(export_dir)
            };

            std::fs::create_dir_all(&resolved_dir).map_err(|e| e.to_string())?;
            let out_path = resolved_dir.join(filename);
            let out_path_str = out_path
                .to_str()
                .ok_or("Invalid path")?
                .replace('\'', "''");

            let copy_sql = format!(
                "COPY ({}) TO '{}' (HEADER, DELIMITER ',')",
                sql.trim_end_matches(';'),
                out_path_str
            );
            db.query(copy_sql).await?;
            Ok(out_path.to_str().unwrap_or("").to_string())
        }

        "echo" => {
            let message = args
                .get("message")
                .and_then(|v| v.as_str())
                .ok_or("Missing argument: message")?;
            Ok(message.to_string())
        }

        _ => Err(format!("Unknown tool: {name}")),
    }
}
