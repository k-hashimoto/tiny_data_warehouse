import { useState, useRef, useEffect } from "react";
import { useAppStore, Tab } from "@/store/appStore";
import { PlusIcon, XIcon, ClockIcon } from "lucide-react";

export function QueryTabBar({ onCloseRequest }: { onCloseRequest: (tab: Tab) => void }) {
  const tabs = useAppStore((s) => s.tabs);
  const activeTabId = useAppStore((s) => s.activeTabId);
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  const addTab = useAppStore((s) => s.addTab);
  const renameTab = useAppStore((s) => s.renameTab);
  const historyOpen = useAppStore((s) => s.historyOpen);
  const setHistoryOpen = useAppStore((s) => s.setHistoryOpen);
  const setSchedulerOpen = useAppStore((s) => s.setSchedulerOpen);

  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const renameInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (renamingId) renameInputRef.current?.select();
  }, [renamingId]);

  function startRename(tab: Tab) {
    setRenamingId(tab.id);
    setRenameValue(tab.title);
  }

  function commitRename() {
    if (!renamingId) return;
    const trimmed = renameValue.trim();
    if (trimmed) renameTab(renamingId, trimmed);
    setRenamingId(null);
  }

  return (
    <div className="flex items-stretch border-b bg-muted/30 overflow-x-auto shrink-0 select-none" style={{ minHeight: 30 }}>
      {tabs.map((tab) => {
        const isActive = tab.id === activeTabId;
        const isRenaming = renamingId === tab.id;

        return (
          <div
            key={tab.id}
            className={`flex items-center gap-1 px-2 border-r cursor-pointer shrink-0 max-w-[160px] group
              ${isActive
                ? "bg-background border-b-background border-b-2 -mb-px"
                : "hover:bg-muted/60 text-muted-foreground"}`}
            onClick={() => setActiveTab(tab.id)}
            onDoubleClick={() => !isRenaming && startRename(tab)}
          >
            {/* Dirty indicator */}
            {tab.isDirty && (
              <span className="h-1.5 w-1.5 rounded-full bg-orange-400 shrink-0" title="未保存の変更あり" />
            )}

            {isRenaming ? (
              <input
                ref={renameInputRef}
                className="text-xs bg-background border rounded px-1 py-0 outline-none focus:ring-1 focus:ring-primary w-24"
                value={renameValue}
                onChange={(e) => setRenameValue(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") commitRename();
                  if (e.key === "Escape") setRenamingId(null);
                }}
                onBlur={commitRename}
                onClick={(e) => e.stopPropagation()}
              />
            ) : (
              <span className="text-xs truncate max-w-[110px]" title={tab.title}>
                {tab.title}
              </span>
            )}

            {/* Close button */}
            <button
              className={`shrink-0 rounded p-0.5 hover:bg-accent
                ${isActive ? "opacity-50 hover:opacity-100" : "opacity-0 group-hover:opacity-50 hover:!opacity-100"}`}
              onClick={(e) => {
                e.stopPropagation();
                onCloseRequest(tab);
              }}
              title="タブを閉じる (⌘W)"
            >
              <XIcon className="h-2.5 w-2.5" />
            </button>
          </div>
        );
      })}

      {/* Add tab button */}
      <button
        className="flex items-center justify-center px-2 text-muted-foreground hover:text-foreground hover:bg-muted/60 shrink-0 disabled:opacity-30 disabled:cursor-not-allowed"
        onClick={addTab}
        disabled={tabs.length >= 10}
        title={tabs.length >= 10 ? "タブは最大10枚まで" : "新しいタブ (⌘T)"}
      >
        <PlusIcon className="h-3.5 w-3.5" />
      </button>

      {/* Spacer */}
      <div className="flex-1" />

      {/* History button */}
      <button
        className={`flex items-center gap-1 px-2 text-xs border-l shrink-0 ${
          historyOpen
            ? "bg-background text-foreground"
            : "text-muted-foreground hover:text-foreground hover:bg-muted/60"
        }`}
        onClick={() => { setHistoryOpen(!historyOpen); setSchedulerOpen(false); }}
        title="Query History"
      >
        <ClockIcon className="h-3 w-3" />
        History
      </button>
    </div>
  );
}
