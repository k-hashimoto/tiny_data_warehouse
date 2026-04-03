import MonacoEditor from "@monaco-editor/react";
import type * as Monaco from "monaco-editor";
import { useRef, useMemo, useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "@/store/appStore";
import { useRunQuery } from "@/hooks/useRunQuery";
import { Button } from "@/components/ui/button";
import { PlayIcon, ClockIcon, SaveIcon, SunIcon, MoonIcon, Loader2Icon, FunctionSquareIcon } from "lucide-react";
import { QueryHistory } from "@/components/QueryHistory/QueryHistory";
import { QueryTabBar } from "@/components/QueryEditor/QueryTabBar";

interface EditorConfig {
  line_numbers: boolean;
  tab_size: number;
  theme: string;
}

const defaultEditorConfig: EditorConfig = { line_numbers: true, tab_size: 4, theme: "dark" };

export function Editor() {
  const activeTabId = useAppStore((s) => s.activeTabId);
  const updateTabSql = useAppStore((s) => s.updateTabSql);
  const closeTab = useAppStore((s) => s.closeTab);
  const setTabLinkedScript = useAppStore((s) => s.setTabLinkedScript);
  const activeTab = useAppStore((s) => s.getActiveTab());
  const scripts = useAppStore((s) => s.scripts);
  const setScripts = useAppStore((s) => s.setScripts);
  const setUdfs = useAppStore((s) => s.setUdfs);
  const isRunning = useAppStore((s) => s.isRunning);
  const historyOpen = useAppStore((s) => s.historyOpen);
  const setHistoryOpen = useAppStore((s) => s.setHistoryOpen);
  const setDarkMode = useAppStore((s) => s.setDarkMode);
  const saveDialogPending = useAppStore((s) => s.saveDialogPending);
  const setSaveDialogPending = useAppStore((s) => s.setSaveDialogPending);
  const closeConfirmTab = useAppStore((s) => s.closeConfirmTab);
  const setCloseConfirmTab = useAppStore((s) => s.setCloseConfirmTab);

  const sql = activeTab.sql;

  const runQuery = useRunQuery();
  const [editorConfig, setEditorConfig] = useState<EditorConfig>(defaultEditorConfig);
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [saveScriptName, setSaveScriptName] = useState("");
  const [saveError, setSaveError] = useState("");
  const [udfSaving, setUdfSaving] = useState(false);

  useEffect(() => {
    if (saveDialogPending) {
      openSaveDialog();
      setSaveDialogPending(false);
    }
  }, [saveDialogPending]);

  useEffect(() => {
    invoke<EditorConfig>("get_editor_config")
      .then((cfg) => {
        setEditorConfig(cfg);
        setDarkMode(cfg.theme === "dark");
      })
      .catch(() => {});
  }, []);

  async function toggleTheme() {
    const newTheme = editorConfig.theme === "dark" ? "light" : "dark";
    const updated = { ...editorConfig, theme: newTheme };
    setEditorConfig(updated);
    setDarkMode(newTheme === "dark");
    try {
      await invoke("save_editor_config", { config: updated });
    } catch (e) {
      console.error(e);
    }
  }

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

  async function handleExecuteUdf() {
    setUdfSaving(true);
    try {
      await invoke("save_udf", { sql });
      const infos = await invoke<{ name: string }[]>("list_udfs");
      setUdfs(infos.map((u) => u.name));
    } catch (e) {
      console.error(e);
    } finally {
      setUdfSaving(false);
    }
  }

  async function handleRunQuery() {
    await runQuery(sql);
  }

  runQueryRef.current = handleRunQuery;

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
        <span className="text-xs font-semibold text-muted-foreground">
          {activeTab.linkedUdf ? "UDF" : "QUERY"}
        </span>
        {activeTab.linkedScript && (
          <span className="text-[10px] text-muted-foreground font-mono truncate max-w-[160px]" title={activeTab.linkedScript}>
            {activeTab.linkedScript}
          </span>
        )}
        {activeTab.linkedUdf && (
          <span className="text-[10px] text-blue-400 font-mono truncate max-w-[160px]" title={activeTab.linkedUdf}>
            {activeTab.linkedUdf}
          </span>
        )}
        <div className="flex-1" />
        <Button
          size="icon"
          variant="ghost"
          className="h-6 w-6"
          onClick={toggleTheme}
          title={editorConfig.theme === "dark" ? "ライトテーマに切り替え" : "ダークテーマに切り替え"}
        >
          {editorConfig.theme === "dark"
            ? <SunIcon className="h-3 w-3" />
            : <MoonIcon className="h-3 w-3" />}
        </Button>
        <Button
          size="sm"
          variant={historyOpen ? "secondary" : "ghost"}
          className="h-6 text-xs gap-1"
          onClick={() => setHistoryOpen(!historyOpen)}
          title="Query History"
        >
          <ClockIcon className="h-3 w-3" />
          History
        </Button>
        <Button
          size="sm"
          variant="ghost"
          className="h-6 text-xs gap-1"
          onClick={handleExecuteUdf}
          disabled={udfSaving}
          title="SQLを実行してUDFを登録（CREATE OR REPLACE MACRO）"
        >
          {udfSaving
            ? <Loader2Icon className="h-3 w-3 animate-spin" />
            : <FunctionSquareIcon className="h-3 w-3" />}
          {activeTab.linkedUdf ? "Save UDF" : "UDF"}
        </Button>
        {!activeTab.linkedUdf && (
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
        )}
        <Button size="sm" onClick={handleRunQuery} disabled={isRunning} className={`h-6 text-xs gap-1 ${isRunning ? "opacity-80" : ""}`}>
          {isRunning
            ? <Loader2Icon className="h-3 w-3 animate-spin" />
            : <PlayIcon className="h-3 w-3" />}
          {isRunning ? "Running..." : "Run"}
          {!isRunning && <span className="text-muted-foreground text-[10px] ml-1">⌘↵</span>}
        </Button>
      </div>

      <div className="flex-1 overflow-hidden relative">
        {historyOpen && (
          <QueryHistory
            onSelect={(q) => updateTabSql(activeTabId, q)}
            onClose={() => setHistoryOpen(false)}
          />
        )}
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
