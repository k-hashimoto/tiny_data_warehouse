use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use cron::Schedule;
use serde::{Deserialize, Serialize};

use crate::db::worker::DbWorker;
use crate::mcp::McpLock;
use crate::scheduler::{JobType, ScheduledJob};

/// フロントエンド向けのログエントリ
#[derive(Debug, Serialize, Deserialize)]
pub struct SchedulerLogEntry {
    pub job_id: String,
    pub script_name: String,
    pub started_at: String,
    pub finished_at: String,
    pub success: bool,
    pub error_message: Option<String>,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default)]
    pub is_final_attempt: bool,
}

const MAX_RETRIES: u32 = 2;
const RETRY_INTERVAL_SECS: u64 = 60;
const JOB_TIMEOUT_SECS: u64 = 300;

/// スケジューラエンジン。Arc<Mutex<>> で共有するが、
/// reload_tx のみ内部変異が必要なため Mutex を使わず watch チャネルで制御する。
pub struct CronEngine {
    db: DbWorker,
    home_dir: String,
    mcp_lock: McpLock,
    reload_tx: tokio::sync::watch::Sender<()>,
    /// 現在実行中のジョブID集合。重複実行防止に使用する。
    executing: Arc<std::sync::Mutex<HashSet<String>>>,
}

impl CronEngine {
    pub fn new(
        db: DbWorker,
        home_dir: String,
        mcp_lock: McpLock,
    ) -> (Arc<Self>, tokio::sync::watch::Receiver<()>) {
        let (reload_tx, reload_rx) = tokio::sync::watch::channel(());
        let engine = Arc::new(Self {
            db,
            home_dir,
            mcp_lock,
            reload_tx,
            executing: Arc::new(std::sync::Mutex::new(HashSet::new())),
        });
        (engine, reload_rx)
    }

    /// バックグラウンドループを起動する。
    pub fn start(self: Arc<Self>, mut reload_rx: tokio::sync::watch::Receiver<()>) {
        tauri::async_runtime::spawn(async move {
            loop {
                // ループを再起動するたびにジョブ一覧を再ロード
                let jobs = match self.db.list_scheduled_jobs().await {
                    Ok(j) => j,
                    Err(e) => {
                        eprintln!("[cron_engine] Failed to load jobs: {e}");
                        // リロードシグナルを待つ
                        let _ = reload_rx.changed().await;
                        continue;
                    }
                };

                // enabled な Query ジョブのみを対象にする
                let active_jobs: Vec<ScheduledJob> = jobs
                    .into_iter()
                    .filter(|j| j.enabled && matches!(j.job_type, JobType::Query))
                    .collect();

                if active_jobs.is_empty() {
                    // ジョブがなければリロードシグナルを待つ
                    let _ = reload_rx.changed().await;
                    continue;
                }

                // 各ジョブの次回実行時刻を計算して最も早い時刻とそのジョブ一覧を収集する
                let now_secs = current_epoch_secs();
                let mut earliest_secs: Option<u64> = None;
                let mut jobs_to_run: Vec<usize> = vec![];

                for (idx, job) in active_jobs.iter().enumerate() {
                    if let Some(next) = next_run_secs(&job.cron_expr, now_secs) {
                        match earliest_secs {
                            None => {
                                earliest_secs = Some(next);
                                jobs_to_run = vec![idx];
                            }
                            Some(e) if next < e => {
                                earliest_secs = Some(next);
                                jobs_to_run = vec![idx];
                            }
                            Some(e) if next == e => {
                                jobs_to_run.push(idx);
                            }
                            _ => {}
                        }
                    }
                }

                let Some(next_secs) = earliest_secs else {
                    // cron パースに失敗したジョブしかない場合
                    let _ = reload_rx.changed().await;
                    continue;
                };

                let wait_secs = next_secs.saturating_sub(now_secs);
                let sleep = tokio::time::sleep(tokio::time::Duration::from_secs(wait_secs));
                tokio::pin!(sleep);

                tokio::select! {
                    _ = &mut sleep => {
                        // 同一時刻にスケジュールされたジョブをすべて spawn する
                        for job_idx in jobs_to_run {
                            let job = active_jobs[job_idx].clone();
                            let db = self.db.clone();
                            let home = self.home_dir.clone();
                            let mcp_active = self.mcp_lock.0.clone();
                            let executing = self.executing.clone();
                            let cancel_rx = reload_rx.clone();
                            tauri::async_runtime::spawn(async move {
                                execute_job(&job, &db, &home, &mcp_active, &executing, cancel_rx).await;
                            });
                        }
                        // 次のループへ（ジョブ一覧を再ロード）
                    }
                    _ = reload_rx.changed() => {
                        // リロードシグナルを受け取ったのでループを再起動
                    }
                }
            }
        });
    }

    /// スケジューラに再ロードシグナルを送る。
    pub fn reload(&self) {
        let _ = self.reload_tx.send(());
    }

    /// ホームディレクトリ文字列を返す。
    pub fn home_dir(&self) -> String {
        self.home_dir.clone()
    }
}

/// Unix エポック秒（UTC）を返す。
fn current_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// cron 式から次回実行時刻（エポック秒）を計算する。
/// パース失敗時は None を返す。
fn next_run_secs(cron_expr: &str, now_secs: u64) -> Option<u64> {
    use std::str::FromStr;
    // cron クレートは quartz 形式（秒フィールド付き）を基本とする。
    // 5 フィールド式（分 時 日 月 曜日）が渡されたら先頭に "0 " を付けて 6 フィールドにする。
    let expr = if cron_expr.split_whitespace().count() == 5 {
        format!("0 {}", cron_expr)
    } else {
        cron_expr.to_string()
    };

    let schedule = Schedule::from_str(&expr).ok()?;
    // cron クレートは chrono::DateTime を使う。
    // chrono 非使用の制約があるが、cron クレート自体が chrono に依存しているため
    // 内部的な next() 呼び出しのみ利用し、エポック秒に変換する。
    let now_dt = chrono_from_epoch(now_secs)?;
    let next_dt = schedule.after(&now_dt).next()?;
    Some(next_dt.timestamp() as u64)
}

/// エポック秒を chrono::DateTime<chrono::Utc> に変換するヘルパー。
fn chrono_from_epoch(secs: u64) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::TimeZone;
    chrono::Utc.timestamp_opt(secs as i64, 0).single()
}

/// エポック秒を "YYYYMMDD" 文字列に変換する（UTC）。
fn epoch_to_date_str(secs: u64) -> String {
    let (y, mo, d, _, _, _) = epoch_to_datetime(secs);
    format!("{y:04}{mo:02}{d:02}")
}

/// エポック秒を ISO 8601 UTC 文字列に変換する。
fn epoch_to_iso(secs: u64) -> String {
    let (y, mo, d, h, mi, s) = epoch_to_datetime(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
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

/// ジョブを実行し、ログを書き込む。
async fn execute_job(
    job: &ScheduledJob,
    db: &DbWorker,
    home_dir: &str,
    mcp_active: &Arc<std::sync::atomic::AtomicBool>,
    executing: &Arc<std::sync::Mutex<HashSet<String>>>,
    mut cancel_rx: tokio::sync::watch::Receiver<()>,
) {
    let started_at = current_epoch_secs();

    // MCP ロック中はスキップ
    if mcp_active.load(Ordering::SeqCst) {
        let finished_at = current_epoch_secs();
        let entry = SchedulerLogEntry {
            job_id: job.id.clone(),
            script_name: job.target_id.clone(),
            started_at: epoch_to_iso(started_at),
            finished_at: epoch_to_iso(finished_at),
            success: false,
            error_message: Some("Skipped: MCP operation in progress".to_string()),
            retry_count: 0,
            is_final_attempt: true,
        };
        write_log_entry(home_dir, &job.target_id, started_at, &entry);
        return;
    }

    // 重複実行防止: 既に実行中の場合はスキップ
    {
        let mut set = executing.lock().unwrap_or_else(|e| e.into_inner());
        if set.contains(&job.id) {
            let finished_at = current_epoch_secs();
            let entry = SchedulerLogEntry {
                job_id: job.id.clone(),
                script_name: job.target_id.clone(),
                started_at: epoch_to_iso(started_at),
                finished_at: epoch_to_iso(finished_at),
                success: false,
                error_message: Some("Skipped: already running".to_string()),
                retry_count: 0,
                is_final_attempt: true,
            };
            write_log_entry(home_dir, &job.target_id, started_at, &entry);
            return;
        }
        set.insert(job.id.clone());
    }

    // スクリプトファイルを読み込む
    let script_path = PathBuf::from(home_dir)
        .join(".tdwh")
        .join("scripts")
        .join(format!("{}.sql", job.target_id));

    let sql = match std::fs::read_to_string(&script_path) {
        Ok(s) => s,
        Err(e) => {
            let finished_at = current_epoch_secs();
            let entry = SchedulerLogEntry {
                job_id: job.id.clone(),
                script_name: job.target_id.clone(),
                started_at: epoch_to_iso(started_at),
                finished_at: epoch_to_iso(finished_at),
                success: false,
                error_message: Some(format!("Failed to read script: {e}")),
                retry_count: 0,
                is_final_attempt: true,
            };
            write_log_entry(home_dir, &job.target_id, started_at, &entry);
            // 実行完了としてジョブIDを削除
            executing
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&job.id);
            return;
        }
    };

    for attempt in 0..=MAX_RETRIES {
        let is_final_attempt = attempt == MAX_RETRIES;
        let attempt_started_at = current_epoch_secs();

        // クエリを実行する（300秒タイムアウト）
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(JOB_TIMEOUT_SECS),
            db.query(sql.clone()),
        )
        .await;
        let finished_at = current_epoch_secs();

        let entry = match result {
            Ok(Ok(_)) => SchedulerLogEntry {
                job_id: job.id.clone(),
                script_name: job.target_id.clone(),
                started_at: epoch_to_iso(attempt_started_at),
                finished_at: epoch_to_iso(finished_at),
                success: true,
                error_message: None,
                retry_count: attempt,
                is_final_attempt: true,
            },
            Ok(Err(e)) => SchedulerLogEntry {
                job_id: job.id.clone(),
                script_name: job.target_id.clone(),
                started_at: epoch_to_iso(attempt_started_at),
                finished_at: epoch_to_iso(finished_at),
                success: false,
                error_message: Some(e),
                retry_count: attempt,
                is_final_attempt,
            },
            Err(_elapsed) => SchedulerLogEntry {
                job_id: job.id.clone(),
                script_name: job.target_id.clone(),
                started_at: epoch_to_iso(attempt_started_at),
                finished_at: epoch_to_iso(finished_at),
                success: false,
                error_message: Some("Job timed out after 300s".to_string()),
                retry_count: attempt,
                is_final_attempt,
            },
        };

        let should_stop = entry.success || is_final_attempt;
        write_log_entry(home_dir, &job.target_id, attempt_started_at, &entry);
        if should_stop {
            break;
        }

        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_INTERVAL_SECS)) => {}
            _ = cancel_rx.changed() => {
                executing.lock().unwrap_or_else(|e| e.into_inner()).remove(&job.id);
                return;
            }
        }
    }

    // 成功・失敗問わず実行完了としてジョブIDを削除
    executing
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&job.id);
}

/// ログエントリを JSONL 形式でファイルに追記する。
fn write_log_entry(home_dir: &str, script_name: &str, epoch_secs: u64, entry: &SchedulerLogEntry) {
    let date_str = epoch_to_date_str(epoch_secs);
    // スクリプト名のスラッシュをディレクトリ区切りとして扱う
    let log_path = PathBuf::from(home_dir)
        .join(".tdwh")
        .join("logs")
        .join("scheduler")
        .join(script_name)
        .join(format!("{date_str}.log"));

    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(line) = serde_json::to_string(entry) {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// ログファイルを読み込んで SchedulerLogEntry の Vec を返す。
pub fn read_log_entries(
    home_dir: &str,
    script_name: &str,
    date_str: &str,
) -> Vec<SchedulerLogEntry> {
    let log_path = PathBuf::from(home_dir)
        .join(".tdwh")
        .join("logs")
        .join("scheduler")
        .join(script_name)
        .join(format!("{date_str}.log"));

    let content = match std::fs::read_to_string(&log_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    content
        .lines()
        .filter_map(|line| serde_json::from_str::<SchedulerLogEntry>(line).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoch_to_date_str() {
        // 2026-04-07 00:00:00 UTC = 1775520000
        assert_eq!(epoch_to_date_str(1775520000), "20260407");
    }

    #[test]
    fn test_epoch_to_iso() {
        // 2026-04-07 12:34:56 UTC
        let secs = 1775520000u64 + 12 * 3600 + 34 * 60 + 56;
        assert_eq!(epoch_to_iso(secs), "2026-04-07T12:34:56Z");
    }

    #[test]
    fn test_read_log_entries_missing_file() {
        // ファイルが存在しない場合は空 Vec を返す
        let entries = read_log_entries("/nonexistent", "test_script", "20260407");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_log_entry_roundtrip() {
        let entry = SchedulerLogEntry {
            job_id: "job-1".to_string(),
            script_name: "test".to_string(),
            started_at: "2026-04-07T12:00:00Z".to_string(),
            finished_at: "2026-04-07T12:00:01Z".to_string(),
            success: true,
            error_message: None,
            retry_count: 0,
            is_final_attempt: true,
        };
        let line = serde_json::to_string(&entry).unwrap();
        let parsed: SchedulerLogEntry = serde_json::from_str(&line).unwrap();
        assert_eq!(parsed.job_id, "job-1");
        assert!(parsed.success);
        assert_eq!(parsed.retry_count, 0);
        assert!(parsed.is_final_attempt);
    }

    #[test]
    fn test_log_entry_with_retry_fields() {
        let entry = SchedulerLogEntry {
            job_id: "job-2".to_string(),
            script_name: "test".to_string(),
            started_at: "2026-04-07T12:00:00Z".to_string(),
            finished_at: "2026-04-07T12:00:01Z".to_string(),
            success: false,
            error_message: Some("query failed".to_string()),
            retry_count: 2,
            is_final_attempt: true,
        };
        let line = serde_json::to_string(&entry).unwrap();
        let parsed: SchedulerLogEntry = serde_json::from_str(&line).unwrap();
        assert_eq!(parsed.retry_count, 2);
        assert!(parsed.is_final_attempt);
        assert_eq!(parsed.error_message, Some("query failed".to_string()));
    }

    #[test]
    fn test_log_entry_backward_compat() {
        let legacy = r#"{
            "job_id":"job-legacy",
            "script_name":"legacy",
            "started_at":"2026-04-07T12:00:00Z",
            "finished_at":"2026-04-07T12:00:01Z",
            "success":false,
            "error_message":"failed"
        }"#;
        let parsed: SchedulerLogEntry = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.job_id, "job-legacy");
        assert_eq!(parsed.retry_count, 0);
        assert!(!parsed.is_final_attempt);
    }

    #[test]
    fn test_next_run_secs_every_15min() {
        // */15 * * * * の cron 式が有効にパースされることを確認
        let now = 1775520000u64; // 2026-04-07 00:00:00 UTC
        let result = next_run_secs("*/15 * * * *", now);
        assert!(result.is_some());
        // 次回は now より後
        assert!(result.unwrap() > now);
    }

    #[test]
    fn test_next_run_secs_invalid() {
        let result = next_run_secs("not a cron", 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_collect_jobs_same_cron() {
        // 同じ cron 式を持つ 2 ジョブが同一の next_secs を返すことを確認
        let now = 1775520000u64; // 2026-04-07 00:00:00 UTC
        let cron = "0 * * * *"; // 毎時0分
        let next1 = next_run_secs(cron, now).unwrap();
        let next2 = next_run_secs(cron, now).unwrap();
        assert_eq!(
            next1, next2,
            "同じ cron 式は同じ next_secs を返す必要がある"
        );
        assert!(next1 > now);
    }

    #[test]
    fn test_collect_jobs_different_cron() {
        // 異なる cron 式は異なる next_secs を返すことを確認
        let now = 1775520000u64; // 2026-04-07 00:00:00 UTC（分=0）
                                 // now はちょうど 00:00:00 なので、"*/15 * * * *" の次回は 00:15:00
                                 // "0 * * * *" の次回は 01:00:00
        let next_15min = next_run_secs("*/15 * * * *", now).unwrap();
        let next_hourly = next_run_secs("0 * * * *", now).unwrap();
        assert!(
            next_15min < next_hourly,
            "15分ごとは1時間ごとより早く実行される"
        );
    }
}
