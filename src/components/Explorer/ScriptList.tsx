import { useState, useEffect, useMemo, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "@/store/appStore";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import {
  FileCodeIcon, PlayIcon, RefreshCwIcon, Trash2Icon, ScrollIcon,
  FolderIcon, FolderOpenIcon, ChevronRightIcon, ChevronDownIcon,
  SaveIcon, PencilIcon,
} from "lucide-react";
import { useRunQuery } from "@/hooks/useRunQuery";

interface ContextMenu {
  x: number;
  y: number;
  name: string;
}

export function ScriptList() {
  const scripts = useAppStore((s) => s.scripts);
  const setScripts = useAppStore((s) => s.setScripts);
  const sql = useAppStore((s) => s.sql);
  const tabs = useAppStore((s) => s.tabs);
  const activeTabId = useAppStore((s) => s.activeTabId);
  const addTab = useAppStore((s) => s.addTab);
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  const setTabLinkedScript = useAppStore((s) => s.setTabLinkedScript);
  const renameTab = useAppStore((s) => s.renameTab);
  const runQuery = useRunQuery();

  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());
  const [contextMenu, setContextMenu] = useState<ContextMenu | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);
  const [renamingScript, setRenamingScript] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [renameError, setRenameError] = useState("");
  const renameInputRef = useRef<HTMLInputElement>(null);
  const contextMenuRef = useRef<HTMLDivElement>(null);

  async function refresh() {
    try {
      const names = await invoke<string[]>("list_scripts");
      setScripts(names);
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

  useEffect(() => {
    if (renamingScript) {
      renameInputRef.current?.focus();
      renameInputRef.current?.select();
    }
  }, [renamingScript]);

  // Group scripts: folder → [fullPath, ...], null → root scripts
  const { rootScripts, folders, folderScripts } = useMemo(() => {
    const root: string[] = [];
    const folderMap = new Map<string, string[]>();
    for (const name of scripts) {
      const slash = name.indexOf("/");
      if (slash === -1) {
        root.push(name);
      } else {
        const folder = name.substring(0, slash);
        if (!folderMap.has(folder)) folderMap.set(folder, []);
        folderMap.get(folder)!.push(name);
      }
    }
    const sortedFolders = [...folderMap.keys()].sort();
    return { rootScripts: root, folders: sortedFolders, folderScripts: folderMap };
  }, [scripts]);

  function toggleFolder(folder: string) {
    setExpandedFolders((prev) => {
      const next = new Set(prev);
      if (next.has(folder)) next.delete(folder); else next.add(folder);
      return next;
    });
  }

  function displayName(fullPath: string) {
    const idx = fullPath.lastIndexOf("/");
    return idx === -1 ? fullPath : fullPath.substring(idx + 1);
  }

  async function loadScript(name: string) {
    try {
      // すでに同じスクリプトが開かれているタブがあればそれをアクティブにする
      const existingTab = tabs.find((t) => t.linkedScript === name);
      if (existingTab) {
        setActiveTab(existingTab.id);
        return;
      }
      const content = await invoke<string>("read_script", { name });
      addTab();
      const { tabs: updatedTabs } = useAppStore.getState();
      const newTab = updatedTabs[updatedTabs.length - 1];
      const tabTitle = name.includes("/") ? name.substring(name.lastIndexOf("/") + 1) : name;
      renameTab(newTab.id, tabTitle);
      setTabLinkedScript(newTab.id, name, content);
      setActiveTab(newTab.id);
    } catch (e) {
      console.error(e);
    }
  }

  async function runScript(name: string) {
    try {
      const content = await invoke<string>("read_script", { name });
      await runQuery(content);
    } catch (e) {
      console.error(e);
    }
  }

  async function overwriteScript(name: string) {
    try {
      await invoke("save_script", { name, content: sql });
      // 紐づいているタブのダーティフラグをリセット
      const linkedTab = tabs.find((t) => t.linkedScript === name);
      if (linkedTab) setTabLinkedScript(linkedTab.id, name);
    } catch (e) {
      console.error(e);
    }
  }

  async function confirmDelete() {
    if (!deleteConfirm) return;
    try {
      await invoke("delete_script", { name: deleteConfirm });
      await refresh();
    } catch (e) {
      console.error(e);
    } finally {
      setDeleteConfirm(null);
    }
  }

  function startRename(name: string) {
    setRenamingScript(name);
    setRenameValue(displayName(name));
    setRenameError("");
    setContextMenu(null);
  }

  async function commitRename() {
    if (!renamingScript) return;
    const newDisplay = renameValue.trim();
    if (!newDisplay) { setRenameError("名前を入力してください"); return; }

    // Preserve folder prefix if any
    const slash = renamingScript.lastIndexOf("/");
    const newName = slash === -1 ? newDisplay : `${renamingScript.substring(0, slash + 1)}${newDisplay}`;

    if (newName === renamingScript) { setRenamingScript(null); return; }

    try {
      await invoke("rename_script", { oldName: renamingScript, newName });
      // タブの linkedScript も更新
      const linkedTab = tabs.find((t) => t.linkedScript === renamingScript);
      if (linkedTab) setTabLinkedScript(linkedTab.id, newName);
      await refresh();
      setRenamingScript(null);
    } catch (e) {
      setRenameError(String(e));
    }
  }

  function ScriptItem({ name }: { name: string }) {
    const isActive = tabs.find((t) => t.id === activeTabId)?.linkedScript === name;
    const isRenaming = renamingScript === name;

    return (
      <div
        className={`flex items-center gap-1 px-2 py-0.5 hover:bg-accent/40 group ${isActive ? "bg-accent/20" : ""}`}
        onContextMenu={(e) => {
          e.preventDefault();
          setContextMenu({ x: e.clientX, y: e.clientY, name });
        }}
      >
        <FileCodeIcon className={`h-3 w-3 shrink-0 ${isActive ? "text-primary" : "text-muted-foreground"}`} />

        {isRenaming ? (
          <div className="flex-1 flex flex-col min-w-0">
            <input
              ref={renameInputRef}
              className="text-xs bg-background border rounded px-1 py-0.5 outline-none focus:ring-1 focus:ring-primary w-full"
              value={renameValue}
              onChange={(e) => { setRenameValue(e.target.value); setRenameError(""); }}
              onKeyDown={(e) => {
                if (e.key === "Enter") commitRename();
                if (e.key === "Escape") setRenamingScript(null);
              }}
              onBlur={commitRename}
            />
            {renameError && <span className="text-[10px] text-destructive">{renameError}</span>}
          </div>
        ) : (
          <button
            className={`flex-1 text-left text-xs truncate ${isActive ? "text-foreground font-medium" : ""}`}
            onClick={() => loadScript(name)}
            onDoubleClick={() => startRename(name)}
            title={`Load "${name}" into editor (double-click to rename)`}
          >
            {displayName(name)}
          </button>
        )}

        {/* 上書き保存ボタン（アクティブスクリプトのみ常時表示、他はホバー時） */}
        {!isRenaming && (
          <Button
            size="icon"
            variant="ghost"
            className={`h-5 w-5 ${isActive ? "opacity-70 hover:opacity-100" : "opacity-0 group-hover:opacity-100"}`}
            title="上書き保存（現在のエディタ内容で保存）"
            onClick={() => overwriteScript(name)}
          >
            <SaveIcon className="h-3 w-3" />
          </Button>
        )}

        {!isRenaming && (
          <Button
            size="icon"
            variant="ghost"
            className="h-5 w-5 opacity-0 group-hover:opacity-100"
            title="Run script"
            onClick={() => runScript(name)}
          >
            <PlayIcon className="h-3 w-3" />
          </Button>
        )}
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full overflow-hidden select-none">
      {/* Header */}
      <div className="flex items-center gap-1 px-2 py-1 border-b shrink-0">
        <span className="flex items-center gap-1 text-xs font-semibold text-muted-foreground flex-1">
          <ScrollIcon className="h-3 w-3" />
          SAVED QUERIES
        </span>
        <Button size="icon" variant="ghost" className="h-5 w-5" title="Refresh" onClick={refresh}>
          <RefreshCwIcon className="h-3 w-3" />
        </Button>
      </div>

      {/* List */}
      <div className="flex-1 overflow-y-auto">
        {scripts.length === 0 && (
          <p className="text-xs text-muted-foreground px-3 py-2">No scripts saved</p>
        )}

        {/* Root-level scripts */}
        {rootScripts.map((name) => <ScriptItem key={name} name={name} />)}

        {/* Folders */}
        {folders.map((folder) => {
          const isExpanded = expandedFolders.has(folder);
          const items = folderScripts.get(folder) ?? [];
          return (
            <div key={folder}>
              <div
                className="flex items-center gap-1 px-1 py-0.5 hover:bg-accent/40 cursor-pointer"
                onClick={() => toggleFolder(folder)}
              >
                {isExpanded
                  ? <ChevronDownIcon className="h-3 w-3 text-muted-foreground shrink-0" />
                  : <ChevronRightIcon className="h-3 w-3 text-muted-foreground shrink-0" />}
                {isExpanded
                  ? <FolderOpenIcon className="h-3 w-3 text-yellow-500 shrink-0" />
                  : <FolderIcon className="h-3 w-3 text-yellow-500 shrink-0" />}
                <span className="text-xs font-semibold text-muted-foreground">{folder}</span>
                <span className="ml-auto text-[10px] text-muted-foreground pr-1">{items.length}</span>
              </div>
              {isExpanded && items.map((name) => (
                <div key={name} className="ml-3">
                  <ScriptItem name={name} />
                </div>
              ))}
            </div>
          );
        })}
      </div>

      {/* Context menu */}
      {contextMenu && (
        <div
          ref={contextMenuRef}
          className="fixed z-50 bg-popover border rounded shadow-md py-1 min-w-[140px] text-xs"
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          <button
            className="flex items-center gap-2 w-full px-3 py-1.5 hover:bg-accent"
            onClick={() => overwriteScript(contextMenu.name).then(() => setContextMenu(null))}
          >
            <SaveIcon className="h-3 w-3" />
            上書き保存
          </button>
          <button
            className="flex items-center gap-2 w-full px-3 py-1.5 hover:bg-accent"
            onClick={() => startRename(contextMenu.name)}
          >
            <PencilIcon className="h-3 w-3" />
            名前を変更
          </button>
          <div className="border-t my-1" />
          <button
            className="flex items-center gap-2 w-full px-3 py-1.5 hover:bg-accent text-destructive"
            onClick={() => {
              setDeleteConfirm(contextMenu.name);
              setContextMenu(null);
            }}
          >
            <Trash2Icon className="h-3 w-3" />
            Delete
          </button>
        </div>
      )}

      {deleteConfirm && (
        <ConfirmDialog
          title="Delete script?"
          description={`${deleteConfirm}.sql を削除します。`}
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
