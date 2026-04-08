import { useState } from "react";
import { Button } from "@/components/ui/button";

export interface ScheduledJobPayload {
  id: string;
  name: string;
  job_type: string;
  target_id: string;
  cron_expr: string;
  enabled: boolean;
  created_at: string;
  last_run_at: string | null;
}

export type ScheduleType = "every_n_minutes" | "daily" | "weekly" | "monthly";

export interface ScheduleForm {
  type: ScheduleType;
  intervalMinutes?: 15 | 30 | 60;
  weekday?: number;   // 0=Sun ... 6=Sat
  monthDay?: number;  // 1-28
  hour?: number;
  minute?: number;
}

const WEEKDAY_LABELS = ["日", "月", "火", "水", "木", "金", "土"];

// ScheduleForm を cron 式に変換する
export function formToCronExpr(form: ScheduleForm): string {
  switch (form.type) {
    case "every_n_minutes": {
      const n = form.intervalMinutes ?? 15;
      return n === 60 ? "0 * * * *" : `*/${n} * * * *`;
    }
    case "daily": {
      const h = form.hour ?? 0;
      const m = form.minute ?? 0;
      return `${m} ${h} * * *`;
    }
    case "weekly": {
      const h = form.hour ?? 0;
      const m = form.minute ?? 0;
      const wd = form.weekday ?? 0;
      return `${m} ${h} * * ${wd}`;
    }
    case "monthly": {
      const h = form.hour ?? 0;
      const m = form.minute ?? 0;
      const d = form.monthDay ?? 1;
      return `${m} ${h} ${d} * *`;
    }
  }
}

// cron 式を ScheduleForm に逆変換する（既存ジョブの編集用）
export function cronExprToForm(expr: string): ScheduleForm {
  const parts = expr.trim().split(/\s+/);
  if (parts.length !== 5) {
    return { type: "every_n_minutes", intervalMinutes: 15 };
  }
  const [minPart, hourPart, dayPart, , wdPart] = parts;

  // 毎 N 分パターン: */15 * * * * or */30 * * * * or 0 * * * *
  if (minPart === "0" && hourPart === "*" && dayPart === "*" && wdPart === "*") {
    return { type: "every_n_minutes", intervalMinutes: 60 };
  }
  if (minPart.startsWith("*/") && hourPart === "*" && dayPart === "*" && wdPart === "*") {
    const n = parseInt(minPart.slice(2), 10);
    if (n === 15 || n === 30) {
      return { type: "every_n_minutes", intervalMinutes: n };
    }
  }
  // 毎日パターン: m h * * *
  if (dayPart === "*" && wdPart === "*" && hourPart !== "*") {
    const h = parseInt(hourPart, 10);
    const m = parseInt(minPart, 10);
    if (!isNaN(h) && !isNaN(m)) {
      return { type: "daily", hour: h, minute: m };
    }
  }
  // 毎週パターン: m h * * wd
  if (dayPart === "*" && wdPart !== "*") {
    const wd = parseInt(wdPart, 10);
    const h = parseInt(hourPart, 10);
    const m = parseInt(minPart, 10);
    if (!isNaN(wd) && !isNaN(h) && !isNaN(m)) {
      return { type: "weekly", weekday: wd, hour: h, minute: m };
    }
  }
  // 毎月パターン: m h d * *
  if (dayPart !== "*" && wdPart === "*") {
    const d = parseInt(dayPart, 10);
    const h = parseInt(hourPart, 10);
    const m = parseInt(minPart, 10);
    if (!isNaN(d) && !isNaN(h) && !isNaN(m)) {
      return { type: "monthly", monthDay: d, hour: h, minute: m };
    }
  }
  return { type: "every_n_minutes", intervalMinutes: 15 };
}

interface Props {
  scriptName: string;
  existingJob?: ScheduledJobPayload;
  onSave: (job: ScheduledJobPayload) => void;
  onCancel: () => void;
}

export function SchedulerJobForm({ scriptName, existingJob, onSave, onCancel }: Props) {
  const initialForm: ScheduleForm = existingJob
    ? cronExprToForm(existingJob.cron_expr)
    : { type: "every_n_minutes", intervalMinutes: 15 };

  const [form, setForm] = useState<ScheduleForm>(initialForm);
  const [jobName, setJobName] = useState(existingJob?.name ?? scriptName);
  const [validationError, setValidationError] = useState("");

  function validate(): boolean {
    if (!jobName.trim()) {
      setValidationError("ジョブ名を入力してください");
      return false;
    }
    if (form.type === "daily" || form.type === "weekly") {
      const h = form.hour ?? 0;
      const m = form.minute ?? 0;
      if (h < 0 || h > 23 || m < 0 || m > 59) {
        setValidationError("時刻が不正です（時: 0-23、分: 0-59）");
        return false;
      }
    }
    if (form.type === "monthly") {
      const d = form.monthDay ?? 1;
      const h = form.hour ?? 0;
      const m = form.minute ?? 0;
      if (d < 1 || d > 28) {
        setValidationError("日付は 1〜28 の範囲で指定してください");
        return false;
      }
      if (h < 0 || h > 23 || m < 0 || m > 59) {
        setValidationError("時刻が不正です（時: 0-23、分: 0-59）");
        return false;
      }
    }
    setValidationError("");
    return true;
  }

  function handleSave() {
    if (!validate()) return;
    const cron_expr = formToCronExpr(form);
    const now = new Date().toISOString();
    const job: ScheduledJobPayload = existingJob
      ? { ...existingJob, name: jobName.trim(), cron_expr }
      : {
          id: crypto.randomUUID(),
          name: jobName.trim(),
          job_type: "Query",
          target_id: scriptName,
          cron_expr,
          enabled: true,
          created_at: now,
          last_run_at: null,
        };
    onSave(job);
  }

  return (
    <div className="flex flex-col gap-3 p-3 text-xs">
      <div className="flex flex-col gap-1">
        <label className="text-muted-foreground font-medium">ジョブ名</label>
        <input
          className="bg-background border rounded px-2 py-1 text-xs outline-none focus:ring-1 focus:ring-primary"
          value={jobName}
          onChange={(e) => setJobName(e.target.value)}
          placeholder="ジョブ名"
        />
      </div>

      <div className="flex flex-col gap-1">
        <label className="text-muted-foreground font-medium">スケジュール種別</label>
        <select
          className="bg-background border rounded px-2 py-1 text-xs outline-none focus:ring-1 focus:ring-primary"
          value={form.type}
          onChange={(e) =>
            setForm({ type: e.target.value as ScheduleType, intervalMinutes: 15 })
          }
        >
          <option value="every_n_minutes">毎 N 分</option>
          <option value="daily">毎日</option>
          <option value="weekly">毎週</option>
          <option value="monthly">毎月</option>
        </select>
      </div>

      {form.type === "every_n_minutes" && (
        <div className="flex flex-col gap-1">
          <label className="text-muted-foreground font-medium">間隔</label>
          <select
            className="bg-background border rounded px-2 py-1 text-xs outline-none focus:ring-1 focus:ring-primary"
            value={form.intervalMinutes ?? 15}
            onChange={(e) =>
              setForm({ ...form, intervalMinutes: parseInt(e.target.value, 10) as 15 | 30 | 60 })
            }
          >
            <option value={15}>15 分ごと</option>
            <option value={30}>30 分ごと</option>
            <option value={60}>60 分ごと（毎時）</option>
          </select>
        </div>
      )}

      {form.type === "daily" && (
        <TimeInput form={form} setForm={setForm} />
      )}

      {form.type === "weekly" && (
        <>
          <div className="flex flex-col gap-1">
            <label className="text-muted-foreground font-medium">曜日</label>
            <select
              className="bg-background border rounded px-2 py-1 text-xs outline-none focus:ring-1 focus:ring-primary"
              value={form.weekday ?? 0}
              onChange={(e) => setForm({ ...form, weekday: parseInt(e.target.value, 10) })}
            >
              {WEEKDAY_LABELS.map((label, idx) => (
                <option key={idx} value={idx}>{label}曜日</option>
              ))}
            </select>
          </div>
          <TimeInput form={form} setForm={setForm} />
        </>
      )}

      {form.type === "monthly" && (
        <>
          <div className="flex flex-col gap-1">
            <label className="text-muted-foreground font-medium">日（1-28）</label>
            <input
              type="number"
              min={1}
              max={28}
              className="bg-background border rounded px-2 py-1 text-xs outline-none focus:ring-1 focus:ring-primary w-20"
              value={form.monthDay ?? 1}
              onChange={(e) => setForm({ ...form, monthDay: parseInt(e.target.value, 10) })}
            />
          </div>
          <TimeInput form={form} setForm={setForm} />
        </>
      )}

      {validationError && (
        <p className="text-destructive text-[11px]">{validationError}</p>
      )}

      <div className="text-muted-foreground text-[11px] break-all">
        cron: <code>{formToCronExpr(form)}</code>
      </div>

      <div className="flex gap-2 justify-end">
        <Button size="sm" variant="ghost" className="h-6 text-xs" onClick={onCancel}>
          キャンセル
        </Button>
        <Button size="sm" className="h-6 text-xs" onClick={handleSave}>
          保存
        </Button>
      </div>
    </div>
  );
}

function TimeInput({
  form,
  setForm,
}: {
  form: ScheduleForm;
  setForm: React.Dispatch<React.SetStateAction<ScheduleForm>>;
}) {
  return (
    <div className="flex flex-col gap-1">
      <label className="text-muted-foreground font-medium">時刻（HH:MM）</label>
      <div className="flex items-center gap-1">
        <input
          type="number"
          min={0}
          max={23}
          className="bg-background border rounded px-2 py-1 text-xs outline-none focus:ring-1 focus:ring-primary w-14"
          value={form.hour ?? 0}
          onChange={(e) => setForm({ ...form, hour: parseInt(e.target.value, 10) })}
        />
        <span>:</span>
        <input
          type="number"
          min={0}
          max={59}
          className="bg-background border rounded px-2 py-1 text-xs outline-none focus:ring-1 focus:ring-primary w-14"
          value={form.minute ?? 0}
          onChange={(e) => setForm({ ...form, minute: parseInt(e.target.value, 10) })}
        />
      </div>
    </div>
  );
}
