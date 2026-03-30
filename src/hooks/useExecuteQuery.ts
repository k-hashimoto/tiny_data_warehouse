import { invoke } from "@tauri-apps/api/core";
import { useAppStore, QueryResult, TableInfo } from "@/store/appStore";

export interface ExecuteOptions {
  updateTabResult?: boolean;  // default true; set false for background/scheduler runs
  tabId?: string;             // override which tab receives the result
}

export function useExecuteQuery() {
  const setTabResult = useAppStore((s) => s.setTabResult);
  const activeTabId = useAppStore((s) => s.activeTabId);
  const setTables = useAppStore((s) => s.setTables);
  const setError = useAppStore((s) => s.setError);
  const setStatus = useAppStore((s) => s.setStatus);
  const setIsRunning = useAppStore((s) => s.setIsRunning);
  const addToHistory = useAppStore((s) => s.addToHistory);
  const isRunning = useAppStore((s) => s.isRunning);
  const setMetaPanel = useAppStore((s) => s.setMetaPanel);

  return async function executeQuery(sql: string, options?: ExecuteOptions): Promise<QueryResult | null> {
    if (isRunning) return null;
    const { updateTabResult = true, tabId = activeTabId } = options ?? {};
    if (updateTabResult) setMetaPanel(null);
    setError("");
    setIsRunning(true);
    setStatus("Running...");
    try {
      const res = await invoke<QueryResult>("run_query", { sql });
      if (updateTabResult) setTabResult(tabId, res);
      const truncatedNote = res.truncated ? " (1000行に制限)" : "";
      setStatus(`${res.row_count} rows · ${res.elapsed_ms}ms${truncatedNote}`);
      addToHistory(sql);
      invoke<TableInfo[]>("list_tables").then(setTables).catch(() => {});
      return res;
    } catch (e) {
      setError(String(e));
      setStatus("Error");
      if (updateTabResult) setTabResult(tabId, null);
      return null;
    } finally {
      setIsRunning(false);
    }
  };
}
