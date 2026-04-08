import { useEffect, useRef } from "react";
import { usePanelRef } from "react-resizable-panels";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { TableTree } from "@/components/Explorer/TableTree";
import { DbtSection } from "@/components/Explorer/DbtSection";
import { ScriptList } from "@/components/Explorer/ScriptList";
import { MacroList } from "@/components/Explorer/MacroList";
import { useAppStore } from "@/store/appStore";
const HEADER_PX = 28;

export function ExplorerPanel() {
  const tableTreeRef = usePanelRef();
  const dbtRef = usePanelRef();
  const scriptRef = usePanelRef();
  const macroRef = usePanelRef();

  const sidebarCollapsed = useAppStore((s) => s.sidebarCollapsed);
  const setSidebarCollapsed = useAppStore((s) => s.setSidebarCollapsed);

  // stale closure 対策: onResize クロージャ内で最新の store 値を参照するための ref
  const sidebarCollapsedRef = useRef(sidebarCollapsed);
  useEffect(() => {
    sidebarCollapsedRef.current = sidebarCollapsed;
  }, [sidebarCollapsed]);

  // マウント時に store の値を各パネルへ適用する
  useEffect(() => {
    if (sidebarCollapsed.tableTree) tableTreeRef.current?.collapse();
    if (sidebarCollapsed.dbt) dbtRef.current?.collapse();
    if (sidebarCollapsed.script) scriptRef.current?.collapse();
    if (sidebarCollapsed.macro) macroRef.current?.collapse();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function togglePanel(
    key: "tableTree" | "dbt" | "script" | "macro",
    panelRef: ReturnType<typeof usePanelRef>,
  ) {
    const panel = panelRef.current;
    if (!panel) return;
    if (panel.isCollapsed()) {
      panel.expand();
      setSidebarCollapsed(key, false);
    } else {
      panel.collapse();
      setSidebarCollapsed(key, true);
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
          const collapsed = tableTreeRef.current?.isCollapsed() ?? false;
          setSidebarCollapsed("tableTree", collapsed);
        }}
      >
        <TableTree
          isCollapsed={sidebarCollapsed.tableTree}
          onToggleCollapse={() => togglePanel("tableTree", tableTreeRef)}
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
          const collapsed = dbtRef.current?.isCollapsed() ?? false;
          // 誤展開防止: パネルが展開されたが store では折り畳み状態のとき即座に再折り畳みする
          if (!collapsed && sidebarCollapsedRef.current.dbt) {
            dbtRef.current?.collapse();
            return;
          }
          setSidebarCollapsed("dbt", collapsed);
        }}
      >
        <DbtSection
          isCollapsed={sidebarCollapsed.dbt}
          onToggleCollapse={() => togglePanel("dbt", dbtRef)}
        />
      </ResizablePanel>

      <ResizableHandle withHandle className="z-10" />

      <ResizablePanel
        panelRef={scriptRef}
        collapsible
        collapsedSize={`${HEADER_PX}px`}
        minSize="60px"
        defaultSize="15%"
        onResize={() => {
          const collapsed = scriptRef.current?.isCollapsed() ?? false;
          setSidebarCollapsed("script", collapsed);
        }}
      >
        <ScriptList
          isCollapsed={sidebarCollapsed.script}
          onToggleCollapse={() => togglePanel("script", scriptRef)}
        />
      </ResizablePanel>

      <ResizableHandle withHandle className="z-10" />

      <ResizablePanel
        panelRef={macroRef}
        collapsible
        collapsedSize={`${HEADER_PX}px`}
        minSize="60px"
        defaultSize="10%"
        onResize={() => {
          const collapsed = macroRef.current?.isCollapsed() ?? false;
          setSidebarCollapsed("macro", collapsed);
        }}
      >
        <MacroList
          isCollapsed={sidebarCollapsed.macro}
          onToggleCollapse={() => togglePanel("macro", macroRef)}
        />
      </ResizablePanel>

    </ResizablePanelGroup>
  );
}
