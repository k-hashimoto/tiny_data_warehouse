import { ChevronRightIcon, ChevronDownIcon, FolderIcon, FolderOpenIcon } from "lucide-react";
import { TableInfo } from "@/store/appStore";
import { ColumnInfo } from "./hooks/useTableTree";
import { TableItem } from "./TableItem";

interface Props {
  schemaName: string;
  tables: TableInfo[];
  isExpanded: boolean;
  expandedTable: string | null;
  schemaCache: Record<string, ColumnInfo[]>;
  onToggle: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
  onTableContextMenu: (e: React.MouseEvent, tableName: string, csvSourcePath: string | null, tableType: string) => void;
  onToggleTable: (tableName: string) => void;
  onSelectTable: (tableName: string) => void;
  onInfoClick: (tableName: string) => void;
}

export function SchemaFolder({
  schemaName, tables, isExpanded, expandedTable, schemaCache,
  onToggle, onContextMenu, onTableContextMenu, onToggleTable, onSelectTable, onInfoClick,
}: Props) {
  return (
    <div>
      <div
        className="flex items-center gap-1 rounded hover:bg-accent px-1 py-0.5 cursor-pointer select-none"
        onClick={onToggle}
        onContextMenu={onContextMenu}
      >
        {isExpanded
          ? <ChevronDownIcon className="h-3 w-3 text-muted-foreground shrink-0" />
          : <ChevronRightIcon className="h-3 w-3 text-muted-foreground shrink-0" />}
        {isExpanded
          ? <FolderOpenIcon className="h-3 w-3 text-yellow-500 shrink-0" />
          : <FolderIcon className="h-3 w-3 text-yellow-500 shrink-0" />}
        <span className="text-xs font-semibold text-muted-foreground">{schemaName}</span>
        <span className="ml-auto text-[10px] text-muted-foreground">{tables.length}</span>
      </div>

      {isExpanded && tables.map((t) => {
        const tableKey = `${schemaName}.${t.name}`;
        return (
          <TableItem
            key={tableKey}
            table={t}
            isExpanded={expandedTable === tableKey}
            columns={schemaCache[tableKey]}
            onToggleSchema={() => onToggleTable(t.name)}
            onDoubleClick={() => onSelectTable(t.name)}
            onContextMenu={(e) => onTableContextMenu(e, t.name, t.csv_source_path, t.table_type)}
            onInfoClick={(e) => { e.stopPropagation(); onInfoClick(t.name); }}
          />
        );
      })}
    </div>
  );
}
