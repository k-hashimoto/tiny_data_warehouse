import { invoke } from "@tauri-apps/api/core";
import { useAppStore, QueryResult, TableInfo } from "@/store/appStore";

export function useRunQuery() {
  const setTabResult = useAppStore((s) => s.setTabResult);
  const activeTabId = useAppStore((s) => s.activeTabId);
  const setTables = useAppStore((s) => s.setTables);
  const setError = useAppStore((s) => s.setError);
  const setStatus = useAppStore((s) => s.setStatus);
  const setIsRunning = useAppStore((s) => s.setIsRunning);
  const addToHistory = useAppStore((s) => s.addToHistory);
  const isRunning = useAppStore((s) => s.isRunning);

  return async function runQuery(sql: string) {
    if (isRunning) return;
    setError("");
    setIsRunning(true);
    setStatus("Running...");
    // Capture the tab that initiated the query
    const tabId = activeTabId;
    try {
      const res = await invoke<QueryResult>("run_query", { sql });
      setTabResult(tabId, res);
      const truncatedNote = res.truncated ? " (1000行に制限)" : "";
      setStatus(`${res.row_count} rows · ${res.elapsed_ms}ms${truncatedNote}`);
      addToHistory(sql);
      invoke<TableInfo[]>("list_tables").then(setTables).catch(() => {});
    } catch (e) {
      setError(String(e));
      setStatus("Error");
      setTabResult(tabId, null);
    } finally {
      setIsRunning(false);
    }
  };
}
