import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { XIcon, CopyIcon } from "lucide-react";
import { useAppStore } from "@/store/appStore";

interface ColumnMeta {
  name: string;
  data_type: string;
  comment: string | null;
}

interface TableMeta {
  schema_name: string;
  table_name: string;
  comment: string | null;
  columns: ColumnMeta[];
  created_at: string | null;
  updated_at: string | null;
}

export function TableMetaPanel() {
  const metaPanel = useAppStore((s) => s.metaPanel);
  const setMetaPanel = useAppStore((s) => s.setMetaPanel);

  const [meta, setMeta] = useState<TableMeta | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tableComment, setTableComment] = useState("");
  const [colComments, setColComments] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);

  useEffect(() => {
    if (!metaPanel) return;
    loadMeta(metaPanel.schemaName, metaPanel.tableName, metaPanel.isDbt);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [metaPanel?.schemaName, metaPanel?.tableName, metaPanel?.isDbt]);

  async function loadMeta(schemaName: string, tableName: string, isDbt: boolean) {
    setLoading(true);
    setError(null);
    setMeta(null);
    setSaveMsg(null);
    try {
      const result = await invoke<TableMeta>(
        isDbt ? "get_dbt_table_meta" : "get_table_meta",
        { schemaName, tableName }
      );
      setMeta(result);
      setTableComment(result.comment ?? "");
      const comments: Record<string, string> = {};
      for (const col of result.columns) {
        comments[col.name] = col.comment ?? "";
      }
      setColComments(comments);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function handleSave() {
    if (!meta || !metaPanel) return;
    const { schemaName, tableName } = metaPanel;
    setSaving(true);
    setSaveMsg(null);
    try {
      const colCommentsPayload = meta.columns.map((col) => ({
        name: col.name,
        comment: colComments[col.name] ?? "",
      }));
      await invoke("save_table_meta", {
        schemaName,
        tableName,
        comment: tableComment,
        colComments: colCommentsPayload,
      });
      setSaveMsg("保存しました");
      await loadMeta(schemaName, tableName, false);
    } catch (e) {
      setSaveMsg(`エラー: ${String(e)}`);
    } finally {
      setSaving(false);
    }
  }

  if (!metaPanel) return null;

  const { schemaName, tableName, isDbt } = metaPanel;
  const fullPath = isDbt
    ? `dbt."${schemaName}"."${tableName}"`
    : `"${schemaName}"."${tableName}"`;

  return (
    <div className="flex flex-col border-t bg-background h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-1.5 border-b shrink-0">
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-xs font-semibold truncate">{tableName}</span>
          <span className="text-[10px] text-muted-foreground shrink-0">
            {schemaName}{isDbt ? " · dbt" : ""}
          </span>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          {saveMsg && (
            <span className={`text-[10px] ${saveMsg.startsWith("エラー") ? "text-destructive" : "text-green-400"}`}>
              {saveMsg}
            </span>
          )}
          {!isDbt && (
            <Button
              size="sm"
              className="text-xs h-6 px-2"
              disabled={saving || loading}
              onClick={handleSave}
            >
              {saving ? "保存中..." : "保存"}
            </Button>
          )}
          <button onClick={() => setMetaPanel(null)} className="opacity-50 hover:opacity-100 p-0.5">
            <XIcon className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      {/* Body */}
      <div className="flex-1 overflow-y-auto px-3 py-2 space-y-2">
        {loading && <p className="text-xs text-muted-foreground">Loading...</p>}
        {error && <p className="text-xs text-destructive">{error}</p>}
        {meta && (
          <>
            {/* Table description */}
            <div>
              <label className="text-[10px] font-semibold text-muted-foreground block mb-1">Description</label>
              {isDbt ? (
                <p className="text-xs">
                  {meta.comment
                    ? meta.comment
                    : <span className="text-muted-foreground italic">No description</span>}
                </p>
              ) : (
                <textarea
                  value={tableComment}
                  onChange={(e) => setTableComment(e.target.value)}
                  rows={2}
                  placeholder="テーブルの説明を入力..."
                  className="w-full text-xs bg-background border rounded px-2 py-1 outline-none focus:ring-1 focus:ring-primary resize-none"
                />
              )}
            </div>

            {/* Path + Timestamps */}
            <div className="space-y-0.5">
              <div className="flex items-center gap-1">
                <span className="text-[10px] text-muted-foreground">Path </span>
                <span className="text-[10px] font-mono">{fullPath}</span>
                <button
                  className="opacity-40 hover:opacity-100 p-0.5"
                  title="コピー"
                  onClick={() => navigator.clipboard.writeText(fullPath)}
                >
                  <CopyIcon className="h-2.5 w-2.5" />
                </button>
              </div>
              <div>
                <span className="text-[10px] text-muted-foreground">Created </span>
                <span className="text-[10px] font-mono">{meta.created_at ?? ""}</span>
              </div>
              <div>
                <span className="text-[10px] text-muted-foreground">Updated </span>
                <span className="text-[10px] font-mono">{meta.updated_at ?? ""}</span>
              </div>
            </div>

            {/* Columns */}
            <div>
              <label className="text-[10px] font-semibold text-muted-foreground block mb-1">Columns</label>
              <div className="space-y-1">
                {meta.columns.map((col) => (
                  <div key={col.name} className="grid grid-cols-[120px_80px_1fr] gap-2 items-center">
                    <span className="text-xs font-medium truncate">{col.name}</span>
                    <span className="text-[10px] font-mono text-muted-foreground bg-muted rounded px-1 py-0.5 truncate">
                      {col.data_type}
                    </span>
                    {isDbt ? (
                      <span className="text-xs text-muted-foreground truncate">{col.comment || "—"}</span>
                    ) : (
                      <input
                        value={colComments[col.name] ?? ""}
                        onChange={(e) =>
                          setColComments((prev) => ({ ...prev, [col.name]: e.target.value }))
                        }
                        placeholder="説明..."
                        className="text-xs bg-background border rounded px-2 py-0.5 outline-none focus:ring-1 focus:ring-primary"
                      />
                    )}
                  </div>
                ))}
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
