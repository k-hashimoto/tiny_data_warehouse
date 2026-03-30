import { ChevronRightIcon, ChevronDownIcon, TableIcon, EyeIcon, InfoIcon } from "lucide-react";
import { TableInfo } from "@/store/appStore";
import { ColumnInfo } from "./hooks/useTableTree";

interface Props {
  table: TableInfo;
  isExpanded: boolean;
  columns: ColumnInfo[] | undefined;
  onToggleSchema: () => void;
  onDoubleClick: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
  onInfoClick: (e: React.MouseEvent) => void;
}

export function TableItem({ table, isExpanded, columns, onToggleSchema, onDoubleClick, onContextMenu, onInfoClick }: Props) {
  return (
    <div className="ml-3">
      <div
        className="flex items-center gap-0.5 rounded hover:bg-accent group"
        onContextMenu={onContextMenu}
      >
        <button
          className="flex items-center justify-center w-4 h-6 shrink-0 text-muted-foreground hover:text-foreground"
          onClick={onToggleSchema}
          title="スキーマを表示"
        >
          {isExpanded
            ? <ChevronDownIcon className="h-3 w-3" />
            : <ChevronRightIcon className="h-3 w-3" />}
        </button>
        <button
          onDoubleClick={onDoubleClick}
          className="flex-1 flex items-center gap-1 py-1 pr-1 text-left text-sm min-w-0"
        >
          {table.table_type === "view"
            ? <EyeIcon className="h-3 w-3 shrink-0 text-purple-400" />
            : <TableIcon className="h-3 w-3 shrink-0 text-yellow-500" />}
          <span className="truncate font-medium">{table.name}</span>
          <span className="ml-auto text-xs text-muted-foreground shrink-0 pl-1">{table.row_count}</span>
        </button>
        <button
          className="p-0.5 rounded hover:bg-accent shrink-0 text-muted-foreground hover:text-foreground"
          title="メタデータを表示"
          onClick={onInfoClick}
        >
          <InfoIcon className="h-3 w-3" />
        </button>
      </div>

      {isExpanded && (
        <div className="ml-4 mb-1 border-l pl-2">
          {columns
            ? columns.map((col) => (
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
}
