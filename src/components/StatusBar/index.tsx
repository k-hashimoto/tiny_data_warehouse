import { useState } from "react";
import { useAppStore } from "@/store/appStore";
import { DatabaseIcon } from "lucide-react";
import { AboutModal } from "@/components/About/AboutModal";

export function StatusBar() {
  const { status, dbPath, result } = useAppStore();
  const [showAbout, setShowAbout] = useState(false);

  return (
    <>
      <div className="flex items-center gap-4 border-t px-4 py-0.5 text-xs text-muted-foreground bg-muted/30 shrink-0">
        <span>{status}</span>
        {result && (
          <>
            <span className="text-border">|</span>
            <span>{result.row_count.toLocaleString()} rows</span>
            <span className="text-border">|</span>
            <span>{result.elapsed_ms}ms</span>
          </>
        )}
        <div className="flex-1" />
        <div className="flex items-center gap-1">
          <DatabaseIcon className="h-3 w-3" />
          <span>{dbPath}</span>
        </div>
        <span className="text-border">|</span>
        <button
          className="opacity-50 hover:opacity-100 transition-opacity"
          onClick={() => setShowAbout(true)}
        >
          About
        </button>
      </div>

      {showAbout && <AboutModal onClose={() => setShowAbout(false)} />}
    </>
  );
}
