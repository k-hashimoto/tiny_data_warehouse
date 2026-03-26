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
import { DownloadIcon, LoaderIcon } from "lucide-react";

const PAGE_SIZE = 100;

interface EditorConfig {
  export_dir: string;
}

function generateFilename(): string {
  const now = new Date();
  const ts = now.toISOString().replace(/[-:T]/g, "").slice(0, 15);
  return `query_${ts}.csv`;
}

export function ResultTable() {
  const activeTab = useAppStore((s) => s.getActiveTab());
  const result = activeTab.result;
  const activeSql = activeTab.sql;
  const error = useAppStore((s) => s.error);

  const [exportDir, setExportDir] = useState("~/Downloads");
  const [showExportConfirm, setShowExportConfirm] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [exportMessage, setExportMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);

  useEffect(() => {
    invoke<EditorConfig>("get_editor_config")
      .then((c) => setExportDir(c.export_dir))
      .catch(() => {});
  }, []);

  const columnHelper = createColumnHelper<Record<string, unknown>>();

  const columns: ColumnDef<Record<string, unknown>, unknown>[] = useMemo(
    () =>
      result?.columns.map((col) =>
        columnHelper.accessor(col, {
          header: col,
          cell: (info) => {
            const v = info.getValue();
            if (v === null || v === undefined) return <span className="text-muted-foreground">NULL</span>;
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

  async function handleExport() {
    setShowExportConfirm(false);
    setExporting(true);
    setExportMessage(null);
    try {
      const filename = generateFilename();
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
            ⚠ 表示は1,000行に制限されています
          </span>
        )}
        <div className="flex-1" />
        <Button
          size="sm"
          variant="ghost"
          className="h-6 text-xs gap-1"
          onClick={() => setShowExportConfirm(true)}
          disabled={exporting}
          title="全件をCSVに保存（クエリを再実行します）"
        >
          {exporting
            ? <LoaderIcon className="h-3 w-3 animate-spin" />
            : <DownloadIcon className="h-3 w-3" />}
          Export CSV
        </Button>
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

      {/* Export confirmation dialog */}
      {showExportConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
          <div className="bg-popover border rounded-lg shadow-xl p-4 w-80 text-sm">
            <p className="font-semibold mb-2">CSVにエクスポート</p>
            <p className="text-xs text-muted-foreground mb-1">
              現在のクエリを再実行して全件をCSVに保存します。
              データ量によっては時間がかかる場合があります。
            </p>
            <p className="text-xs text-muted-foreground mb-4">
              保存先: <span className="font-mono">{exportDir}/query_*.csv</span>
            </p>
            <div className="flex justify-end gap-2">
              <Button size="sm" variant="ghost" onClick={() => setShowExportConfirm(false)}>
                キャンセル
              </Button>
              <Button size="sm" onClick={handleExport}>
                エクスポート
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
