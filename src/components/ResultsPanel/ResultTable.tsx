import {
  useReactTable,
  getCoreRowModel,
  getPaginationRowModel,
  flexRender,
  createColumnHelper,
  ColumnDef,
} from "@tanstack/react-table";
import { useAppStore } from "@/store/appStore";
import { Button } from "@/components/ui/button";
import { useMemo, useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DownloadIcon, LoaderIcon, Settings2Icon } from "lucide-react";

const PAGE_SIZE = 100;

interface EditorConfig {
  export_dir: string;
}

function defaultFilename(title: string, linkedScript: string | null): string {
  if (linkedScript) {
    const base = linkedScript.split("/").pop() ?? linkedScript;
    return base.replace(/\.[^.]+$/, "") + ".csv";
  }
  return title.replace(/\s+/g, "_") + ".csv";
}

export function ResultTable() {
  const activeTab = useAppStore((s) => s.getActiveTab());
  const result = activeTab.result;
  const activeSql = activeTab.sql;
  const error = useAppStore((s) => s.error);

  const [exportDir, setExportDir] = useState("~/Downloads");
  // tabId -> filename mapping so each tab remembers its own export filename
  const [exportFilenames, setExportFilenames] = useState<Record<string, string>>({});
  const exportFilename = exportFilenames[activeTab.id] ?? null;

  const [showExportSetup, setShowExportSetup] = useState(false);
  const [setupFilename, setSetupFilename] = useState("");
  const [exporting, setExporting] = useState(false);
  const [exportMessage, setExportMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);

  useEffect(() => {
    invoke<EditorConfig>("get_editor_config")
      .then((c) => setExportDir(c.export_dir))
      .catch(() => {});
  }, []);

  function openExportSetup() {
    setSetupFilename(exportFilename ?? defaultFilename(activeTab.title, activeTab.linkedScript));
    setShowExportSetup(true);
  }

  async function handleExportClick() {
    if (!exportFilename) {
      openExportSetup();
    } else {
      await runExport(exportFilename);
    }
  }

  async function handleSetupConfirm() {
    const filename = setupFilename.trim();
    if (!filename) return;
    setExportFilenames((prev) => ({ ...prev, [activeTab.id]: filename }));
    setShowExportSetup(false);
    await runExport(filename);
  }

  async function runExport(filename: string) {
    setExporting(true);
    setExportMessage(null);
    try {
      const savedPath = await invoke<string>("export_query_csv", {
        sql: activeSql,
        exportDir,
        filename,
      });
      setExportMessage({ type: "success", text: `保存しました: ${savedPath}` });
    } catch (e) {
      setExportMessage({ type: "error", text: String(e) });
    } finally {
      setExporting(false);
    }
  }

  const columnHelper = createColumnHelper<Record<string, unknown>>();

  const columns: ColumnDef<Record<string, unknown>, unknown>[] = useMemo(
    () =>
      result?.columns.map((col) =>
        columnHelper.accessor(col, {
          header: col,
          cell: (info) => {
            const v = info.getValue();
            if (v === null || v === undefined) return <span className="text-muted-foreground">NULL</span>;
            if (typeof v === "object") return JSON.stringify(v);
            return String(v);
          },
        })
      ) ?? [],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [result?.columns]
  );

  const data: Record<string, unknown>[] = useMemo(
    () =>
      result?.rows.map((row) => {
        const obj: Record<string, unknown> = {};
        result.columns.forEach((col, i) => {
          obj[col] = row[i];
        });
        return obj;
      }) ?? [],
    [result]
  );

  const table = useReactTable({
    data,
    columns,
    getCoreRowModel: getCoreRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    initialState: { pagination: { pageSize: PAGE_SIZE } },
  });

  if (error) {
    return (
      <div className="p-3 text-sm text-destructive border border-destructive/30 rounded m-2 bg-destructive/5">
        {error}
      </div>
    );
  }

  if (!result) {
    return <div className="p-3 text-sm text-muted-foreground">Run a query to see results.</div>;
  }

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Toolbar */}
      <div className="flex items-center gap-2 border-b px-2 py-1 shrink-0">
        <span className="text-xs font-semibold text-muted-foreground">RESULTS</span>
        {result.truncated && (
          <span className="text-[10px] text-orange-400 font-medium">
            ⚠ 表示は1,000行に制限されています。全件取得したい場合はExport CSVを実行してください
          </span>
        )}
        <div className="flex-1" />
        {/* Export CSV button group */}
        <div className="flex items-center border rounded overflow-hidden">
          <Button
            size="sm"
            variant="ghost"
            className="h-6 text-xs gap-1 rounded-none border-r"
            onClick={handleExportClick}
            disabled={exporting}
            title={exportFilename ? `${exportDir}/${exportFilename} に保存` : "Export先を設定してCSV保存"}
          >
            {exporting
              ? <LoaderIcon className="h-3 w-3 animate-spin" />
              : <DownloadIcon className="h-3 w-3" />}
            Export CSV
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className="h-6 px-1.5 rounded-none"
            onClick={openExportSetup}
            disabled={exporting}
            title="Export先を変更"
          >
            <Settings2Icon className="h-3 w-3" />
          </Button>
        </div>
      </div>

      {/* Export result message */}
      {exportMessage && (
        <div
          className={`px-3 py-1.5 text-[11px] border-b shrink-0 flex items-center justify-between
            ${exportMessage.type === "success"
              ? "bg-green-500/10 text-green-400 border-green-500/20"
              : "bg-destructive/10 text-destructive border-destructive/20"}`}
        >
          <span className="truncate">{exportMessage.text}</span>
          <button className="ml-2 shrink-0 opacity-60 hover:opacity-100" onClick={() => setExportMessage(null)}>✕</button>
        </div>
      )}

      <div className="flex-1 overflow-auto">
        <table className="w-full text-sm border-collapse">
          <thead className="sticky top-0 bg-muted z-10">
            {table.getHeaderGroups().map((hg) => (
              <tr key={hg.id}>
                {hg.headers.map((h) => (
                  <th key={h.id} className="border px-2 py-1 text-left font-medium whitespace-nowrap">
                    {flexRender(h.column.columnDef.header, h.getContext())}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row) => (
              <tr key={row.id} className="hover:bg-accent/40">
                {row.getVisibleCells().map((cell) => (
                  <td key={cell.id} className="border px-2 py-0.5 font-mono whitespace-nowrap">
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Pagination */}
      {table.getPageCount() > 1 && (
        <div className="flex items-center gap-2 border-t px-3 py-1 text-xs shrink-0">
          <Button size="sm" variant="outline" className="h-6 text-xs" onClick={() => table.previousPage()} disabled={!table.getCanPreviousPage()}>
            ← Prev
          </Button>
          <span className="text-muted-foreground">
            Page {table.getState().pagination.pageIndex + 1} / {table.getPageCount()}
          </span>
          <Button size="sm" variant="outline" className="h-6 text-xs" onClick={() => table.nextPage()} disabled={!table.getCanNextPage()}>
            Next →
          </Button>
        </div>
      )}

      {/* Export setup dialog */}
      {showExportSetup && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
          <div className="bg-popover border rounded-lg shadow-xl p-4 w-80 text-sm">
            <p className="font-semibold mb-2">Export先の設定</p>
            <p className="text-xs text-muted-foreground mb-3">
              ファイル名を設定すると、次回からワンクリックでExportできます。
            </p>
            <div className="mb-4">
              <label className="text-xs text-muted-foreground block mb-1">ファイル名</label>
              <input
                className="w-full border rounded px-2 py-1 text-xs bg-background font-mono"
                value={setupFilename}
                onChange={(e) => setSetupFilename(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleSetupConfirm()}
                autoFocus
              />
              <p className="text-[10px] text-muted-foreground mt-1 font-mono truncate">
                {exportDir}/{setupFilename}
              </p>
            </div>
            <div className="flex justify-end gap-2">
              <Button size="sm" variant="ghost" onClick={() => setShowExportSetup(false)}>
                キャンセル
              </Button>
              <Button size="sm" onClick={handleSetupConfirm} disabled={!setupFilename.trim()}>
                設定してExport
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
