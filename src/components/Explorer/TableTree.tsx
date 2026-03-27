import { useState, useCallback, useEffect, useMemo, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore, TableInfo } from "@/store/appStore";
import { Button } from "@/components/ui/button";
import {
  TableIcon, RefreshCwIcon, UploadIcon,
  ChevronRightIcon, ChevronDownIcon,
  FolderIcon, FolderOpenIcon, PlusIcon, DatabaseIcon, InfoIcon,
} from "lucide-react";
import { CsvImportDialog } from "@/components/CsvImport/CsvImportDialog";
import { TableMetaModal } from "@/components/Explorer/TableMetaModal";

interface ColumnInfo {
  name: string;
  column_type: string;
  nullable: boolean;
}

interface SchemaResult {
  table_name: string;
  columns: ColumnInfo[];
}

interface ContextMenu {
  x: number;
  y: number;
  type: "table";
  schemaName: string;
  tableName: string;
}

interface SchemaContextMenu {
  x: number;
  y: number;
  type: "schema";
  schemaName: string;
}

export function TableTree() {
  const tables = useAppStore((s) => s.tables);
  const setTables = useAppStore((s) => s.setTables);
  const setSql = useAppStore((s) => s.setSql);
  const setError = useAppStore((s) => s.setError);
  const tabs = useAppStore((s) => s.tabs);
  const activeTabId = useAppStore((s) => s.activeTabId);

  const [showImport, setShowImport] = useState(false);
  const [dragOver, setDragOver] = useState(false);
  const [pendingFile, setPendingFile] = useState<string | null>(null);

  // Per-table column schema expansion
  const [schemas, setSchemas] = useState<string[]>([]);
  const [expandedTable, setExpandedTable] = useState<string | null>(null);
  const [schemaCache, setSchemaCache] = useState<Record<string, ColumnInfo[]>>({});

  // Schema folder expansion
  const [expandedSchemas, setExpandedSchemas] = useState<Set<string>>(new Set(["main"]));

  // Search — hidden, to be re-enabled later
  const [searchQuery, _setSearchQuery] = useState("");

  // Context menus
  const [contextMenu, setContextMenu] = useState<ContextMenu | SchemaContextMenu | null>(null);

  // Drop confirmation
  const [dropConfirm, setDropConfirm] = useState<{ type: "table" | "schema"; schemaName: string; tableName?: string } | null>(null);

  // Schema creation dialog
  const [showCreateSchema, setShowCreateSchema] = useState(false);
  const [newSchemaName, setNewSchemaName] = useState("");

  // Table metadata modal
  const [metaTable, setMetaTable] = useState<{ schemaName: string; tableName: string } | null>(null);

  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => { refresh(); }, []);

  // Close context menu on outside click
  useEffect(() => {
    if (!contextMenu) return;
    const close = () => setContextMenu(null);
    window.addEventListener("click", close);
    return () => window.removeEventListener("click", close);
  }, [contextMenu]);

  async function refresh() {
    try {
      const [tableRes, schemaRes] = await Promise.all([
        invoke<TableInfo[]>("list_tables"),
        invoke<string[]>("list_schemas"),
      ]);
      setTables(tableRes);
      setSchemas(schemaRes);
    } catch (e) {
      setError(String(e));
    }
  }

  // All unique schemas from tables + list_schemas (includes empty schemas; main first)
  const allSchemas = useMemo(() => {
    const set = new Set([...schemas, ...tables.map((t) => t.schema_name)]);
    return [...set].sort((a, b) => {
      if (a === "main") return -1;
      if (b === "main") return 1;
      return a.localeCompare(b);
    });
  }, [tables, schemas]);

  // Tables grouped by schema, filtered by search
  const tablesBySchema = useMemo(() => {
    const q = searchQuery.toLowerCase();
    const map = new Map<string, TableInfo[]>();
    for (const schema of allSchemas) map.set(schema, []);
    for (const t of tables) {
      if (q && !t.name.toLowerCase().includes(q) && !t.schema_name.toLowerCase().includes(q)) continue;
      const list = map.get(t.schema_name) ?? [];
      list.push(t);
      map.set(t.schema_name, list);
    }
    return map;
  }, [tables, allSchemas, searchQuery]);

  function toggleSchemaFolder(schemaName: string) {
    setExpandedSchemas((prev) => {
      const next = new Set(prev);
      if (next.has(schemaName)) next.delete(schemaName);
      else next.add(schemaName);
      return next;
    });
  }

  async function toggleTableSchema(schemaName: string, tableName: string) {
    const key = `${schemaName}.${tableName}`;
    if (expandedTable === key) {
      setExpandedTable(null);
      return;
    }
    setExpandedTable(key);
    if (!schemaCache[key]) {
      try {
        const result = await invoke<SchemaResult>("get_schema", { schemaName, tableName });
        setSchemaCache((prev) => ({ ...prev, [key]: result.columns }));
      } catch (e) {
        setError(String(e));
      }
    }
  }

  function selectTable(schemaName: string, name: string) {
    const qualified = schemaName === "main" ? `"${name}"` : `"${schemaName}"."${name}"`;
    const newSql = `SELECT * FROM ${qualified} LIMIT 100`;
    const currentSql = tabs.find((t) => t.id === activeTabId)?.sql ?? "";
    setSql(currentSql.trim() === "" ? newSql : `${currentSql}\n\n${newSql}`);
  }

  function handleImported(table: TableInfo) {
    setTables(
      [...tables.filter((t) => !(t.schema_name === table.schema_name && t.name === table.name)), table]
        .sort((a, b) => a.schema_name.localeCompare(b.schema_name) || a.name.localeCompare(b.name))
    );
    setShowImport(false);
    setPendingFile(null);
  }

  function handleTableContextMenu(e: React.MouseEvent, schemaName: string, tableName: string) {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY, type: "table", schemaName, tableName });
  }

  function handleSchemaContextMenu(e: React.MouseEvent, schemaName: string) {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY, type: "schema", schemaName });
  }

  async function confirmDropTable(schemaName: string, tableName: string) {
    setContextMenu(null);
    setDropConfirm({ type: "table", schemaName, tableName });
  }

  async function confirmDropSchema(schemaName: string) {
    setContextMenu(null);
    setDropConfirm({ type: "schema", schemaName });
  }

  async function executeDropTable(schemaName: string, tableName: string) {
    setDropConfirm(null);
    try {
      const qualified = schemaName === "main"
        ? `"${tableName.replace(/"/g, '""')}"`
        : `"${schemaName.replace(/"/g, '""')}"."${tableName.replace(/"/g, '""')}"`;
      await invoke("run_query", { sql: `DROP TABLE IF EXISTS ${qualified}` });
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  async function executeDropSchema(schemaName: string) {
    setDropConfirm(null);
    try {
      await invoke("run_query", { sql: `DROP SCHEMA IF EXISTS "${schemaName.replace(/"/g, '""')}" CASCADE` });
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  function createSchema() {
    setNewSchemaName("");
    setShowCreateSchema(true);
  }

  async function executeCreateSchema() {
    const name = newSchemaName.trim();
    if (!name) return;
    setShowCreateSchema(false);
    try {
      await invoke("run_query", { sql: `CREATE SCHEMA IF NOT EXISTS "${name.replace(/"/g, '""')}"` });
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    if (e.dataTransfer.types.includes("Files")) setDragOver(true);
  }, []);

  const handleDragLeave = useCallback(() => setDragOver(false), []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
    const file = e.dataTransfer.files[0];
    if (!file) return;
    const path = (file as File & { path?: string }).path;
    if (path) {
      setPendingFile(path);
      setShowImport(true);
    }
  }, []);

  return (
    <>
      <div
        ref={containerRef}
        className={`relative flex flex-col h-full transition-colors ${dragOver ? "bg-accent/20 ring-2 ring-inset ring-primary" : ""}`}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-2 py-1 border-b shrink-0">
          <span className="flex items-center gap-1 text-xs font-semibold text-muted-foreground">
            <DatabaseIcon className="h-3 w-3" />
            ADHOC TABLES
          </span>
          <div className="flex items-center gap-0.5">
            <Button
              size="icon" variant="ghost" className="h-5 w-5"
              title="新しいスキーマ" onClick={createSchema}
            >
              <PlusIcon className="h-3 w-3" />
            </Button>
            <Button
              size="icon" variant="ghost" className="h-5 w-5"
              title="Import CSV"
              onClick={() => { setPendingFile(null); setShowImport(true); }}
            >
              <UploadIcon className="h-3 w-3" />
            </Button>
            <Button size="icon" variant="ghost" className="h-5 w-5" title="Refresh" onClick={refresh}>
              <RefreshCwIcon className="h-3 w-3" />
            </Button>
          </div>
        </div>

        {/* Search — hidden, to be re-enabled later */}

        {/* Table list grouped by schema */}
        <div className="flex-1 overflow-y-auto p-1">
          {allSchemas.length === 0 ? (
            <p className="text-xs text-muted-foreground px-2 py-1">No tables</p>
          ) : (
            allSchemas.map((schemaName) => {
              const schemaTables = tablesBySchema.get(schemaName) ?? [];
              if (searchQuery && schemaTables.length === 0) return null;
              const isExpanded = expandedSchemas.has(schemaName);

              return (
                <div key={schemaName}>
                  {/* Schema folder header */}
                  <div
                    className="flex items-center gap-1 rounded hover:bg-accent px-1 py-0.5 cursor-pointer select-none"
                    onClick={() => toggleSchemaFolder(schemaName)}
                    onContextMenu={(e) => handleSchemaContextMenu(e, schemaName)}
                  >
                    {isExpanded
                      ? <ChevronDownIcon className="h-3 w-3 text-muted-foreground shrink-0" />
                      : <ChevronRightIcon className="h-3 w-3 text-muted-foreground shrink-0" />}
                    {isExpanded
                      ? <FolderOpenIcon className="h-3 w-3 text-yellow-500 shrink-0" />
                      : <FolderIcon className="h-3 w-3 text-yellow-500 shrink-0" />}
                    <span className="text-xs font-semibold text-muted-foreground">{schemaName}</span>
                    <span className="ml-auto text-[10px] text-muted-foreground">{schemaTables.length}</span>
                  </div>

                  {/* Tables within schema */}
                  {isExpanded && schemaTables.map((t) => {
                    const tableKey = `${schemaName}.${t.name}`;
                    return (
                      <div key={tableKey} className="ml-3">
                        <div
                          className="flex items-center gap-0.5 rounded hover:bg-accent group"
                          onContextMenu={(e) => handleTableContextMenu(e, schemaName, t.name)}
                        >
                          <button
                            className="flex items-center justify-center w-4 h-6 shrink-0 text-muted-foreground hover:text-foreground"
                            onClick={() => toggleTableSchema(schemaName, t.name)}
                            title="スキーマを表示"
                          >
                            {expandedTable === tableKey
                              ? <ChevronDownIcon className="h-3 w-3" />
                              : <ChevronRightIcon className="h-3 w-3" />}
                          </button>
                          <button
                            onDoubleClick={() => selectTable(schemaName, t.name)}
                            className="flex-1 flex items-center gap-1 py-1 pr-1 text-left text-sm min-w-0"
                          >
                            <TableIcon className="h-3 w-3 shrink-0 text-yellow-500" />
                            <span className="truncate font-medium">{t.name}</span>
                            <span className="ml-auto text-xs text-muted-foreground shrink-0 pl-1">{t.row_count}</span>
                          </button>
                          <button
                            className="invisible group-hover:visible p-0.5 rounded hover:bg-accent shrink-0 text-muted-foreground hover:text-foreground"
                            title="メタデータを表示"
                            onClick={(e) => { e.stopPropagation(); setMetaTable({ schemaName, tableName: t.name }); }}
                          >
                            <InfoIcon className="h-3 w-3" />
                          </button>
                        </div>

                        {/* Column schema */}
                        {expandedTable === tableKey && (
                          <div className="ml-4 mb-1 border-l pl-2">
                            {schemaCache[tableKey]
                              ? schemaCache[tableKey].map((col) => (
                                  <div key={col.name} className="flex items-center gap-1 py-0.5 text-xs text-muted-foreground">
                                    <span className="truncate font-medium text-foreground">{col.name}</span>
                                    <span className="ml-auto shrink-0 text-[10px] font-mono">{col.column_type}</span>
                                  </div>
                                ))
                              : <span className="text-xs text-muted-foreground italic">Loading…</span>}
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              );
            })
          )}
        </div>

        {dragOver && (
          <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
            <span className="text-xs font-medium text-primary bg-background/90 px-3 py-1.5 rounded border border-primary">
              Drop CSV to import
            </span>
          </div>
        )}
      </div>

      {/* Context menu */}
      {contextMenu && (
        <div
          className="fixed z-50 bg-background border rounded shadow-lg py-1 min-w-[140px]"
          style={{ top: contextMenu.y, left: contextMenu.x }}
          onClick={(e) => e.stopPropagation()}
        >
          {contextMenu.type === "table" && (
            <>
              <button
                className="w-full text-left text-xs px-3 py-1.5 hover:bg-accent"
                onClick={() => { selectTable(contextMenu.schemaName, contextMenu.tableName); setContextMenu(null); }}
              >
                SELECT * FROM …
              </button>
              <div className="border-t my-1" />
              <button
                className="w-full text-left text-xs px-3 py-1.5 hover:bg-accent text-destructive"
                onClick={() => confirmDropTable(contextMenu.schemaName, contextMenu.tableName)}
              >
                Drop Table
              </button>
            </>
          )}
          {contextMenu.type === "schema" && (
            <button
              className={`w-full text-left text-xs px-3 py-1.5 ${contextMenu.schemaName === "main" ? "opacity-40 cursor-not-allowed" : "hover:bg-accent text-destructive"}`}
              disabled={contextMenu.schemaName === "main"}
              onClick={() => contextMenu.schemaName !== "main" && confirmDropSchema(contextMenu.schemaName)}
            >
              Drop Schema (CASCADE)
              {contextMenu.schemaName === "main" && <span className="ml-1 text-muted-foreground">(保護済み)</span>}
            </button>
          )}
        </div>
      )}

      {/* Create schema modal */}
      {showCreateSchema && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="bg-background border rounded-lg shadow-xl w-72 p-4">
            <p className="text-sm font-semibold mb-3">新しいスキーマを作成</p>
            <input
              autoFocus
              value={newSchemaName}
              onChange={(e) => setNewSchemaName(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") executeCreateSchema(); if (e.key === "Escape") setShowCreateSchema(false); }}
              placeholder="スキーマ名"
              className="w-full text-xs bg-background border rounded px-2 py-1.5 mb-3 outline-none focus:ring-1 focus:ring-primary"
            />
            <div className="flex justify-end gap-2">
              <Button size="sm" variant="outline" className="text-xs h-7" onClick={() => setShowCreateSchema(false)}>
                キャンセル
              </Button>
              <Button size="sm" className="text-xs h-7" disabled={!newSchemaName.trim()} onClick={executeCreateSchema}>
                作成
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Drop confirmation modal */}
      {dropConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="bg-background border rounded-lg shadow-xl w-80 p-4">
            <p className="text-sm font-semibold mb-2">
              {dropConfirm.type === "table" ? "テーブルを削除" : "スキーマを削除"}
            </p>
            <p className="text-xs text-muted-foreground mb-4">
              {dropConfirm.type === "table"
                ? `"${dropConfirm.schemaName}"."${dropConfirm.tableName}" を削除します。この操作は元に戻せません。`
                : `スキーマ "${dropConfirm.schemaName}" とその全テーブルを削除します（CASCADE）。この操作は元に戻せません。`}
            </p>
            <div className="flex justify-end gap-2">
              <Button size="sm" variant="outline" className="text-xs h-7" onClick={() => setDropConfirm(null)}>
                キャンセル
              </Button>
              <Button
                size="sm"
                variant="destructive"
                className="text-xs h-7"
                onClick={() =>
                  dropConfirm.type === "table"
                    ? executeDropTable(dropConfirm.schemaName, dropConfirm.tableName!)
                    : executeDropSchema(dropConfirm.schemaName)
                }
              >
                削除
              </Button>
            </div>
          </div>
        </div>
      )}

      {showImport && (
        <CsvImportDialog
          initialFilePath={pendingFile ?? undefined}
          schemas={allSchemas}
          onImported={handleImported}
          onClose={() => { setShowImport(false); setPendingFile(null); }}
        />
      )}

      {metaTable && (
        <TableMetaModal
          schemaName={metaTable.schemaName}
          tableName={metaTable.tableName}
          isDbt={false}
          onClose={() => setMetaTable(null)}
        />
      )}
    </>
  );
}
