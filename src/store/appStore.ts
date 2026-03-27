import { create } from "zustand";
import { persist } from "zustand/middleware";
import { MAX_TABS } from "@/constants";

export interface QueryResult {
  columns: string[];
  rows: unknown[][];
  row_count: number;
  elapsed_ms: number;
  truncated: boolean;
}

export interface TableInfo {
  name: string;
  schema_name: string;
  row_count: number;
  column_count: number;
  csv_source_path: string | null;
}

export interface CsvImportOptions {
  file_path: string;
  table_name: string;
  schema_name: string;
  has_header: boolean;
  delimiter: string;  // "comma" | "tab" | "semicolon"
  encoding: string;   // "utf8" | "sjis"
  if_exists: string;  // "replace" | "append" | "fail"
}

export interface CsvPreviewResult {
  preview: QueryResult;
  suggested_table_name: string;
}

export type JobKind =
  | { type: "saved_query"; name: string }
  | { type: "csv_import"; path: string }
  | { type: "workflow"; id: string };

export interface ScheduledJob {
  id: string;
  kind: JobKind;
  cron: string;
  enabled: boolean;
}

export interface Tab {
  id: string;
  title: string;
  sql: string;
  linkedScript: string | null;
  isDirty: boolean;
  result: QueryResult | null;  // not persisted
}

const DEFAULT_SQL = [
  "SELECT",
  "  'Tiny Data Warehouse' as app_name,",
  "  'https://github.com/k-hashimoto/tiny_data_warehouse' as link",
  "UNION ALL",
  "SELECT",
  "  'Sidebar Guide' as app_name,",
  "  'https://github.com/k-hashimoto/tiny_data_warehouse/blob/main/docs/sidebar.md' as link",
  "UNION ALL",
  "SELECT",
  "  'dbt Guide' as app_name,",
  "  'https://github.com/k-hashimoto/tiny_data_warehouse/blob/main/docs/dbt-integration.md' as link",
].join("\n");

function makeTab(id: string, n: number): Tab {
  return { id, title: `Untitled ${n}`, sql: DEFAULT_SQL, linkedScript: null, isDirty: false, result: null };
}

let tabCounter = 1;

function nextTabId(): string {
  return `tab-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
}

interface AppState {
  // Tabs
  tabs: Tab[];
  activeTabId: string;
  addTab: () => void;
  closeTab: (id: string) => void;
  setActiveTab: (id: string) => void;
  updateTabSql: (id: string, sql: string) => void;
  renameTab: (id: string, title: string) => void;
  setTabLinkedScript: (id: string, scriptName: string | null, sql?: string) => void;
  setTabResult: (id: string, result: QueryResult | null) => void;
  getActiveTab: () => Tab;

  // Results / status (result is now per-tab; kept for compat via getter)
  result: QueryResult | null;
  tables: TableInfo[];
  scripts: string[];
  dbtTables: TableInfo[];
  error: string;
  status: string;
  dbPath: string;
  isRunning: boolean;
  queryHistory: string[];

  // Derived from active tab for convenience
  sql: string;
  activeScript: string | null;
  setActiveScript: (name: string | null) => void;

  setResult: (result: QueryResult | null) => void;
  setTables: (tables: TableInfo[]) => void;
  setScripts: (scripts: string[]) => void;
  setDbtTables: (tables: TableInfo[]) => void;
  setError: (error: string) => void;
  setStatus: (status: string) => void;
  setDbPath: (path: string) => void;
  setIsRunning: (v: boolean) => void;
  addToHistory: (sql: string) => void;

  scheduledJobs: ScheduledJob[];
  setScheduledJobs: (jobs: ScheduledJob[]) => void;

  historyOpen: boolean;
  setHistoryOpen: (v: boolean) => void;

  darkMode: boolean;
  setDarkMode: (v: boolean) => void;
}

const initialTabId = nextTabId();
const initialTab: Tab = makeTab(initialTabId, 1);

export const useAppStore = create<AppState>()(
  persist(
    (set, get) => ({
      tabs: [initialTab],
      activeTabId: initialTabId,

      addTab: () => {
        const { tabs } = get();
        if (tabs.length >= MAX_TABS) return;
        tabCounter = tabs.length + 1;
        const id = nextTabId();
        const tab = { ...makeTab(id, tabCounter), sql: "" };
        set((s) => ({ tabs: [...s.tabs, tab], activeTabId: id }));
      },

      closeTab: (id: string) => {
        const { tabs, activeTabId } = get();
        if (tabs.length === 1) return; // 最後の1枚は閉じない
        const idx = tabs.findIndex((t) => t.id === id);
        const newTabs = tabs.filter((t) => t.id !== id);
        let newActiveId = activeTabId;
        if (activeTabId === id) {
          // 閉じたタブの右隣 or 左隣をアクティブに
          newActiveId = newTabs[Math.min(idx, newTabs.length - 1)].id;
        }
        set({ tabs: newTabs, activeTabId: newActiveId });
      },

      setActiveTab: (id: string) => set({ activeTabId: id }),

      updateTabSql: (id: string, sql: string) => {
        set((s) => ({
          tabs: s.tabs.map((t) =>
            t.id === id ? { ...t, sql, isDirty: t.linkedScript !== null } : t
          ),
        }));
      },

      renameTab: (id: string, title: string) => {
        set((s) => ({
          tabs: s.tabs.map((t) => t.id === id ? { ...t, title } : t),
        }));
      },

      setTabLinkedScript: (id: string, scriptName: string | null, sql?: string) => {
        set((s) => ({
          tabs: s.tabs.map((t) =>
            t.id === id
              ? { ...t, linkedScript: scriptName, isDirty: false, ...(sql !== undefined ? { sql } : {}) }
              : t
          ),
        }));
      },

      setTabResult: (id: string, result: QueryResult | null) => {
        set((s) => ({
          tabs: s.tabs.map((t) => t.id === id ? { ...t, result } : t),
        }));
      },

      getActiveTab: () => {
        const { tabs, activeTabId } = get();
        return tabs.find((t) => t.id === activeTabId) ?? tabs[0];
      },

      // Derived from active tab
      get sql() { return get().getActiveTab().sql; },
      get activeScript() { return get().getActiveTab().linkedScript; },
      setActiveScript: (name: string | null) => {
        const { activeTabId } = get();
        get().setTabLinkedScript(activeTabId, name);
      },
      get result() { return get().getActiveTab().result; },
      setResult: (result: QueryResult | null) => {
        const { activeTabId } = get();
        get().setTabResult(activeTabId, result);
      },
      tables: [],
      scripts: [],
      dbtTables: [],
      error: "",
      status: "Ready",
      dbPath: ":memory:",
      isRunning: false,
      queryHistory: [],

      scheduledJobs: [],
      setScheduledJobs: (scheduledJobs) => set({ scheduledJobs }),

      historyOpen: false,
      setHistoryOpen: (historyOpen) => set({ historyOpen }),

      darkMode: false,
      setDarkMode: (darkMode) => set({ darkMode }),

      setTables: (tables) => set({ tables }),
      setScripts: (scripts) => set({ scripts }),
      setDbtTables: (dbtTables) => set({ dbtTables }),
      setError: (error) => set({ error }),
      setStatus: (status) => set({ status }),
      setDbPath: (dbPath) => set({ dbPath }),
      setIsRunning: (isRunning) => set({ isRunning }),
      addToHistory: (sql: string) => {
        const trimmed = sql.trim();
        if (!trimmed) return;
        set((state) => {
          const filtered = state.queryHistory.filter((q) => q !== trimmed);
          return { queryHistory: [trimmed, ...filtered].slice(0, 100) };
        });
      },
    }),
    {
      name: "tiny-data-ware-house-storage",
      partialize: (state) => ({
        queryHistory: state.queryHistory,
        darkMode: state.darkMode,
        // Strip result from tabs before persisting (results can be large)
        tabs: state.tabs.map(({ result: _r, ...rest }) => ({ ...rest, result: null })),
        activeTabId: state.activeTabId,
      }),
      onRehydrateStorage: () => (state) => {
        // ダーティフラグをリセット（起動時は全タブをクリーンに）
        if (state) {
          state.tabs = state.tabs.map((t) => ({ ...t, isDirty: false }));
        }
      },
    }
  )
);
