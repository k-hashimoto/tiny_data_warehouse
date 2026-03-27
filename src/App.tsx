import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
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

function App() {
  const historyOpen = useAppStore((s) => s.historyOpen);
  const darkMode = useAppStore((s) => s.darkMode);
  const tabs = useAppStore((s) => s.tabs);
  const activeTabId = useAppStore((s) => s.activeTabId);
  const runQuery = useRunQuery();
  const [mcpActive, setMcpActive] = useState(false);

  useEffect(() => {
    const activeTab = tabs.find((t) => t.id === activeTabId);
    if (activeTab) runQuery(activeTab.sql);
  }, []);

  useEffect(() => {
    const unlisten1 = listen("mcp-active", () => setMcpActive(true));
    const unlisten2 = listen("mcp-idle", () => setMcpActive(false));
    return () => {
      unlisten1.then((f) => f());
      unlisten2.then((f) => f());
    };
  }, []);

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
      <div data-tauri-drag-region className="h-7 shrink-0 bg-background" />
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
