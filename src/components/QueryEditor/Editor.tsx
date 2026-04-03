import MonacoEditor from "@monaco-editor/react";
import type * as Monaco from "monaco-editor";
import { useRef, useMemo, useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "@/store/appStore";
import { useRunQuery } from "@/hooks/useRunQuery";
import { Button } from "@/components/ui/button";
import { PlayIcon, SaveIcon, Loader2Icon } from "lucide-react";
import { QueryTabBar } from "@/components/QueryEditor/QueryTabBar";

export interface EditorConfig {
  line_numbers: boolean;
  tab_size: number;
  theme: string;
}

interface EditorProps {
  editorConfig: EditorConfig;
}

export function Editor({ editorConfig }: EditorProps) {
  const activeTabId = useAppStore((s) => s.activeTabId);
  const updateTabSql = useAppStore((s) => s.updateTabSql);
  const closeTab = useAppStore((s) => s.closeTab);
  const setTabLinkedScript = useAppStore((s) => s.setTabLinkedScript);
  const setTabLinkedMacro = useAppStore((s) => s.setTabLinkedMacro);
  const activeTab = useAppStore((s) => s.getActiveTab());
  const scripts = useAppStore((s) => s.scripts);
  const setScripts = useAppStore((s) => s.setScripts);
  const isRunning = useAppStore((s) => s.isRunning);
  const saveDialogPending = useAppStore((s) => s.saveDialogPending);
  const setSaveDialogPending = useAppStore((s) => s.setSaveDialogPending);
  const closeConfirmTab = useAppStore((s) => s.closeConfirmTab);
  const setCloseConfirmTab = useAppStore((s) => s.setCloseConfirmTab);

  const sql = activeTab.sql;

  const runQuery = useRunQuery();
  const saveShortcutRef = useRef<() => Promise<void>>(async () => {});
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [saveScriptName, setSaveScriptName] = useState("");
  const [saveError, setSaveError] = useState("");

  useEffect(() => {
    if (saveDialogPending) {
      openSaveDialog();
      setSaveDialogPending(false);
    }
  }, [saveDialogPending]);

  const editorRef = useRef<Monaco.editor.IStandaloneCodeEditor | null>(null);
  const runQueryRef = useRef<() => Promise<void>>(async () => {});

  const monacoOptions = useMemo<Monaco.editor.IStandaloneEditorConstructionOptions>(() => ({
    minimap: { enabled: false },
    fontSize: 13,
    lineNumbers: editorConfig.line_numbers ? "on" : "off",
    tabSize: editorConfig.tab_size,
    scrollBeyondLastLine: false,
    wordWrap: "on",
    automaticLayout: true,
    padding: { top: 8 },
  }), [editorConfig]);

  async function handleRunQuery() {
    await runQuery(sql);
  }

  async function handleSaveShortcut() {
    const tab = useAppStore.getState().getActiveTab();
    if (tab.linkedMacro) {
      try {
        await invoke("save_macro", { name: tab.linkedMacro, content: tab.sql });
        setTabLinkedMacro(tab.id, tab.linkedMacro);
        // ファイル保存後、DuckDB に即時反映
        await invoke("reload_macros");
      } catch (e) { console.error(e); }
    } else if (tab.linkedScript) {
      try {
        await invoke("save_script", { name: tab.linkedScript, content: tab.sql });
        const updated = await invoke<string[]>("list_scripts");
        useAppStore.getState().setScripts(updated);
        setTabLinkedScript(tab.id, tab.linkedScript);
      } catch (e) { console.error(e); }
    } else {
      openSaveDialog();
    }
  }

  runQueryRef.current = handleRunQuery;
  saveShortcutRef.current = handleSaveShortcut;

  function handleEditorMount(
    editor: Monaco.editor.IStandaloneCodeEditor,
    monaco: typeof Monaco
  ) {
    editorRef.current = editor;
    editor.addAction({
      id: "run-query",
      label: "Run Query",
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter],
      run: () => { runQueryRef.current(); },
    });
    editor.addAction({
      id: "save-file",
      label: "Save File",
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS],
      run: () => { saveShortcutRef.current(); },
    });
  }

  function openSaveDialog() {
    const name = activeTab.linkedScript
      ?? `query_${new Date().toISOString().slice(0, 10).replace(/-/g, "")}`;
    setSaveScriptName(name);
    setSaveError("");
    setShowSaveDialog(true);
  }

  async function handleSave() {
    const name = saveScriptName.trim();
    if (!name) {
      setSaveError("名前を入力してください");
      return;
    }
    if (scripts.includes(name) && name !== activeTab.linkedScript) {
      setSaveError(`"${name}" は既に存在します。上書きしますか？`);
      return;
    }
    await doSave(name);
  }

  async function doSave(name: string) {
    try {
      await invoke("save_script", { name, content: sql });
      const updated = await invoke<string[]>("list_scripts");
      setScripts(updated);
      setTabLinkedScript(activeTabId, name);
      setShowSaveDialog(false);
      setSaveError("");
    } catch (e) {
      setSaveError(String(e));
    }
  }

  function handleCloseRequest(tab: { id: string; title: string; isDirty: boolean }) {
    if (tab.isDirty) {
      setCloseConfirmTab({ id: tab.id, title: tab.title });
    } else {
      closeTab(tab.id);
    }
  }

  // Sync Monaco content when switching tabs or when SQL is set externally (e.g. double-click in explorer)
  useEffect(() => {
    const editor = editorRef.current;
    if (editor && editor.getValue() !== sql) {
      editor.setValue(sql);
    }
  }, [activeTabId, sql]);

  return (
    <div className="flex flex-col h-full relative">
      {/* Tab bar */}
      <QueryTabBar onCloseRequest={handleCloseRequest} />

      {/* Toolbar */}
      <div className="flex items-center gap-2 border-b px-2 py-1">
        <span className="text-xs font-semibold text-muted-foreground">QUERY</span>
        {activeTab.linkedScript && (
          <span className="text-[10px] text-muted-foreground font-mono truncate max-w-[160px]" title={activeTab.linkedScript}>
            {activeTab.linkedScript}
          </span>
        )}
        <div className="flex-1" />
        {activeTab.linkedMacro && (
          <Button
            size="sm"
            variant="ghost"
            className="h-6 text-xs gap-1"
            onClick={handleSaveShortcut}
            title={`Save macro "${activeTab.linkedMacro}" (⌘S)`}
          >
            <SaveIcon className="h-3 w-3" />
            Save Macro
            <span className="text-muted-foreground text-[10px] ml-1">⌘S</span>
          </Button>
        )}
        <Button
          size="sm"
          variant="ghost"
          className="h-6 text-xs gap-1"
          onClick={openSaveDialog}
          title="Save as script (⌘S)"
        >
          <SaveIcon className="h-3 w-3" />
          Save
          <span className="text-muted-foreground text-[10px] ml-1">⌘S</span>
        </Button>
        <Button size="sm" onClick={handleRunQuery} disabled={isRunning} className={`h-6 text-xs gap-1 ${isRunning ? "opacity-80" : ""}`}>
          {isRunning
            ? <Loader2Icon className="h-3 w-3 animate-spin" />
            : <PlayIcon className="h-3 w-3" />}
          {isRunning ? "Running..." : "Run"}
          {!isRunning && <span className="text-muted-foreground text-[10px] ml-1">⌘↵</span>}
        </Button>
      </div>

      <div className="flex-1 overflow-hidden">
        <MonacoEditor
          height="100%"
          language="sql"
          value={sql}
          onChange={(v) => updateTabSql(activeTabId, v ?? "")}
          onMount={handleEditorMount}
          theme={editorConfig.theme === "light" ? "vs" : "vs-dark"}
          options={monacoOptions}
        />
      </div>

      {/* Save dialog */}
      {showSaveDialog && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
          <div className="bg-popover border rounded-lg shadow-xl p-4 w-80 text-sm">
            <p className="font-semibold mb-3">スクリプトとして保存</p>
            <input
              autoFocus
              className="w-full border rounded px-2 py-1 text-xs bg-background mb-1"
              placeholder="script name"
              value={saveScriptName}
              onChange={(e) => { setSaveScriptName(e.target.value); setSaveError(""); }}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleSave();
                if (e.key === "Escape") setShowSaveDialog(false);
              }}
            />
            <p className="text-[10px] text-muted-foreground mb-3">
              ~/.tdwh/scripts/<span className="font-mono">{saveScriptName || "…"}</span>.sql
            </p>
            {saveError && (
              <p className="text-xs text-destructive mb-2">{saveError}</p>
            )}
            <div className="flex justify-end gap-2">
              <Button size="sm" variant="ghost" onClick={() => setShowSaveDialog(false)}>
                Cancel
              </Button>
              <Button
                size="sm"
                onClick={() => {
                  if (saveError && saveScriptName.trim()) {
                    doSave(saveScriptName.trim());
                  } else {
                    handleSave();
                  }
                }}
              >
                Save
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Close tab confirmation */}
      {closeConfirmTab && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
          <div className="bg-popover border rounded-lg shadow-xl p-4 w-72 text-sm">
            <p className="font-semibold mb-1">タブを閉じますか？</p>
            <p className="text-xs text-muted-foreground mb-4">
              「{closeConfirmTab.title}」には未保存の変更があります。
            </p>
            <div className="flex justify-end gap-2">
              <Button size="sm" variant="ghost" onClick={() => setCloseConfirmTab(null)}>
                キャンセル
              </Button>
              <Button
                size="sm"
                variant="destructive"
                onClick={() => {
                  closeTab(closeConfirmTab.id);
                  setCloseConfirmTab(null);
                }}
              >
                閉じる
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
