import { useState, useEffect } from "react";
import { usePanelRef } from "react-resizable-panels";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { TableTree } from "@/components/Explorer/TableTree";
import { DbtSection } from "@/components/Explorer/DbtSection";
import { ScriptList } from "@/components/Explorer/ScriptList";
import { TableMetaPanel } from "@/components/Explorer/TableMetaPanel";
import { useAppStore } from "@/store/appStore";

const HEADER_PX = 28;

export function ExplorerPanel() {
  const metaPanel = useAppStore((s) => s.metaPanel);

  const tableTreeRef = usePanelRef();
  const dbtRef = usePanelRef();
  const scriptRef = usePanelRef();
  const metaRef = usePanelRef();

  const [tableTreeCollapsed, setTableTreeCollapsed] = useState(false);
  const [dbtCollapsed, setDbtCollapsed] = useState(false);
  const [scriptCollapsed, setScriptCollapsed] = useState(false);

  // metaPanel が設定されたときにメタパネルを展開、解除されたら折りたたむ
  useEffect(() => {
    if (metaPanel) {
      const panel = metaRef.current;
      if (panel?.isCollapsed()) {
        panel.resize("38%");
      }
    } else {
      metaRef.current?.collapse();
    }
  }, [metaPanel, metaRef]);

  function togglePanel(
    panelRef: ReturnType<typeof usePanelRef>,
    setCollapsed: React.Dispatch<React.SetStateAction<boolean>>,
  ) {
    const panel = panelRef.current;
    if (!panel) return;
    if (panel.isCollapsed()) {
      panel.expand();
      setCollapsed(false);
    } else {
      panel.collapse();
      setCollapsed(true);
    }
  }

  return (
    <ResizablePanelGroup orientation="vertical" className="h-full">
      <ResizablePanel
        panelRef={tableTreeRef}
        collapsible
        collapsedSize={`${HEADER_PX}px`}
        minSize="80px"
        defaultSize="50%"
        onResize={() => {
          setTableTreeCollapsed(tableTreeRef.current?.isCollapsed() ?? false);
        }}
      >
        <TableTree
          isCollapsed={tableTreeCollapsed}
          onToggleCollapse={() => togglePanel(tableTreeRef, setTableTreeCollapsed)}
        />
      </ResizablePanel>

      <ResizableHandle withHandle className="z-10" />

      <ResizablePanel
        panelRef={dbtRef}
        collapsible
        collapsedSize={`${HEADER_PX}px`}
        minSize="60px"
        defaultSize="25%"
        onResize={() => {
          setDbtCollapsed(dbtRef.current?.isCollapsed() ?? false);
        }}
      >
        <DbtSection
          isCollapsed={dbtCollapsed}
          onToggleCollapse={() => togglePanel(dbtRef, setDbtCollapsed)}
        />
      </ResizablePanel>

      <ResizableHandle withHandle className="z-10" />

      <ResizablePanel
        panelRef={scriptRef}
        collapsible
        collapsedSize={`${HEADER_PX}px`}
        minSize="60px"
        defaultSize="25%"
        onResize={() => {
          setScriptCollapsed(scriptRef.current?.isCollapsed() ?? false);
        }}
      >
        <ScriptList
          isCollapsed={scriptCollapsed}
          onToggleCollapse={() => togglePanel(scriptRef, setScriptCollapsed)}
        />
      </ResizablePanel>

      <ResizableHandle withHandle className={`z-10 ${!metaPanel ? "hidden" : ""}`} />

      <ResizablePanel
        panelRef={metaRef}
        collapsible
        collapsedSize="0px"
        minSize="120px"
        defaultSize="0px"
      >
        {metaPanel && <TableMetaPanel />}
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
