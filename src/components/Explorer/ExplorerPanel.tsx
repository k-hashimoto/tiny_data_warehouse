import { useState } from "react";
import { usePanelRef } from "react-resizable-panels";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { TableTree } from "@/components/Explorer/TableTree";
import { DbtSection } from "@/components/Explorer/DbtSection";
import { ScriptList } from "@/components/Explorer/ScriptList";
const HEADER_PX = 28;

export function ExplorerPanel() {
  const tableTreeRef = usePanelRef();
  const dbtRef = usePanelRef();
  const scriptRef = usePanelRef();

  const [tableTreeCollapsed, setTableTreeCollapsed] = useState(false);
  const [dbtCollapsed, setDbtCollapsed] = useState(false);
  const [scriptCollapsed, setScriptCollapsed] = useState(false);

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

    </ResizablePanelGroup>
  );
}
