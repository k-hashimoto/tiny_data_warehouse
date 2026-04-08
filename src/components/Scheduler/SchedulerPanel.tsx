import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CheckCircleIcon, XCircleIcon, Trash2Icon, PencilIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useAppStore } from "@/store/appStore";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { SchedulerJobForm, ScheduledJobPayload, cronExprToForm } from "./SchedulerJobForm";

interface SchedulerLogEntry {
  job_id: string;
  script_name: string;
  started_at: string;
  finished_at: string;
  success: boolean;
  error_message: string | null;
}

export function SchedulerPanel() {
  const activeTabId = useAppStore((s) => s.activeTabId);
  const linkedScript = useAppStore((s) => s.getActiveTab().linkedScript);

  const [job, setJob] = useState<ScheduledJobPayload | null>(null);
  const [logs, setLogs] = useState<SchedulerLogEntry[]>([]);
  const [editing, setEditing] = useState(false);

  const loadJob = useCallback(async (scriptName: string) => {
    try {
      const all = await invoke<ScheduledJobPayload[]>("list_scheduled_jobs");
      const found = all.find((j) => j.target_id === scriptName) ?? null;
      setJob(found);
      setEditing(false);
    } catch (e) {
      console.error("Failed to load scheduled job", e);
    }
  }, []);

  const loadLogs = useCallback(async (scriptName: string) => {
    try {
      const entries = await invoke<SchedulerLogEntry[]>("get_scheduler_logs", {
        scriptName,
        date: null,
      });
      setLogs(entries);
    } catch (e) {
      setLogs([]);
    }
  }, []);

  // アクティブタブが切り替わるたびにジョブ・ログを再取得
  useEffect(() => {
    setJob(null);
    setLogs([]);
    setEditing(false);
    if (!linkedScript) return;
    loadJob(linkedScript);
    loadLogs(linkedScript);
  }, [activeTabId, linkedScript, loadJob, loadLogs]);

  async function handleSave(payload: ScheduledJobPayload) {
    try {
      // 既存ジョブがあれば同じIDで上書き（1スクリプト1ジョブ）
      const toSave = job ? { ...payload, id: job.id } : payload;
      await invoke("save_scheduled_job", { job: toSave });
      await invoke("reload_scheduler");
      if (linkedScript) {
        await loadJob(linkedScript);
        await loadLogs(linkedScript);
      }
    } catch (e) {
      console.error("Failed to save job", e);
    }
  }

  async function handleDelete() {
    if (!job) return;
    try {
      await invoke("delete_scheduled_job", { id: job.id });
      await invoke("reload_scheduler");
      setJob(null);
      setLogs([]);
    } catch (e) {
      console.error("Failed to delete job", e);
    }
  }

  async function handleToggle() {
    if (!job) return;
    try {
      await invoke("save_scheduled_job", { job: { ...job, enabled: !job.enabled } });
      await invoke("reload_scheduler");
      if (linkedScript) await loadJob(linkedScript);
    } catch (e) {
      console.error("Failed to toggle job", e);
    }
  }

  // linkedScript なし
  if (!linkedScript) {
    return (
      <div className="h-full flex items-center justify-center">
        <p className="text-xs text-muted-foreground">このタブはSaved Queryではありません</p>
      </div>
    );
  }

  const scriptLabel = linkedScript.includes("/")
    ? linkedScript.substring(linkedScript.lastIndexOf("/") + 1)
    : linkedScript;

  return (
    <ResizablePanelGroup orientation="vertical" className="h-full">
      {/* 上ペイン: スケジュール設定 */}
      <ResizablePanel defaultSize="45%" minSize="120px">
        <div className="h-full flex flex-col overflow-hidden">
          <div className="px-3 py-1.5 border-b text-xs font-semibold text-muted-foreground shrink-0 flex items-center justify-between">
            <span>スケジュール設定 — <span className="text-foreground">{scriptLabel}</span></span>
            {job && !editing && (
              <div className="flex items-center gap-1">
                <button
                  className={`rounded-full w-2.5 h-2.5 border shrink-0 ${
                    job.enabled ? "bg-green-500 border-green-600" : "bg-muted border-muted-foreground/40"
                  }`}
                  title={job.enabled ? "有効（クリックで無効化）" : "無効（クリックで有効化）"}
                  onClick={handleToggle}
                />
                <Button size="icon" variant="ghost" className="h-5 w-5" title="編集" onClick={() => setEditing(true)}>
                  <PencilIcon className="h-3 w-3" />
                </Button>
                <Button size="icon" variant="ghost" className="h-5 w-5" title="削除" onClick={handleDelete}>
                  <Trash2Icon className="h-3 w-3 text-destructive" />
                </Button>
              </div>
            )}
          </div>

          <div className="flex-1 overflow-y-auto">
            {editing || !job ? (
              <SchedulerJobForm
                scriptName={linkedScript}
                existingJob={editing && job ? job : undefined}
                onSave={handleSave}
                onCancel={() => setEditing(false)}
              />
            ) : (
              /* ジョブ設定サマリー */
              <div className="px-3 py-2 flex flex-col gap-1">
                <div className="flex items-center gap-2 text-xs">
                  <span className="text-muted-foreground w-20 shrink-0">ジョブ名</span>
                  <span>{job.name}</span>
                </div>
                <div className="flex items-center gap-2 text-xs">
                  <span className="text-muted-foreground w-20 shrink-0">スケジュール</span>
                  <span className="font-mono">{job.cron_expr}</span>
                  <span className="text-muted-foreground">({scheduleLabel(job.cron_expr)})</span>
                </div>
                <div className="flex items-center gap-2 text-xs">
                  <span className="text-muted-foreground w-20 shrink-0">ステータス</span>
                  <span className={job.enabled ? "text-green-600" : "text-muted-foreground"}>
                    {job.enabled ? "有効" : "無効"}
                  </span>
                </div>
                {job.last_run_at && (
                  <div className="flex items-center gap-2 text-xs">
                    <span className="text-muted-foreground w-20 shrink-0">前回実行</span>
                    <span>{job.last_run_at.replace("T", " ").replace("Z", " UTC")}</span>
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      </ResizablePanel>

      <ResizableHandle withHandle className="z-10" />

      {/* 下ペイン: 実行ログ */}
      <ResizablePanel minSize="80px">
        <div className="h-full flex flex-col overflow-hidden">
          <div className="px-3 py-1.5 border-b text-xs font-semibold text-muted-foreground shrink-0">
            実行ログ（本日）
          </div>
          <div className="flex-1 overflow-y-auto px-3 py-2">
            {!job ? (
              <p className="text-xs text-muted-foreground">スケジュールが設定されていません</p>
            ) : logs.length === 0 ? (
              <p className="text-xs text-muted-foreground">本日の実行ログがありません</p>
            ) : (
              <div className="flex flex-col gap-1">
                {logs.map((entry, idx) => (
                  <div key={idx} className="flex items-start gap-2 py-1 border-b last:border-0">
                    {entry.success
                      ? <CheckCircleIcon className="h-3.5 w-3.5 text-green-500 shrink-0 mt-0.5" />
                      : <XCircleIcon className="h-3.5 w-3.5 text-destructive shrink-0 mt-0.5" />}
                    <div className="flex flex-col min-w-0 gap-0.5">
                      <span className="text-xs text-muted-foreground">
                        {entry.started_at.replace("T", " ").replace("Z", " UTC")}
                        {" → "}
                        {entry.finished_at.replace("T", " ").replace("Z", " UTC")}
                      </span>
                      {entry.error_message && (
                        <span className="text-xs text-destructive break-all">
                          {entry.error_message}
                        </span>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}

/** cron式を人が読める文字列に変換 */
function scheduleLabel(cronExpr: string): string {
  const form = cronExprToForm(cronExpr);
  const WEEKDAY = ["日", "月", "火", "水", "木", "金", "土"];
  switch (form.type) {
    case "every_n_minutes":
      return form.intervalMinutes === 60 ? "毎時" : `${form.intervalMinutes}分ごと`;
    case "daily":
      return `毎日 ${String(form.hour ?? 0).padStart(2, "0")}:${String(form.minute ?? 0).padStart(2, "0")}`;
    case "weekly":
      return `毎週${WEEKDAY[form.weekday ?? 0]} ${String(form.hour ?? 0).padStart(2, "0")}:${String(form.minute ?? 0).padStart(2, "0")}`;
    case "monthly":
      return `毎月${form.monthDay ?? 1}日 ${String(form.hour ?? 0).padStart(2, "0")}:${String(form.minute ?? 0).padStart(2, "0")}`;
    default:
      return cronExpr;
  }
}
