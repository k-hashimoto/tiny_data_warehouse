import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { ExplorerPanel } from "@/components/Explorer/ExplorerPanel";
import { Editor, type EditorConfig } from "@/components/QueryEditor/Editor";
import { ResultTable } from "@/components/ResultsPanel/ResultTable";
import { TableMetaPanel } from "@/components/Explorer/TableMetaPanel";
import { StatusBar } from "@/components/StatusBar";
import { QueryHistory } from "@/components/QueryHistory/QueryHistory";
import { useAppStore } from "@/store/appStore";
import { useRunQuery } from "@/hooks/useRunQuery";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";
import { SunIcon, MoonIcon, ClockIcon } from "lucide-react";
import { Button } from "@/components/ui/button";

const defaultEditorConfig: EditorConfig = { line_numbers: true, tab_size: 4, theme: "dark" };

function App() {
  const historyOpen = useAppStore((s) => s.historyOpen);
  const setHistoryOpen = useAppStore((s) => s.setHistoryOpen);
  const metaPanel = useAppStore((s) => s.getActiveTab().metaPanel);
  const darkMode = useAppStore((s) => s.darkMode);
  const setDarkMode = useAppStore((s) => s.setDarkMode);
  const tabs = useAppStore((s) => s.tabs);
  const activeTabId = useAppStore((s) => s.activeTabId);
  const updateTabSql = useAppStore((s) => s.updateTabSql);
  const runQuery = useRunQuery();
  const [mcpActive, setMcpActive] = useState(false);
  useKeyboardShortcuts();

  const [editorConfig, setEditorConfig] = useState<EditorConfig>(defaultEditorConfig);

  useEffect(() => {
    const activeTab = tabs.find((t) => t.id === activeTabId);
    if (activeTab) runQuery(activeTab.sql);
    // yml → DuckDB COMMENT 同期（起動時）
    invoke("sync_yml_metadata").catch(() => {});
  }, []);

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

  const [mcpServerReady, setMcpServerReady] = useState<boolean | null>(null);
  const [mcpControlling, setMcpControlling] = useState(false);

  useEffect(() => {
    // Query current status on mount to handle race condition where
    // mcp-server-ready fires before listeners are registered
    invoke<boolean>("get_mcp_server_status").then((ready) => setMcpServerReady(ready));

    const unlisten1 = listen("mcp-active", () => setMcpActive(true));
    const unlisten2 = listen("mcp-idle", () => setMcpActive(false));
    const unlisten3 = listen("mcp-server-ready", () => setMcpServerReady(true));
    const unlisten4 = listen("mcp-server-error", () => setMcpServerReady(false));
    const unlisten5 = listen("mcp-server-stopped", () => setMcpServerReady(false));
    return () => {
      unlisten1.then((f) => f());
      unlisten2.then((f) => f());
      unlisten3.then((f) => f());
      unlisten4.then((f) => f());
      unlisten5.then((f) => f());
    };
  }, []);

  async function handleStopMcp() {
    setMcpControlling(true);
    try { await invoke("stop_mcp_server"); } finally { setMcpControlling(false); }
  }

  async function handleRestartMcp() {
    setMcpControlling(true);
    try { await invoke("restart_mcp_server"); } finally { setMcpControlling(false); }
  }

  return (
    <div className={`flex h-screen flex-col overflow-hidden bg-background text-foreground${darkMode ? " dark" : ""}`}>
      {mcpActive && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
          <div className="rounded-lg bg-background px-8 py-5 shadow-xl border text-center">
            <p className="text-sm font-medium">AI 操作中...</p>
            <p className="mt-1 text-xs text-muted-foreground">完了するまでお待ちください</p>
          </div>
        </div>
      )}
      {/* macOS titlebar drag region */}
      <div data-tauri-drag-region className="h-7 shrink-0 bg-background flex items-center justify-end px-3 gap-1.5">
        <button
          className="pointer-events-auto flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] text-muted-foreground hover:bg-accent hover:text-foreground transition-colors"
          onClick={toggleTheme}
          title={editorConfig.theme === "dark" ? "ライトテーマに切り替え" : "ダークテーマに切り替え"}
        >
          {editorConfig.theme === "dark"
            ? <><SunIcon className="h-3 w-3" /><span>Light</span></>
            : <><MoonIcon className="h-3 w-3" /><span>Dark</span></>}
        </button>
        <span className="text-border text-[10px] mx-1">|</span>
        {mcpServerReady !== null && (
          <>
            <div className="pointer-events-none flex items-center gap-1 text-[10px] text-muted-foreground">
              <span className={`h-1.5 w-1.5 rounded-full ${mcpServerReady ? "bg-green-500" : "bg-red-500"}`} />
              <span>MCP</span>
            </div>
            {mcpServerReady ? (
              <button
                className="pointer-events-auto text-[10px] text-muted-foreground opacity-50 hover:opacity-100 transition-opacity disabled:opacity-30"
                title="MCPサーバーを停止"
                disabled={mcpControlling}
                onClick={handleStopMcp}
              >
                Stop
              </button>
            ) : (
              <button
                className="pointer-events-auto text-[10px] text-muted-foreground opacity-50 hover:opacity-100 transition-opacity disabled:opacity-30"
                title="MCPサーバーを再起動"
                disabled={mcpControlling}
                onClick={handleRestartMcp}
              >
                Restart
              </button>
            )}
          </>
        )}
      </div>
      {/* Main: horizontal split (Explorer | Editor+Results) */}
      <ResizablePanelGroup orientation="horizontal" className="flex-1 overflow-hidden">
        {/* Explorer */}
        <ResizablePanel defaultSize="240px" minSize="160px" maxSize="45%">
          <ExplorerPanel />
        </ResizablePanel>

        <ResizableHandle withHandle className="z-10" />

        {/* Right: vertical split (Editor | Results) */}
        <ResizablePanel>
          <ResizablePanelGroup orientation="vertical">
            <ResizablePanel defaultSize="38%" minSize="120px">
              <Editor editorConfig={editorConfig} />
            </ResizablePanel>

            <ResizableHandle withHandle className="z-10" />

            <ResizablePanel minSize="100px">
              <div className="h-full flex flex-col">
                {/* Results panel toolbar */}
                <div className="flex items-center border-b px-2 py-0.5 shrink-0">
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
                </div>
                {historyOpen ? (
                  <div className="flex-1 overflow-hidden relative">
                    <QueryHistory
                      onSelect={(q) => { updateTabSql(activeTabId, q); setHistoryOpen(false); }}
                      onClose={() => setHistoryOpen(false)}
                    />
                  </div>
                ) : (
                  <div className="flex-1 overflow-hidden">
                    {metaPanel ? <TableMetaPanel /> : <ResultTable />}
                  </div>
                )}
              </div>
            </ResizablePanel>
          </ResizablePanelGroup>
        </ResizablePanel>
      </ResizablePanelGroup>

      <StatusBar />
    </div>
  );
}

export default App;
