import { useAppStore } from "@/store/appStore";
import { Button } from "@/components/ui/button";
import { XIcon } from "lucide-react";

interface QueryHistoryProps {
  onSelect: (sql: string) => void;
  onClose: () => void;
}

export function QueryHistory({ onSelect, onClose }: QueryHistoryProps) {
  const queryHistory = useAppStore((s) => s.queryHistory);

  if (queryHistory.length === 0) {
    return (
      <div className="absolute inset-0 z-20 bg-background border-b flex flex-col">
        <div className="flex items-center gap-2 px-2 py-1 border-b">
          <span className="text-xs font-semibold text-muted-foreground flex-1">QUERY HISTORY</span>
          <Button size="sm" variant="ghost" className="h-5 w-5 p-0" onClick={onClose}>
            <XIcon className="h-3 w-3" />
          </Button>
        </div>
        <div className="p-3 text-sm text-muted-foreground">No history yet.</div>
      </div>
    );
  }

  return (
    <div className="absolute inset-0 z-20 bg-background border-b flex flex-col">
      <div className="flex items-center gap-2 px-2 py-1 border-b shrink-0">
        <span className="text-xs font-semibold text-muted-foreground flex-1">
          QUERY HISTORY ({queryHistory.length})
        </span>
        <Button size="sm" variant="ghost" className="h-5 w-5 p-0" onClick={onClose}>
          <XIcon className="h-3 w-3" />
        </Button>
      </div>
      <div className="flex-1 overflow-auto">
        {queryHistory.map((sql, i) => (
          <button
            key={i}
            className="w-full text-left px-3 py-2 border-b text-xs font-mono hover:bg-accent/40 truncate block"
            onClick={() => { onSelect(sql); onClose(); }}
            title={sql}
          >
            {sql}
          </button>
        ))}
      </div>
    </div>
  );
}
