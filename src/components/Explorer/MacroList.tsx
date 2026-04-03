import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "@/store/appStore";
import { Button } from "@/components/ui/button";
import {
  FileCodeIcon, RefreshCwIcon, SaveIcon,
  ChevronRightIcon, ChevronDownIcon, FunctionSquareIcon,
  RotateCcwIcon, CheckCircleIcon, XCircleIcon, PlusIcon,
} from "lucide-react";

interface Props {
  isCollapsed: boolean;
  onToggleCollapse: () => void;
}

type ReloadStatus = { ok: true; count: number } | { ok: false; msg: string } | null;

export function MacroList({ isCollapsed, onToggleCollapse }: Props) {
  const macros = useAppStore((s) => s.macros);
  const setMacros = useAppStore((s) => s.setMacros);
  const tabs = useAppStore((s) => s.tabs);
  const activeTabId = useAppStore((s) => s.activeTabId);
  const addTab = useAppStore((s) => s.addTab);
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  const setTabLinkedMacro = useAppStore((s) => s.setTabLinkedMacro);
  const renameTab = useAppStore((s) => s.renameTab);

  const [reloadStatus, setReloadStatus] = useState<ReloadStatus>(null);
  const [isReloading, setIsReloading] = useState(false);

  // New macro dialog state
  const [showNewDialog, setShowNewDialog] = useState(false);
  const [newName, setNewName] = useState("");
  const [newError, setNewError] = useState("");
  const newInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (showNewDialog) {
      setTimeout(() => newInputRef.current?.focus(), 0);
    }
  }, [showNewDialog]);

  async function refresh() {
    try {
      const names = await invoke<string[]>("list_macros");
      setMacros(names);
    } catch (_) {}
  }

  useEffect(() => {
    refresh();
  }, []);

  async function openMacro(name: string) {
    try {
      const existingTab = tabs.find((t) => t.linkedMacro === name);
      if (existingTab) {
        setActiveTab(existingTab.id);
        return;
      }
      const content = await invoke<string>("read_macro", { name });
      addTab();
      const { tabs: updatedTabs } = useAppStore.getState();
      const newTab = updatedTabs[updatedTabs.length - 1];
      renameTab(newTab.id, name);
      setTabLinkedMacro(newTab.id, name, content);
      setActiveTab(newTab.id);
    } catch (e) {
      console.error(e);
    }
  }

  async function saveMacro(name: string) {
    try {
      const linkedTab = useAppStore.getState().tabs.find((t) => t.linkedMacro === name);
      if (!linkedTab) return;
      await invoke("save_macro", { name, content: linkedTab.sql });
      setTabLinkedMacro(linkedTab.id, name);
      // ファイル保存後、DuckDB に即時反映
      await invoke("reload_macros");
    } catch (e) {
      console.error(e);
    }
  }

  async function handleReload() {
    setIsReloading(true);
    setReloadStatus(null);
    try {
      const loaded = await invoke<string[]>("reload_macros");
      setReloadStatus({ ok: true, count: loaded.length });
    } catch (e) {
      setReloadStatus({ ok: false, msg: String(e) });
    } finally {
      setIsReloading(false);
      setTimeout(() => setReloadStatus(null), 4000);
    }
  }

  async function handleCreateMacro() {
    const name = newName.trim();
    if (!name) { setNewError("名前を入力してください"); return; }
    if (macros.includes(name)) { setNewError(`"${name}" は既に存在します`); return; }
    try {
      const template = `-- ${name}.sql\nCREATE OR REPLACE MACRO ${name}() AS (\n  -- implement here\n  NULL\n);\n`;
      await invoke("save_macro", { name, content: template });
      await refresh();
      setShowNewDialog(false);
      setNewName("");
      // Open the new file in editor
      await openMacro(name);
    } catch (e) {
      setNewError(String(e));
    }
  }

  const activeTabMacro = tabs.find((t) => t.id === activeTabId)?.linkedMacro ?? null;

  return (
    <div className="flex flex-col h-full overflow-hidden select-none">
      {/* Header */}
      <div className="flex items-center gap-1 px-2 py-1 border-b shrink-0">
        <button
          className="flex items-center gap-1 text-xs font-semibold text-muted-foreground hover:text-foreground flex-1"
          onClick={onToggleCollapse}
        >
          {isCollapsed
            ? <ChevronRightIcon className="h-3 w-3" />
            : <ChevronDownIcon className="h-3 w-3" />}
          <FunctionSquareIcon className="h-3 w-3" />
          MACROS
        </button>
        <Button
          size="icon"
          variant="ghost"
          className="h-5 w-5"
          title="新規マクロファイルを作成"
          onClick={() => { setShowNewDialog(true); setNewName(""); setNewError(""); }}
        >
          <PlusIcon className="h-3 w-3" />
        </Button>
        <Button
          size="icon"
          variant="ghost"
          className="h-5 w-5"
          title="Reload macros into DuckDB"
          onClick={handleReload}
          disabled={isReloading}
        >
          <RotateCcwIcon className={`h-3 w-3 ${isReloading ? "animate-spin" : ""}`} />
        </Button>
        <Button size="icon" variant="ghost" className="h-5 w-5" title="Refresh list" onClick={refresh}>
          <RefreshCwIcon className="h-3 w-3" />
        </Button>
      </div>

      {/* Reload status */}
      {reloadStatus && (
        <div className={`flex items-center gap-1 px-2 py-0.5 text-[10px] shrink-0 ${reloadStatus.ok ? "text-green-500" : "text-destructive"}`}>
          {reloadStatus.ok
            ? <><CheckCircleIcon className="h-3 w-3" />Loaded {reloadStatus.count} macro{reloadStatus.count !== 1 ? "s" : ""}</>
            : <><XCircleIcon className="h-3 w-3" /><span className="truncate" title={reloadStatus.msg}>{reloadStatus.msg}</span></>
          }
        </div>
      )}

      {/* List */}
      <div className="flex-1 overflow-y-auto">
        {macros.length === 0 && (
          <p className="text-xs text-muted-foreground px-3 py-2">
            + ボタンで新規作成
          </p>
        )}
        {macros.map((name) => {
          const isActive = activeTabMacro === name;
          const isOpen = tabs.some((t) => t.linkedMacro === name);
          return (
            <div
              key={name}
              className={`flex items-center gap-1 px-2 py-0.5 hover:bg-accent/40 group ${isActive ? "bg-accent/20" : ""}`}
            >
              <FileCodeIcon className={`h-3 w-3 shrink-0 ${isActive ? "text-primary" : "text-muted-foreground"}`} />
              <button
                className={`flex-1 text-left text-xs truncate ${isActive ? "text-foreground font-medium" : ""}`}
                onClick={() => openMacro(name)}
                title={`Open macro "${name}" in editor`}
              >
                {name}
              </button>
              {isOpen && (
                <Button
                  size="icon"
                  variant="ghost"
                  className={`h-5 w-5 ${isActive ? "opacity-70 hover:opacity-100" : "opacity-0 group-hover:opacity-100"}`}
                  title="上書き保存（エディタの内容でファイルを保存）"
                  onClick={() => saveMacro(name)}
                >
                  <SaveIcon className="h-3 w-3" />
                </Button>
              )}
            </div>
          );
        })}
      </div>

      {/* New macro dialog */}
      {showNewDialog && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
          <div className="bg-popover border rounded-lg shadow-xl p-4 w-72 text-sm">
            <p className="font-semibold mb-3">新規マクロファイル</p>
            <input
              ref={newInputRef}
              className="w-full border rounded px-2 py-1 text-xs bg-background mb-1"
              placeholder="macro_name"
              value={newName}
              onChange={(e) => { setNewName(e.target.value); setNewError(""); }}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleCreateMacro();
                if (e.key === "Escape") setShowNewDialog(false);
              }}
            />
            <p className="text-[10px] text-muted-foreground mb-3">
              ~/.tdwh/macros/<span className="font-mono">{newName || "…"}</span>.sql
            </p>
            {newError && <p className="text-xs text-destructive mb-2">{newError}</p>}
            <div className="flex justify-end gap-2">
              <Button size="sm" variant="ghost" onClick={() => setShowNewDialog(false)}>Cancel</Button>
              <Button size="sm" onClick={handleCreateMacro}>作成</Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
