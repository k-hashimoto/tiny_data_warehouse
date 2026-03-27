import { useState, useCallback, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore, TableInfo } from "@/store/appStore";

export interface ColumnInfo {
  name: string;
  column_type: string;
  nullable: boolean;
}

export interface TableContextMenu {
  x: number;
  y: number;
  type: "table";
  schemaName: string;
  tableName: string;
  csvSourcePath: string | null;
}

export interface SchemaContextMenu {
  x: number;
  y: number;
  type: "schema";
  schemaName: string;
}

export type ContextMenuState = TableContextMenu | SchemaContextMenu;

export function useTableTree() {
  const tables = useAppStore((s) => s.tables);
  const setTables = useAppStore((s) => s.setTables);
  const setSql = useAppStore((s) => s.setSql);
  const setError = useAppStore((s) => s.setError);
  const setStatus = useAppStore((s) => s.setStatus);
  const tabs = useAppStore((s) => s.tabs);
  const activeTabId = useAppStore((s) => s.activeTabId);

  const [showImport, setShowImport] = useState(false);
  const [dragOver, setDragOver] = useState(false);
  const [pendingFile, setPendingFile] = useState<string | null>(null);
  const [schemas, setSchemas] = useState<string[]>([]);
  const [expandedTable, setExpandedTable] = useState<string | null>(null);
  const [schemaCache, setSchemaCache] = useState<Record<string, ColumnInfo[]>>({});
  const [expandedSchemas, setExpandedSchemas] = useState<Set<string>>(new Set(["main"]));
  const [searchQuery] = useState("");
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [dropConfirm, setDropConfirm] = useState<{ type: "table" | "schema"; schemaName: string; tableName?: string } | null>(null);
  const [showCreateSchema, setShowCreateSchema] = useState(false);
  const [newSchemaName, setNewSchemaName] = useState("");
  const [metaTable, setMetaTable] = useState<{ schemaName: string; tableName: string } | null>(null);

  useEffect(() => { refresh(); }, []);

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

  const allSchemas = useMemo(() => {
    const set = new Set([...schemas, ...tables.map((t) => t.schema_name)]);
    return [...set].sort((a, b) => {
      if (a === "main") return -1;
      if (b === "main") return 1;
      return a.localeCompare(b);
    });
  }, [tables, schemas]);

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
    if (expandedTable === key) { setExpandedTable(null); return; }
    setExpandedTable(key);
    if (!schemaCache[key]) {
      try {
        const result = await invoke<{ table_name: string; columns: ColumnInfo[] }>("get_schema", { schemaName, tableName });
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
    const isEmpty = currentSql.trim() === "";
    setSql(isEmpty ? newSql : `${currentSql}\n\n${newSql}`);
    setStatus(isEmpty ? `Inserted: ${newSql}` : `Appended: ${newSql}`);
  }

  function handleImported(table: TableInfo) {
    setTables(
      [...tables.filter((t) => !(t.schema_name === table.schema_name && t.name === table.name)), table]
        .sort((a, b) => a.schema_name.localeCompare(b.schema_name) || a.name.localeCompare(b.name))
    );
    setShowImport(false);
    setPendingFile(null);
  }

  function handleTableContextMenu(e: React.MouseEvent, schemaName: string, tableName: string, csvSourcePath: string | null) {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY, type: "table", schemaName, tableName, csvSourcePath });
  }

  function handleSchemaContextMenu(e: React.MouseEvent, schemaName: string) {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY, type: "schema", schemaName });
  }

  function confirmDropTable(schemaName: string, tableName: string) {
    setContextMenu(null);
    setDropConfirm({ type: "table", schemaName, tableName });
  }

  async function handleReimport(schemaName: string, tableName: string) {
    setContextMenu(null);
    setStatus("Reloading from CSV...");
    try {
      await invoke("reimport_csv", { schemaName, tableName });
      await refresh();
      setStatus("Reloaded from CSV");
    } catch (e) {
      setError(String(e));
      setStatus("Error");
    }
  }

  function confirmDropSchema(schemaName: string) {
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
    if (path) { setPendingFile(path); setShowImport(true); }
  }, []);

  return {
    allSchemas, tablesBySchema, expandedSchemas, expandedTable, schemaCache, searchQuery,
    contextMenu, dropConfirm, showCreateSchema, newSchemaName, metaTable,
    showImport, dragOver, pendingFile,
    refresh, toggleSchemaFolder, toggleTableSchema, selectTable,
    handleImported, handleTableContextMenu, handleSchemaContextMenu,
    confirmDropTable, handleReimport, confirmDropSchema,
    executeDropTable, executeDropSchema,
    createSchema, executeCreateSchema,
    setNewSchemaName, setShowCreateSchema, setDropConfirm,
    setShowImport, setPendingFile, setMetaTable, setContextMenu,
    handleDragOver, handleDragLeave, handleDrop,
  };
}
