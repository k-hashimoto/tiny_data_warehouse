import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { CsvImportOptions, CsvPreviewResult, TableInfo, QueryResult } from "@/store/appStore";
import { Button } from "@/components/ui/button";

interface Props {
  initialFilePath?: string;
  schemas?: string[];
  onImported: (table: TableInfo) => void;
  onClose: () => void;
}

const DEFAULT_OPTS: Omit<CsvImportOptions, "file_path" | "table_name" | "schema_name"> = {
  has_header: true,
  delimiter: "comma",
  encoding: "utf8",
  if_exists: "fail",
};

export function CsvImportDialog({ initialFilePath, schemas = ["main"], onImported, onClose }: Props) {
  const [filePath, setFilePath] = useState(initialFilePath ?? "");
  const [schemaName, setSchemaName] = useState("main");
  const [opts, setOpts] = useState(DEFAULT_OPTS);
  const initialTableName = initialFilePath
    ? initialFilePath.split("/").pop()?.replace(/\.[^.]+$/, "").replace(/[^a-zA-Z0-9_]/g, "_") ?? ""
    : "";
  const [tableName, setTableName] = useState(initialTableName);
  const [preview, setPreview] = useState<CsvPreviewResult | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  async function pickFile() {
    const selected = await open({
      filters: [{ name: "CSV", extensions: ["csv", "tsv", "txt"] }],
      multiple: false,
    });
    if (!selected) return;
    const path = typeof selected === "string" ? selected : selected;
    setFilePath(path);
    setPreview(null);
    setError("");
    // Auto-suggest table name from filename
    const stem = path.split("/").pop()?.replace(/\.[^.]+$/, "") ?? "table";
    setTableName(stem.replace(/[^a-zA-Z0-9_]/g, "_"));
  }

  async function loadPreview() {
    if (!filePath) return;
    setLoading(true);
    setError("");
    try {
      const result = await invoke<CsvPreviewResult>("preview_csv", {
        opts: { ...opts, file_path: filePath, table_name: tableName || "preview" },
      });
      setPreview(result);
      if (!tableName) setTableName(result.suggested_table_name);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function doImport() {
    if (!filePath || !tableName) return;
    setLoading(true);
    setError("");
    try {
      const table = await invoke<TableInfo>("import_csv", {
        opts: { ...opts, file_path: filePath, table_name: tableName, schema_name: schemaName },
      });
      onImported(table);
    } catch (e) {
      setError(String(e));
      setLoading(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-background border rounded-lg shadow-xl w-[640px] max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b shrink-0">
          <span className="font-semibold text-sm">Import CSV</span>
          <button onClick={onClose} className="text-muted-foreground hover:text-foreground text-lg leading-none">✕</button>
        </div>

        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {/* File picker */}
          <div className="space-y-1">
            <label className="text-xs font-medium text-muted-foreground">File</label>
            <div className="flex gap-2">
              <input
                readOnly
                value={filePath}
                placeholder="No file selected"
                className="flex-1 text-xs bg-muted border rounded px-2 py-1.5 truncate"
              />
              <Button size="sm" variant="outline" className="text-xs h-7 shrink-0" onClick={pickFile}>
                Browse…
              </Button>
            </div>
          </div>

          {/* Options */}
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <label className="text-xs font-medium text-muted-foreground">Schema</label>
              <select
                value={schemaName}
                onChange={(e) => setSchemaName(e.target.value)}
                className="w-full text-xs bg-background border rounded px-2 py-1.5"
              >
                {schemas.map((s) => (
                  <option key={s} value={s}>{s}</option>
                ))}
              </select>
            </div>
            <div className="space-y-1">
              <label className="text-xs font-medium text-muted-foreground">Delimiter</label>
              <select
                value={opts.delimiter}
                onChange={(e) => setOpts((o) => ({ ...o, delimiter: e.target.value }))}
                className="w-full text-xs bg-background border rounded px-2 py-1.5"
              >
                <option value="comma">Comma (,)</option>
                <option value="tab">Tab</option>
                <option value="semicolon">Semicolon (;)</option>
              </select>
            </div>
            <div className="space-y-1">
              <label className="text-xs font-medium text-muted-foreground">Encoding</label>
              <select
                value={opts.encoding}
                onChange={(e) => setOpts((o) => ({ ...o, encoding: e.target.value }))}
                className="w-full text-xs bg-background border rounded px-2 py-1.5"
              >
                <option value="utf8">UTF-8</option>
                <option value="sjis">Shift-JIS</option>
              </select>
            </div>
            <div className="space-y-1">
              <label className="text-xs font-medium text-muted-foreground">Table name</label>
              <input
                value={tableName}
                onChange={(e) => setTableName(e.target.value)}
                className="w-full text-xs bg-background border rounded px-2 py-1.5"
              />
            </div>
            <div className="space-y-1">
              <label className="text-xs font-medium text-muted-foreground">If table exists</label>
              <select
                value={opts.if_exists}
                onChange={(e) => setOpts((o) => ({ ...o, if_exists: e.target.value }))}
                className="w-full text-xs bg-background border rounded px-2 py-1.5"
              >
                <option value="fail">Fail</option>
                <option value="replace">Replace</option>
                <option value="append">Append</option>
              </select>
            </div>
          </div>

          <label className="flex items-center gap-2 text-xs cursor-pointer">
            <input
              type="checkbox"
              checked={opts.has_header}
              onChange={(e) => setOpts((o) => ({ ...o, has_header: e.target.checked }))}
            />
            First row is header
          </label>

          {/* Preview button */}
          <Button
            size="sm"
            variant="outline"
            className="text-xs h-7"
            disabled={!filePath || loading}
            onClick={loadPreview}
          >
            {loading ? "Loading…" : "Preview (first 10 rows)"}
          </Button>

          {/* Preview table */}
          {preview && <PreviewTable result={preview.preview} />}

          {/* Error */}
          {error && (
            <div className="text-xs text-destructive bg-destructive/10 border border-destructive/30 rounded p-2">
              {error}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 px-4 py-3 border-t shrink-0">
          <Button size="sm" variant="outline" className="text-xs h-7" onClick={onClose}>
            Cancel
          </Button>
          <Button
            size="sm"
            className="text-xs h-7"
            disabled={!filePath || !tableName || loading}
            onClick={doImport}
          >
            {loading ? "Importing…" : "Import"}
          </Button>
        </div>
      </div>
    </div>
  );
}

function PreviewTable({ result }: { result: QueryResult }) {
  return (
    <div className="overflow-auto border rounded max-h-48">
      <table className="w-full text-xs border-collapse">
        <thead className="sticky top-0 bg-muted">
          <tr>
            {result.columns.map((col) => (
              <th key={col} className="border px-2 py-1 text-left whitespace-nowrap font-medium">
                {col}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {result.rows.map((row, i) => (
            <tr key={i} className="hover:bg-accent/40">
              {row.map((cell, j) => (
                <td key={j} className="border px-2 py-0.5 font-mono whitespace-nowrap">
                  {cell === null ? <span className="text-muted-foreground">NULL</span> : String(cell)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
