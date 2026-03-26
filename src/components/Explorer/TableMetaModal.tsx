import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { XIcon } from "lucide-react";

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
}

interface Props {
  schemaName: string;
  tableName: string;
  isDbt: boolean;
  onClose: () => void;
}

export function TableMetaModal({ schemaName, tableName, isDbt, onClose }: Props) {
  const [meta, setMeta] = useState<TableMeta | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tableComment, setTableComment] = useState("");
  const [colComments, setColComments] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);

  useEffect(() => {
    loadMeta();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [schemaName, tableName]);

  async function loadMeta() {
    setLoading(true);
    setError(null);
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
    if (!meta) return;
    setSaving(true);
    setSaveMsg(null);
    try {
      await invoke("set_table_comment", { schemaName, tableName, comment: tableComment });
      for (const col of meta.columns) {
        const newComment = colComments[col.name] ?? "";
        if (newComment !== (col.comment ?? "")) {
          await invoke("set_column_comment", {
            schemaName,
            tableName,
            columnName: col.name,
            comment: newComment,
          });
        }
      }
      setSaveMsg("保存しました");
      await loadMeta();
    } catch (e) {
      setSaveMsg(`エラー: ${String(e)}`);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-background border rounded-lg shadow-xl w-[560px] max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b shrink-0">
          <div>
            <p className="text-sm font-semibold">{tableName}</p>
            <p className="text-xs text-muted-foreground">
              {schemaName}{isDbt ? " · dbt" : ""}
            </p>
          </div>
          <button onClick={onClose} className="opacity-50 hover:opacity-100 p-1">
            <XIcon className="h-4 w-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto px-4 py-3">
          {loading && <p className="text-xs text-muted-foreground">Loading...</p>}
          {error && <p className="text-xs text-destructive">{error}</p>}
          {meta && (
            <>
              {/* Table description */}
              <div className="mb-4">
                <label className="text-xs font-semibold text-muted-foreground block mb-1">
                  Description
                </label>
                {isDbt ? (
                  <p className="text-sm">
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
                    className="w-full text-xs bg-background border rounded px-2 py-1.5 outline-none focus:ring-1 focus:ring-primary resize-none"
                  />
                )}
              </div>

              {/* Columns */}
              <div>
                <label className="text-xs font-semibold text-muted-foreground block mb-2">
                  Columns
                </label>
                <div className="space-y-1.5">
                  {meta.columns.map((col) => (
                    <div key={col.name} className="grid grid-cols-[130px_90px_1fr] gap-2 items-center">
                      <span className="text-xs font-medium truncate">{col.name}</span>
                      <span className="text-[10px] font-mono text-muted-foreground bg-muted rounded px-1 py-0.5 truncate">
                        {col.data_type}
                      </span>
                      {isDbt ? (
                        <span className="text-xs text-muted-foreground">
                          {col.comment || "—"}
                        </span>
                      ) : (
                        <input
                          value={colComments[col.name] ?? ""}
                          onChange={(e) =>
                            setColComments((prev) => ({ ...prev, [col.name]: e.target.value }))
                          }
                          placeholder="説明..."
                          className="text-xs bg-background border rounded px-2 py-1 outline-none focus:ring-1 focus:ring-primary"
                        />
                      )}
                    </div>
                  ))}
                </div>
              </div>
            </>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between px-4 py-3 border-t shrink-0">
          <span className={`text-xs ${saveMsg?.startsWith("エラー") ? "text-destructive" : "text-green-400"}`}>
            {saveMsg ?? ""}
          </span>
          <div className="flex gap-2">
            <Button size="sm" variant="outline" className="text-xs h-7" onClick={onClose}>
              閉じる
            </Button>
            {!isDbt && (
              <Button
                size="sm"
                className="text-xs h-7"
                disabled={saving || loading}
                onClick={handleSave}
              >
                {saving ? "保存中..." : "保存"}
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
