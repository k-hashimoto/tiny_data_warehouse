mod commands;
mod config;
mod db;
mod file_io;
mod metadata_yml;
mod mcp;
mod scheduler;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use db::worker::DbWorker;
use mcp::{McpLock, McpServerHandle};
use notify::Watcher;
use tauri::{Emitter, Manager};

#[tauri::command]
async fn get_mcp_server_status(handle: tauri::State<'_, Arc<McpServerHandle>>) -> Result<bool, String> {
    Ok(handle.is_running().await)
}

#[tauri::command]
async fn stop_mcp_server(handle: tauri::State<'_, Arc<McpServerHandle>>) -> Result<(), String> {
    handle.stop().await;
    Ok(())
}

#[tauri::command]
async fn restart_mcp_server(
    db: tauri::State<'_, DbWorker>,
    lock: tauri::State<'_, McpLock>,
    handle: tauri::State<'_, Arc<McpServerHandle>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    handle.stop().await;
    // Brief pause to let the port be released
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let home_str = home.to_str().unwrap_or("").to_string();
    let db_clone = db.inner().clone();
    let active_clone = lock.0.clone();
    let handle_clone = handle.inner().clone();
    tauri::async_runtime::spawn(mcp::run_mcp_server(db_clone, active_clone, home_str, app, handle_clone));
    Ok(())
}

struct FileWatcher(notify::RecommendedWatcher);
unsafe impl Send for FileWatcher {}
unsafe impl Sync for FileWatcher {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let home = app.path().home_dir()?;
            let db_dir = home.join(".tdwh").join("db");
            std::fs::create_dir_all(&db_dir)?;

            let app_db_path = db_dir.join("app.db");
            let dbt_db_path = db_dir.join("dbt.db");
            let db = DbWorker::new(
                app_db_path.to_str().unwrap_or(":memory:"),
                dbt_db_path.to_str().unwrap_or(""),
            );
            app.manage(db.clone());

            // MCP lock — shared between the MCP server task and Tauri commands
            let mcp_active = Arc::new(AtomicBool::new(false));
            app.manage(McpLock(mcp_active.clone()));

            // MCP server handle — used for stop/restart
            let mcp_handle = Arc::new(McpServerHandle::empty());
            app.manage(mcp_handle.clone());

            // Launch the built-in MCP server (Streamable HTTP)
            let mcp_home = home.to_str().unwrap_or("").to_string();
            let mcp_app = app.handle().clone();
            tauri::async_runtime::spawn(mcp::run_mcp_server(
                db,
                mcp_active,
                mcp_home,
                mcp_app,
                mcp_handle,
            ));

            commands::scripts::seed_default_scripts(app.handle());

            // File watcher: notify frontend when dbt.db changes
            let (change_tx, mut change_rx) = tokio::sync::mpsc::channel::<()>(8);

            let watcher_result = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    use notify::EventKind;
                    let relevant = matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                    );
                    if relevant {
                        let is_dbt = event.paths.iter().any(|p| {
                            p.file_name().and_then(|n| n.to_str()) == Some("dbt.db")
                        });
                        if is_dbt {
                            let _ = change_tx.try_send(());
                        }
                    }
                }
            });

            match watcher_result {
                Ok(mut watcher) => {
                    let _ = watcher.watch(&db_dir, notify::RecursiveMode::NonRecursive);
                    app.manage(FileWatcher(watcher));
                }
                Err(e) => eprintln!("Failed to start file watcher: {e}"),
            }

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    // Wait for first event
                    if change_rx.recv().await.is_none() { break; }

                    // Notify frontend that dbt is running
                    let _ = app_handle.emit("dbt-db-running", ());

                    // Debounce: wait until no events for 1.5s (dbt run may produce many events)
                    loop {
                        tokio::select! {
                            msg = change_rx.recv() => {
                                if msg.is_none() { return; }
                                // More events coming — keep waiting
                            }
                            _ = tokio::time::sleep(tokio::time::Duration::from_millis(config::DEBOUNCE_MS)) => {
                                break; // Quiet for 1.5s — dbt likely finished
                            }
                        }
                    }

                    let _ = app_handle.emit("dbt-db-changed", ());
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::config::get_editor_config,
            commands::config::save_editor_config,
            commands::scripts::list_scripts,
            commands::scripts::read_script,
            commands::scripts::save_script,
            commands::scripts::delete_script,
            commands::scripts::rename_script,
            commands::query::run_query,
            commands::explorer::list_tables,
            commands::explorer::list_dbt_tables,
            commands::explorer::preview_dbt_table,
            commands::explorer::get_schema,
            commands::explorer::preview_table,
            commands::csv::preview_csv,
            commands::csv::import_csv,
            commands::csv::reimport_csv,
            commands::csv::export_csv,
            commands::csv::export_query_csv,
            commands::scripts::execute_saved_query,
            commands::explorer::list_schemas,
            commands::explorer::attach_dbt,
            commands::explorer::detach_dbt,
            commands::explorer::get_dbt_schema,
            commands::explorer::drop_dbt_table,
            commands::explorer::drop_dbt_schema,
            commands::metadata::get_table_meta,
            commands::metadata::get_dbt_table_meta,
            commands::metadata::set_table_comment,
            commands::metadata::set_column_comment,
            commands::metadata::save_table_meta,
            commands::metadata::sync_yml_metadata,
            commands::metadata::touch_dbt_timestamps,
            get_mcp_server_status,
            stop_mcp_server,
            restart_mcp_server,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
