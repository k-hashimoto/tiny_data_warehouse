import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useAppStore, TableInfo } from "@/store/appStore";
import { Button } from "@/components/ui/button";
import {
  RefreshCwIcon, TableIcon,
  ChevronRightIcon, ChevronDownIcon,
  FolderIcon, FolderOpenIcon,
  LinkIcon, UnlinkIcon, Trash2Icon, PackageIcon, LoaderIcon, InfoIcon,
} from "lucide-react";
import { TableMetaModal } from "@/components/Explorer/TableMetaModal";

export function DbtSection() {
  const dbtTables = useAppStore((s) => s.dbtTables);
  const setDbtTables = useAppStore((s) => s.setDbtTables);
  const setSql = useAppStore((s) => s.setSql);
  const setError = useAppStore((s) => s.setError);
  const setStatus = useAppStore((s) => s.setStatus);
  const tabs = useAppStore((s) => s.tabs);
  const activeTabId = useAppStore((s) => s.activeTabId);

  const [expandedSchemas, setExpandedSchemas] = useState<Set<string>>(new Set());
  const [expandedTable, setExpandedTable] = useState<string | null>(null);
  const [schemaCache, setSchemaCache] = useState<Record<string, { name: string; column_type: string }[]>>({});
  const [attached, setAttached] = useState(false);
  const [attaching, setAttaching] = useState(false);
  const [dbtRunning, setDbtRunning] = useState(false);
  const [metaTable, setMetaTable] = useState<{ schemaName: string; tableName: string } | null>(null);

  async function attachDbt() {
    try {
      await invoke("attach_dbt");
      setAttached(true);
    } catch (_) {
      // dbt.db may not exist yet — silently ignore
      setAttached(false);
    }
  }

  async function handleAttach() {
    setAttaching(true);
    try {
      await invoke("attach_dbt");
      setAttached(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setAttaching(false);
    }
  }

  async function handleDetach() {
    setAttaching(true);
    try {
      await invoke("detach_dbt");
      setAttached(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setAttaching(false);
    }
  }

  async function refresh() {
    try {
      const tables = await invoke<TableInfo[]>("list_dbt_tables");
      setDbtTables(tables);
      // Auto-expand all schemas on first load
      if (tables.length > 0) {
        setExpandedSchemas(new Set(tables.map((t) => t.schema_name)));
      }
    } catch (_) {
      setDbtTables([]);
    }
  }

  useEffect(() => {
    // Auto-attach on startup
    attachDbt().then(() => refresh());

    // dbt run started — show running banner
    const unlistenRunning = listen("dbt-db-running", () => {
      setDbtRunning(true);
    });

    // dbt run finished — re-attach and refresh
    const unlistenChanged = listen("dbt-db-changed", async () => {
      setDbtRunning(false);
      try { await invoke("detach_dbt"); } catch (_) {}
      await attachDbt();
      await refresh();
    });

    return () => {
      unlistenRunning.then((fn) => fn());
      unlistenChanged.then((fn) => fn());
    };
  }, []);

  const tablesBySchema = useMemo(() => {
    const map = new Map<string, TableInfo[]>();
    for (const t of dbtTables) {
      const list = map.get(t.schema_name) ?? [];
      list.push(t);
      map.set(t.schema_name, list);
    }
    return map;
  }, [dbtTables]);

  const allSchemas = useMemo(() => [...tablesBySchema.keys()].sort(), [tablesBySchema]);

  function toggleSchema(name: string) {
    setExpandedSchemas((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  }

  async function dropTable(schemaName: string, tableName: string) {
    if (!window.confirm(`テーブル "${schemaName}.${tableName}" を削除しますか？`)) return;
    try {
      await invoke("drop_dbt_table", { schemaName, tableName });
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  async function dropSchema(schemaName: string) {
    if (!window.confirm(`スキーマ "${schemaName}" とその全テーブルを削除しますか？`)) return;
    try {
      await invoke("drop_dbt_schema", { schemaName });
      await refresh();
    } catch (e) {
      setError(String(e));
    }
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
        const result = await invoke<{ table_name: string; columns: { name: string; column_type: string }[] }>(
          "get_dbt_schema", { schemaName, tableName }
        );
        setSchemaCache((prev) => ({ ...prev, [key]: result.columns }));
      } catch (e) {
        setError(String(e));
      }
    }
  }

  function selectTable(schemaName: string, tableName: string) {
    const schema = schemaName.replace(/"/g, '""');
    const table = tableName.replace(/"/g, '""');
    const newSql = `SELECT * FROM dbt."${schema}"."${table}" LIMIT 100`;
    const currentSql = tabs.find((t) => t.id === activeTabId)?.sql ?? "";
    const isEmpty = currentSql.trim() === "";
    setSql(isEmpty ? newSql : `${currentSql}\n\n${newSql}`);
    setStatus(isEmpty ? `Inserted: ${newSql}` : `Appended: ${newSql}`);
  }

  return (
    <div className="flex flex-col h-full overflow-hidden select-none">
      {/* Header */}
      <div className="flex items-center gap-1 px-2 py-1 border-b shrink-0">
        <span className="flex items-center gap-1 text-xs font-semibold text-muted-foreground flex-1">
          <PackageIcon className="h-3 w-3" />
          DBT TABLES
        </span>
        {attached ? (
          <span className="text-[10px] text-green-400 font-medium px-1 rounded bg-green-400/10">attached</span>
        ) : (
          <span className="text-[10px] text-blue-400 font-medium px-1 rounded bg-blue-400/10">read-only</span>
        )}
        {attached ? (
          <Button
            size="icon" variant="ghost" className="h-5 w-5 text-red-400 hover:text-red-500"
            title="Detach dbt.db" onClick={handleDetach} disabled={attaching}
          >
            <UnlinkIcon className="h-3 w-3" />
          </Button>
        ) : (
          <Button
            size="icon" variant="ghost" className="h-5 w-5"
            title="Attach dbt.db (allows FROM dbt.schema.table in queries)" onClick={handleAttach} disabled={attaching}
          >
            <LinkIcon className="h-3 w-3" />
          </Button>
        )}
        <Button size="icon" variant="ghost" className="h-5 w-5" title="Refresh" onClick={refresh}>
          <RefreshCwIcon className="h-3 w-3" />
        </Button>
      </div>

      {/* dbt running banner */}
      {dbtRunning && (
        <div className="flex items-center gap-1.5 px-2 py-1 bg-yellow-500/10 border-b border-yellow-500/20 shrink-0">
          <LoaderIcon className="h-3 w-3 text-yellow-400 animate-spin shrink-0" />
          <span className="text-[11px] text-yellow-400 font-medium">dbt 実行中...</span>
        </div>
      )}

      {/* Table metadata modal */}
      {metaTable && (
        <TableMetaModal
          schemaName={metaTable.schemaName}
          tableName={metaTable.tableName}
          isDbt={true}
          onClose={() => setMetaTable(null)}
        />
      )}

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-1">
        {dbtTables.length === 0 ? (
          <p className="text-xs text-muted-foreground px-2 py-2">
            dbt.db が見つかりません
          </p>
        ) : (
          allSchemas.map((schemaName) => {
            const schemaTables = tablesBySchema.get(schemaName) ?? [];
            const isExpanded = expandedSchemas.has(schemaName);

            return (
              <div key={schemaName}>
                <div
                  className="group flex items-center gap-1 rounded hover:bg-accent px-1 py-0.5 cursor-pointer"
                  onClick={() => toggleSchema(schemaName)}
                >
                  {isExpanded
                    ? <ChevronDownIcon className="h-3 w-3 text-muted-foreground shrink-0" />
                    : <ChevronRightIcon className="h-3 w-3 text-muted-foreground shrink-0" />}
                  {isExpanded
                    ? <FolderOpenIcon className="h-3 w-3 text-blue-400 shrink-0" />
                    : <FolderIcon className="h-3 w-3 text-blue-400 shrink-0" />}
                  <span className="text-xs font-semibold text-muted-foreground">{schemaName}</span>
                  <span className="ml-auto text-[10px] text-muted-foreground">{schemaTables.length}</span>
                  <button
                    className="invisible group-hover:visible ml-1 p-0.5 rounded hover:bg-red-500/20 text-red-400 hover:text-red-500"
                    title={`Drop schema ${schemaName}`}
                    onClick={(e) => { e.stopPropagation(); dropSchema(schemaName); }}
                  >
                    <Trash2Icon className="h-3 w-3" />
                  </button>
                </div>

                {isExpanded && schemaTables.map((t) => {
                  const tableKey = `${schemaName}.${t.name}`;
                  return (
                    <div key={t.name} className="ml-3">
                      <div className="group flex items-center gap-0.5 rounded hover:bg-accent">
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
                          className="flex items-center gap-1 flex-1 text-left text-sm min-w-0 py-1 pr-1"
                          onDoubleClick={() => selectTable(schemaName, t.name)}
                          title={`ダブルクリックでSELECT from dbt.${schemaName}.${t.name}`}
                        >
                          <TableIcon className="h-3 w-3 shrink-0 text-blue-400" />
                          <span className="truncate font-medium">{t.name}</span>
                          <span className="ml-auto text-xs text-muted-foreground shrink-0 pl-1">{t.row_count}</span>
                        </button>
                        <button
                          className="invisible group-hover:visible p-0.5 rounded hover:bg-accent shrink-0 text-muted-foreground hover:text-foreground"
                          title="メタデータを表示"
                          onClick={() => setMetaTable({ schemaName, tableName: t.name })}
                        >
                          <InfoIcon className="h-3 w-3" />
                        </button>
                        <button
                          className="invisible group-hover:visible p-0.5 rounded hover:bg-red-500/20 text-red-400 hover:text-red-500 shrink-0"
                          title={`Drop table ${schemaName}.${t.name}`}
                          onClick={() => dropTable(schemaName, t.name)}
                        >
                          <Trash2Icon className="h-3 w-3" />
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
    </div>
  );
}
