use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde_json::{json, Value};
use tauri::Emitter;

use crate::db::worker::DbWorker;

/// Shared lock: true while an MCP tool call is in progress.
/// Tauri commands check this and reject requests when true.
pub struct McpLock(pub Arc<AtomicBool>);

pub async fn run_mcp_server(
    db: DbWorker,
    active: Arc<AtomicBool>,
    home_dir: String,
    app_handle: tauri::AppHandle,
) {
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
                    handle_request(&req, &db, &active, &home_dir, &app_handle).await;
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

            let result = call_tool(tool_name, &args, db, home_dir).await;

            // Release lock & notify frontend
            active.store(false, Ordering::SeqCst);
            let _ = app_handle.emit("mcp-idle", ());

            match result {
                Ok(text) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{ "type": "text", "text": text }]
                    }
                })),
                Err(e) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{ "type": "text", "text": format!("Error: {e}") }],
                        "isError": true
                    }
                })),
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

        _ => Err(format!("Unknown tool: {name}")),
    }
}
