import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { ExplorerPanel } from "@/components/Explorer/ExplorerPanel";
import { Editor } from "@/components/QueryEditor/Editor";
import { ResultTable } from "@/components/ResultsPanel/ResultTable";
import { StatusBar } from "@/components/StatusBar";
import { useAppStore } from "@/store/appStore";
import { useRunQuery } from "@/hooks/useRunQuery";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";

function App() {
  const historyOpen = useAppStore((s) => s.historyOpen);
  const darkMode = useAppStore((s) => s.darkMode);
  const tabs = useAppStore((s) => s.tabs);
  const activeTabId = useAppStore((s) => s.activeTabId);
  const runQuery = useRunQuery();
  const [mcpActive, setMcpActive] = useState(false);
  useKeyboardShortcuts();

  useEffect(() => {
    const activeTab = tabs.find((t) => t.id === activeTabId);
    if (activeTab) runQuery(activeTab.sql);
  }, []);

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
              <Editor />
            </ResizablePanel>

            <ResizableHandle withHandle className="z-10" />

            <ResizablePanel minSize="100px">
              <div className={`h-full overflow-hidden ${historyOpen ? "hidden" : ""}`}>
                <ResultTable />
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
