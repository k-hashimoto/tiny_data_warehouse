import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "@/store/appStore";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import {
  FunctionSquareIcon, RefreshCwIcon, Trash2Icon,
  ChevronRightIcon, ChevronDownIcon, PlusIcon,
} from "lucide-react";

interface Props {
  isCollapsed: boolean;
  onToggleCollapse: () => void;
}

const UDF_TEMPLATE = `-- UDF名に合わせてマクロ名を変更してください
CREATE OR REPLACE MACRO my_func(x) AS (x);
`;

export function UdfList({ isCollapsed, onToggleCollapse }: Props) {
  const udfs = useAppStore((s) => s.udfs);
  const setUdfs = useAppStore((s) => s.setUdfs);
  const tabs = useAppStore((s) => s.tabs);
  const activeTabId = useAppStore((s) => s.activeTabId);
  const addTab = useAppStore((s) => s.addTab);
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  const setTabLinkedUdf = useAppStore((s) => s.setTabLinkedUdf);
  const renameTab = useAppStore((s) => s.renameTab);

  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; name: string } | null>(null);
  const contextMenuRef = useRef<HTMLDivElement>(null);

  async function refresh() {
    try {
      const names = await invoke<string[]>("list_udfs");
      setUdfs(names);
    } catch (_) {}
  }

  useEffect(() => {
    refresh();
  }, []);

  useEffect(() => {
    if (!contextMenu) return;
    function handleClick(e: MouseEvent) {
      if (contextMenuRef.current && !contextMenuRef.current.contains(e.target as Node)) {
        setContextMenu(null);
      }
    }
    window.addEventListener("click", handleClick);
    return () => window.removeEventListener("click", handleClick);
  }, [contextMenu]);

  async function openUdf(name: string) {
    try {
      const existingTab = tabs.find((t) => t.linkedUdf === name);
      if (existingTab) {
        setActiveTab(existingTab.id);
        return;
      }
      const content = await invoke<string>("read_udf", { name });
      addTab();
      const { tabs: updatedTabs } = useAppStore.getState();
      const newTab = updatedTabs[updatedTabs.length - 1];
      renameTab(newTab.id, name);
      setTabLinkedUdf(newTab.id, name, content);
      setActiveTab(newTab.id);
    } catch (e) {
      console.error(e);
    }
  }

  async function createNewUdf() {
    addTab();
    const { tabs: updatedTabs } = useAppStore.getState();
    const newTab = updatedTabs[updatedTabs.length - 1];
    renameTab(newTab.id, "new_udf");
    setTabLinkedUdf(newTab.id, "new_udf", UDF_TEMPLATE);
    setActiveTab(newTab.id);
  }

  async function confirmDelete() {
    if (!deleteConfirm) return;
    try {
      await invoke("delete_udf", { name: deleteConfirm });
      await refresh();
    } catch (e) {
      console.error(e);
    } finally {
      setDeleteConfirm(null);
    }
  }

  const isUdfTabActive = (name: string) =>
    tabs.find((t) => t.id === activeTabId)?.linkedUdf === name;

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
          UDFs
        </button>
        <Button size="icon" variant="ghost" className="h-5 w-5" title="新規UDF" onClick={createNewUdf}>
          <PlusIcon className="h-3 w-3" />
        </Button>
        <Button size="icon" variant="ghost" className="h-5 w-5" title="Refresh" onClick={refresh}>
          <RefreshCwIcon className="h-3 w-3" />
        </Button>
      </div>

      {/* List */}
      {!isCollapsed && (
        <div className="flex-1 overflow-y-auto">
          {udfs.length === 0 && (
            <p className="text-xs text-muted-foreground px-3 py-2">No UDFs saved</p>
          )}
          {udfs.map((name) => {
            const isActive = isUdfTabActive(name);
            return (
              <div
                key={name}
                className={`flex items-center gap-1 px-2 py-0.5 hover:bg-accent/40 group ${isActive ? "bg-accent/20" : ""}`}
                onContextMenu={(e) => {
                  e.preventDefault();
                  setContextMenu({ x: e.clientX, y: e.clientY, name });
                }}
              >
                <FunctionSquareIcon className={`h-3 w-3 shrink-0 ${isActive ? "text-primary" : "text-muted-foreground"}`} />
                <button
                  className={`flex-1 text-left text-xs truncate ${isActive ? "text-foreground font-medium" : ""}`}
                  onClick={() => openUdf(name)}
                  title={name}
                >
                  {name}
                </button>
                <Button
                  size="icon"
                  variant="ghost"
                  className="h-5 w-5 opacity-0 group-hover:opacity-100"
                  title="削除"
                  onClick={() => setDeleteConfirm(name)}
                >
                  <Trash2Icon className="h-3 w-3" />
                </Button>
              </div>
            );
          })}
        </div>
      )}

      {/* Context menu */}
      {contextMenu && (
        <div
          ref={contextMenuRef}
          className="fixed z-50 bg-popover border rounded shadow-md py-1 min-w-[140px] text-xs"
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          <button
            className="flex items-center gap-2 w-full px-3 py-1.5 hover:bg-accent"
            onClick={() => { openUdf(contextMenu.name); setContextMenu(null); }}
          >
            <FunctionSquareIcon className="h-3 w-3" />
            エディタで開く
          </button>
          <div className="border-t my-1" />
          <button
            className="flex items-center gap-2 w-full px-3 py-1.5 hover:bg-accent text-destructive"
            onClick={() => { setDeleteConfirm(contextMenu.name); setContextMenu(null); }}
          >
            <Trash2Icon className="h-3 w-3" />
            Delete
          </button>
        </div>
      )}

      {deleteConfirm && (
        <ConfirmDialog
          title="UDFを削除"
          description={`"${deleteConfirm}" を削除します。DuckDB 内のマクロ定義は残ります（手動で DROP MACRO が必要です）。`}
          confirmLabel="Delete"
          cancelLabel="Cancel"
          destructive
          onConfirm={confirmDelete}
          onCancel={() => setDeleteConfirm(null)}
        />
      )}
    </div>
  );
}
