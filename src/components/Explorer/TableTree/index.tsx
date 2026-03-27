import { Button } from "@/components/ui/button";
import { RefreshCwIcon, UploadIcon, PlusIcon, DatabaseIcon } from "lucide-react";
import { useTableTree } from "./hooks/useTableTree";
import { SchemaFolder } from "./SchemaFolder";
import { FileImportDialog } from "@/components/FileImport/FileImportDialog";
import { TableMetaModal } from "@/components/Explorer/TableMetaModal";

export function TableTree() {
  const {
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
  } = useTableTree();

  return (
    <>
      <div
        className={`relative flex flex-col h-full transition-colors ${dragOver ? "bg-accent/20 ring-2 ring-inset ring-primary" : ""}`}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        <div className="flex items-center justify-between px-2 py-1 border-b shrink-0">
          <span className="flex items-center gap-1 text-xs font-semibold text-muted-foreground">
            <DatabaseIcon className="h-3 w-3" />
            ADHOC TABLES
          </span>
          <div className="flex items-center gap-0.5">
            <Button size="icon" variant="ghost" className="h-5 w-5" title="新しいスキーマ" onClick={createSchema}>
              <PlusIcon className="h-3 w-3" />
            </Button>
            <Button size="icon" variant="ghost" className="h-5 w-5" title="Import CSV" onClick={() => { setPendingFile(null); setShowImport(true); }}>
              <UploadIcon className="h-3 w-3" />
            </Button>
            <Button size="icon" variant="ghost" className="h-5 w-5" title="Refresh" onClick={refresh}>
              <RefreshCwIcon className="h-3 w-3" />
            </Button>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-1">
          {allSchemas.length === 0 ? (
            <p className="text-xs text-muted-foreground px-2 py-1">No tables</p>
          ) : (
            allSchemas.map((schemaName) => {
              const schemaTables = tablesBySchema.get(schemaName) ?? [];
              if (searchQuery && schemaTables.length === 0) return null;
              return (
                <SchemaFolder
                  key={schemaName}
                  schemaName={schemaName}
                  tables={schemaTables}
                  isExpanded={expandedSchemas.has(schemaName)}
                  expandedTable={expandedTable}
                  schemaCache={schemaCache}
                  onToggle={() => toggleSchemaFolder(schemaName)}
                  onContextMenu={(e) => handleSchemaContextMenu(e, schemaName)}
                  onTableContextMenu={(e, tableName, csvSourcePath) => handleTableContextMenu(e, schemaName, tableName, csvSourcePath)}
                  onToggleTable={(tableName) => toggleTableSchema(schemaName, tableName)}
                  onSelectTable={(tableName) => selectTable(schemaName, tableName)}
                  onInfoClick={(tableName) => setMetaTable({ schemaName, tableName })}
                />
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

      {contextMenu && (
        <div
          className="fixed z-50 bg-background border rounded shadow-lg py-1 min-w-[140px]"
          style={{ top: contextMenu.y, left: contextMenu.x }}
          onClick={(e) => e.stopPropagation()}
        >
          {contextMenu.type === "table" && (
            <>
              <button className="w-full text-left text-xs px-3 py-1.5 hover:bg-accent" onClick={() => { selectTable(contextMenu.schemaName, contextMenu.tableName); setContextMenu(null); }}>
                SELECT * FROM …
              </button>
              {contextMenu.csvSourcePath && (
                <>
                  <div className="border-t my-1" />
                  <button className="w-full text-left text-xs px-3 py-1.5 hover:bg-accent" onClick={() => handleReimport(contextMenu.schemaName, contextMenu.tableName)}>
                    Reload from CSV
                  </button>
                </>
              )}
              <div className="border-t my-1" />
              <button className="w-full text-left text-xs px-3 py-1.5 hover:bg-accent text-destructive" onClick={() => confirmDropTable(contextMenu.schemaName, contextMenu.tableName)}>
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
              <Button size="sm" variant="outline" className="text-xs h-7" onClick={() => setShowCreateSchema(false)}>キャンセル</Button>
              <Button size="sm" className="text-xs h-7" disabled={!newSchemaName.trim()} onClick={executeCreateSchema}>作成</Button>
            </div>
          </div>
        </div>
      )}

      {dropConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="bg-background border rounded-lg shadow-xl w-80 p-4">
            <p className="text-sm font-semibold mb-2">{dropConfirm.type === "table" ? "テーブルを削除" : "スキーマを削除"}</p>
            <p className="text-xs text-muted-foreground mb-4">
              {dropConfirm.type === "table"
                ? `"${dropConfirm.schemaName}"."${dropConfirm.tableName}" を削除します。この操作は元に戻せません。`
                : `スキーマ "${dropConfirm.schemaName}" とその全テーブルを削除します（CASCADE）。この操作は元に戻せません。`}
            </p>
            <div className="flex justify-end gap-2">
              <Button size="sm" variant="outline" className="text-xs h-7" onClick={() => setDropConfirm(null)}>キャンセル</Button>
              <Button size="sm" variant="destructive" className="text-xs h-7" onClick={() => dropConfirm.type === "table" ? executeDropTable(dropConfirm.schemaName, dropConfirm.tableName!) : executeDropSchema(dropConfirm.schemaName)}>削除</Button>
            </div>
          </div>
        </div>
      )}

      {showImport && (
        <FileImportDialog
          initialFilePath={pendingFile ?? undefined}
          schemas={allSchemas}
          onImported={handleImported}
          onClose={() => { setShowImport(false); setPendingFile(null); }}
        />
      )}

      {metaTable && (
        <TableMetaModal schemaName={metaTable.schemaName} tableName={metaTable.tableName} isDbt={false} onClose={() => setMetaTable(null)} />
      )}
    </>
  );
}
