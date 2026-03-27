import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { TableTree } from "@/components/Explorer/TableTree";
import { DbtSection } from "@/components/Explorer/DbtSection";
import { ScriptList } from "@/components/Explorer/ScriptList";

export function ExplorerPanel() {
  return (
    <ResizablePanelGroup orientation="vertical" className="h-full">
      <ResizablePanel defaultSize="50%" minSize="80px">
        <TableTree />
      </ResizablePanel>
      <ResizableHandle withHandle className="z-10" />
      <ResizablePanel defaultSize="25%" minSize="60px">
        <DbtSection />
      </ResizablePanel>
      <ResizableHandle withHandle className="z-10" />
      <ResizablePanel defaultSize="25%" minSize="60px">
        <ScriptList />
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
