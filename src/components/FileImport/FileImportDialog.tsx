import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { CsvImportOptions, CsvPreviewResult, JsonImportOptions, JsonPreviewResult, TableInfo, QueryResult } from "@/store/appStore";
import { Button } from "@/components/ui/button";

type ImportTab = "csv" | "json";

interface Props {
  initialFilePath?: string;
  schemas?: string[];
  onImported: (table: TableInfo) => void;
  onClose: () => void;
}

function suggestTableName(filePath: string): string {
  const stem = filePath.split("/").pop()?.replace(/\.[^.]+$/, "") ?? "table";
  return stem.replace(/[^a-zA-Z0-9_]/g, "_");
}

// ---------------------------------------------------------------------------
// CSV tab
// ---------------------------------------------------------------------------

const DEFAULT_CSV_OPTS: Omit<CsvImportOptions, "file_path" | "table_name" | "schema_name"> = {
  has_header: true,
  delimiter: "comma",
  encoding: "utf8",
  if_exists: "fail",
};

function CsvTab({
  initialFilePath,
  schemas,
  onImported,
}: {
  initialFilePath?: string;
  schemas: string[];
  onImported: (table: TableInfo) => void;
}) {
  const [filePath, setFilePath] = useState(initialFilePath ?? "");
  const [schemaName, setSchemaName] = useState("main");
  const [opts, setOpts] = useState(DEFAULT_CSV_OPTS);
  const [tableName, setTableName] = useState(initialFilePath ? suggestTableName(initialFilePath) : "");
  const [preview, setPreview] = useState<CsvPreviewResult | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  async function pickFile() {
    const selected = await open({ filters: [{ name: "CSV", extensions: ["csv", "tsv", "txt"] }], multiple: false });
    if (!selected) return;
    const path = typeof selected === "string" ? selected : selected;
    setFilePath(path);
    setPreview(null);
    setError("");
    setTableName(suggestTableName(path));
  }

  async function loadPreview() {
    if (!filePath) return;
    setLoading(true);
    setError("");
    try {
      const result = await invoke<CsvPreviewResult>("preview_csv", {
        opts: { ...opts, file_path: filePath, table_name: tableName || "preview", schema_name: schemaName },
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
    <div className="space-y-4">
      <div className="space-y-1">
        <label className="text-xs font-medium text-muted-foreground">File</label>
        <div className="flex gap-2">
          <input readOnly value={filePath} placeholder="No file selected"
            className="flex-1 text-xs bg-muted border rounded px-2 py-1.5 truncate" />
          <Button size="sm" variant="outline" className="text-xs h-7 shrink-0" onClick={pickFile}>Browse…</Button>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1">
          <label className="text-xs font-medium text-muted-foreground">Schema</label>
          <select value={schemaName} onChange={(e) => setSchemaName(e.target.value)}
            className="w-full text-xs bg-background border rounded px-2 py-1.5">
            {schemas.map((s) => <option key={s} value={s}>{s}</option>)}
          </select>
        </div>
        <div className="space-y-1">
          <label className="text-xs font-medium text-muted-foreground">Delimiter</label>
          <select value={opts.delimiter} onChange={(e) => setOpts((o) => ({ ...o, delimiter: e.target.value }))}
            className="w-full text-xs bg-background border rounded px-2 py-1.5">
            <option value="comma">Comma (,)</option>
            <option value="tab">Tab</option>
            <option value="semicolon">Semicolon (;)</option>
          </select>
        </div>
        <div className="space-y-1">
          <label className="text-xs font-medium text-muted-foreground">Encoding</label>
          <select value={opts.encoding} onChange={(e) => setOpts((o) => ({ ...o, encoding: e.target.value }))}
            className="w-full text-xs bg-background border rounded px-2 py-1.5">
            <option value="utf8">UTF-8</option>
            <option value="sjis">Shift-JIS</option>
          </select>
        </div>
        <div className="space-y-1">
          <label className="text-xs font-medium text-muted-foreground">Table name</label>
          <input value={tableName} onChange={(e) => setTableName(e.target.value)}
            className="w-full text-xs bg-background border rounded px-2 py-1.5" />
        </div>
        <div className="space-y-1">
          <label className="text-xs font-medium text-muted-foreground">If table exists</label>
          <select value={opts.if_exists} onChange={(e) => setOpts((o) => ({ ...o, if_exists: e.target.value }))}
            className="w-full text-xs bg-background border rounded px-2 py-1.5">
            <option value="fail">Fail</option>
            <option value="replace">Replace</option>
            <option value="append">Append</option>
          </select>
        </div>
      </div>

      <label className="flex items-center gap-2 text-xs cursor-pointer">
        <input type="checkbox" checked={opts.has_header}
          onChange={(e) => setOpts((o) => ({ ...o, has_header: e.target.checked }))} />
        First row is header
      </label>

      <Button size="sm" variant="outline" className="text-xs h-7"
        disabled={!filePath || loading} onClick={loadPreview}>
        {loading ? "Loading…" : "Preview (first 10 rows)"}
      </Button>

      {preview && <PreviewTable result={preview.preview} />}
      {error && <ErrorBox message={error} />}

      <div className="flex justify-end gap-2 pt-2">
        <Button size="sm" className="text-xs h-7"
          disabled={!filePath || !tableName || loading} onClick={doImport}>
          {loading ? "Importing…" : "Import"}
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// JSON tab
// ---------------------------------------------------------------------------

function JsonTab({
  initialFilePath,
  schemas,
  onImported,
}: {
  initialFilePath?: string;
  schemas: string[];
  onImported: (table: TableInfo) => void;
}) {
  const [filePath, setFilePath] = useState(initialFilePath ?? "");
  const [schemaName, setSchemaName] = useState("main");
  const [ifExists, setIfExists] = useState("fail");
  const [tableName, setTableName] = useState(initialFilePath ? suggestTableName(initialFilePath) : "");
  const [preview, setPreview] = useState<JsonPreviewResult | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  async function pickFile() {
    const selected = await open({ filters: [{ name: "JSON", extensions: ["json", "jsonl", "ndjson"] }], multiple: false });
    if (!selected) return;
    const path = typeof selected === "string" ? selected : selected;
    setFilePath(path);
    setPreview(null);
    setError("");
    setTableName(suggestTableName(path));
  }

  function buildOpts(): JsonImportOptions {
    return { file_path: filePath, table_name: tableName || "preview", schema_name: schemaName, if_exists: ifExists };
  }

  async function loadPreview() {
    if (!filePath) return;
    setLoading(true);
    setError("");
    try {
      const result = await invoke<JsonPreviewResult>("preview_json", { opts: buildOpts() });
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
      const table = await invoke<TableInfo>("import_json", {
        opts: { ...buildOpts(), table_name: tableName },
      });
      onImported(table);
    } catch (e) {
      setError(String(e));
      setLoading(false);
    }
  }

  return (
    <div className="space-y-4">
      <div className="space-y-1">
        <label className="text-xs font-medium text-muted-foreground">File</label>
        <div className="flex gap-2">
          <input readOnly value={filePath} placeholder="No file selected"
            className="flex-1 text-xs bg-muted border rounded px-2 py-1.5 truncate" />
          <Button size="sm" variant="outline" className="text-xs h-7 shrink-0" onClick={pickFile}>Browse…</Button>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1">
          <label className="text-xs font-medium text-muted-foreground">Schema</label>
          <select value={schemaName} onChange={(e) => setSchemaName(e.target.value)}
            className="w-full text-xs bg-background border rounded px-2 py-1.5">
            {schemas.map((s) => <option key={s} value={s}>{s}</option>)}
          </select>
        </div>
        <div className="space-y-1">
          <label className="text-xs font-medium text-muted-foreground">Table name</label>
          <input value={tableName} onChange={(e) => setTableName(e.target.value)}
            className="w-full text-xs bg-background border rounded px-2 py-1.5" />
        </div>
        <div className="space-y-1">
          <label className="text-xs font-medium text-muted-foreground">If table exists</label>
          <select value={ifExists} onChange={(e) => setIfExists(e.target.value)}
            className="w-full text-xs bg-background border rounded px-2 py-1.5">
            <option value="fail">Fail</option>
            <option value="replace">Replace</option>
            <option value="append">Append</option>
          </select>
        </div>
      </div>

      <Button size="sm" variant="outline" className="text-xs h-7"
        disabled={!filePath || loading} onClick={loadPreview}>
        {loading ? "Loading…" : "Preview (first 10 rows)"}
      </Button>

      {preview && <PreviewTable result={preview.preview} />}
      {error && <ErrorBox message={error} />}

      <div className="flex justify-end gap-2 pt-2">
        <Button size="sm" className="text-xs h-7"
          disabled={!filePath || !tableName || loading} onClick={doImport}>
          {loading ? "Importing…" : "Import"}
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main dialog (tabbed)
// ---------------------------------------------------------------------------

export function FileImportDialog({ initialFilePath, schemas = ["main"], onImported, onClose }: Props) {
  const initialTab: ImportTab =
    initialFilePath?.match(/\.(json|jsonl|ndjson)$/i) ? "json" : "csv";
  const [activeTab, setActiveTab] = useState<ImportTab>(initialTab);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-background border rounded-lg shadow-xl w-[640px] max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b shrink-0">
          <span className="font-semibold text-sm">Import</span>
          <button onClick={onClose} className="text-muted-foreground hover:text-foreground text-lg leading-none">✕</button>
        </div>

        {/* Tabs */}
        <div className="flex border-b shrink-0">
          {(["csv", "json"] as ImportTab[]).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-2 text-xs font-medium border-b-2 transition-colors ${
                activeTab === tab
                  ? "border-primary text-primary"
                  : "border-transparent text-muted-foreground hover:text-foreground"
              }`}
            >
              {tab.toUpperCase()}
            </button>
          ))}
        </div>

        <div className="flex-1 overflow-y-auto p-4">
          {activeTab === "csv" ? (
            <CsvTab
              initialFilePath={initialTab === "csv" ? initialFilePath : undefined}
              schemas={schemas}
              onImported={onImported}
            />
          ) : (
            <JsonTab
              initialFilePath={initialTab === "json" ? initialFilePath : undefined}
              schemas={schemas}
              onImported={onImported}
            />
          )}
        </div>

        {/* Footer cancel */}
        <div className="flex justify-start px-4 py-3 border-t shrink-0">
          <Button size="sm" variant="outline" className="text-xs h-7" onClick={onClose}>
            Cancel
          </Button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

function PreviewTable({ result }: { result: QueryResult }) {
  return (
    <div className="overflow-auto border rounded max-h-48">
      <table className="w-full text-xs border-collapse">
        <thead className="sticky top-0 bg-muted">
          <tr>
            {result.columns.map((col) => (
              <th key={col} className="border px-2 py-1 text-left whitespace-nowrap font-medium">{col}</th>
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

function ErrorBox({ message }: { message: string }) {
  return (
    <div className="text-xs text-destructive bg-destructive/10 border border-destructive/30 rounded p-2">
      {message}
    </div>
  );
}
