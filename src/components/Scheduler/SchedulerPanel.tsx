import { ClockIcon } from "lucide-react";

/** Placeholder UI for the scheduler feature (KHA-10 #1, #3, #4). */
export function SchedulerPanel() {
  return (
    <div className="flex flex-col h-full overflow-hidden select-none">
      <div className="flex items-center gap-1 px-2 py-1 border-b shrink-0">
        <span className="flex items-center gap-1 text-xs font-semibold text-muted-foreground">
          <ClockIcon className="h-3 w-3" />
          SCHEDULER
        </span>
      </div>
      <div className="flex-1 flex items-center justify-center">
        <p className="text-xs text-muted-foreground">Coming soon (KHA-10)</p>
      </div>
    </div>
  );
}
