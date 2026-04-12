import { useEffect, useRef, useState } from "react";
import { usePanelRef } from "react-resizable-panels";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { TableTree } from "@/components/Explorer/TableTree";
import { DbtSection } from "@/components/Explorer/DbtSection";
import { ScriptList } from "@/components/Explorer/ScriptList";
import { MacroList } from "@/components/Explorer/MacroList";

const HEADER_PX = 28;
const STORAGE_KEY = "sidebar-collapsed";

type SidebarKey = "tableTree" | "dbt" | "script" | "macro";
type SidebarCollapsed = Record<SidebarKey, boolean>;

const DEFAULT_COLLAPSED: SidebarCollapsed = {
  tableTree: false,
  dbt: true,
  script: false,
  macro: false,
};

function readSidebarState(): SidebarCollapsed {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return { ...DEFAULT_COLLAPSED, ...JSON.parse(raw) };
  } catch {}
  return { ...DEFAULT_COLLAPSED };
}

function writeSidebarState(state: SidebarCollapsed): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {}
}

export function ExplorerPanel() {
  const tableTreeRef = usePanelRef();
  const dbtRef = usePanelRef();
  const scriptRef = usePanelRef();
  const macroRef = usePanelRef();

  // localStorage から同期で初期値を読み込む（非同期復元を介さないため async hydration 問題が起きない）
  const [collapsed, setCollapsed] = useState<SidebarCollapsed>(() => readSidebarState());

  // onResize クロージャ内で最新の collapsed を参照するための ref
  // togglePanel 内で panel API 呼び出しより先に更新することで stale closure を防ぐ
  const collapsedRef = useRef(collapsed);

  function updateCollapsed(key: SidebarKey, value: boolean) {
    collapsedRef.current = { ...collapsedRef.current, [key]: value };
    writeSidebarState(collapsedRef.current);
    setCollapsed({ ...collapsedRef.current });
  }

  // マウント時に localStorage の値を各パネルへ適用する
  useEffect(() => {
    if (collapsedRef.current.tableTree) tableTreeRef.current?.collapse();
    if (collapsedRef.current.dbt) dbtRef.current?.collapse();
    if (collapsedRef.current.script) scriptRef.current?.collapse();
    if (collapsedRef.current.macro) macroRef.current?.collapse();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function togglePanel(key: SidebarKey, panelRef: ReturnType<typeof usePanelRef>) {
    const panel = panelRef.current;
    if (!panel) return;
    if (panel.isCollapsed()) {
      // panel.expand() が同期的に onResize を発火させるため、ref を先に更新する
      updateCollapsed(key, false);
      panel.expand();
    } else {
      updateCollapsed(key, true);
      panel.collapse();
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
          const isCollapsed = tableTreeRef.current?.isCollapsed() ?? false;
          if (isCollapsed !== collapsedRef.current.tableTree) {
            updateCollapsed("tableTree", isCollapsed);
          }
        }}
      >
        <TableTree
          isCollapsed={collapsed.tableTree}
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
          const isCollapsed = dbtRef.current?.isCollapsed() ?? false;
          if (!isCollapsed && collapsedRef.current.dbt) {
            // adhoc 折り畳み時などにライブラリが強制展開した場合は即座に再折り畳みする
            dbtRef.current?.collapse();
            return;
          }
          if (isCollapsed !== collapsedRef.current.dbt) {
            updateCollapsed("dbt", isCollapsed);
          }
        }}
      >
        <DbtSection
          isCollapsed={collapsed.dbt}
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
          const isCollapsed = scriptRef.current?.isCollapsed() ?? false;
          if (isCollapsed !== collapsedRef.current.script) {
            updateCollapsed("script", isCollapsed);
          }
        }}
      >
        <ScriptList
          isCollapsed={collapsed.script}
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
          const isCollapsed = macroRef.current?.isCollapsed() ?? false;
          if (isCollapsed !== collapsedRef.current.macro) {
            updateCollapsed("macro", isCollapsed);
          }
        }}
      >
        <MacroList
          isCollapsed={collapsed.macro}
          onToggleCollapse={() => togglePanel("macro", macroRef)}
        />
      </ResizablePanel>

    </ResizablePanelGroup>
  );
}
