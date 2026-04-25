use std::sync::Arc;
use tauri::State;

use crate::cron_engine::{read_log_entries, CronEngine, SchedulerLogEntry};

/// 指定スクリプトの実行ログを取得する。
/// date が None の場合は当日（UTC）のログを返す。
/// ファイルが存在しない場合は空 Vec を返す。
#[tauri::command]
pub async fn get_scheduler_logs(
    engine: State<'_, Arc<CronEngine>>,
    script_name: String,
    date: Option<String>,
) -> Result<Vec<SchedulerLogEntry>, String> {
    let home_dir = engine.home_dir();
    let date_str = match date {
        Some(d) => d,
        None => {
            // 当日の YYYYMMDD 文字列（UTC）
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            epoch_to_date_str(now)
        }
    };
    Ok(read_log_entries(&home_dir, &script_name, &date_str))
}

/// スケジューラに再ロードシグナルを送る。
#[tauri::command]
pub async fn reload_scheduler(engine: State<'_, Arc<CronEngine>>) -> Result<(), String> {
    engine.reload();
    Ok(())
}

/// エポック秒を "YYYYMMDD" 文字列に変換する（UTC）。
fn epoch_to_date_str(secs: u64) -> String {
    let (y, mo, d, _, _, _) = epoch_to_datetime(secs);
    format!("{y:04}{mo:02}{d:02}")
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
