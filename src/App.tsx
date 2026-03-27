import { useEffect } from "react";
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

  useEffect(() => {
    const activeTab = tabs.find((t) => t.id === activeTabId);
    if (activeTab) runQuery(activeTab.sql);
  }, []);

  return (
    <div className={`flex h-screen flex-col overflow-hidden bg-background text-foreground${darkMode ? " dark" : ""}`}>
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
