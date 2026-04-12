pub mod config;
pub mod csv;
pub mod explorer;
pub mod json;
pub mod macros;
pub mod metadata;
pub mod query;
pub mod scheduled_jobs;
pub mod scheduler_commands;
pub mod scripts;

use std::sync::atomic::Ordering;
use crate::mcp::McpLock;

/// MCP ロックが有効な場合にエラーを返すヘルパー。
/// Tauri コマンド内で `ensure_not_mcp_locked(&lock)?;` として使用する。
pub fn ensure_not_mcp_locked(mcp_lock: &McpLock) -> Result<(), String> {
    if mcp_lock.0.load(Ordering::SeqCst) {
        Err("AI操作中です。しばらくお待ちください。".to_string())
    } else {
        Ok(())
    }
}
