import { Button } from "@/components/ui/button";

interface Props {
  title: string;
  placeholder?: string;
  value: string;
  onChange: (value: string) => void;
  onConfirm: () => void;
  onCancel: () => void;
  confirmLabel?: string;
  cancelLabel?: string;
}

export function InputModal({
  title,
  placeholder,
  value,
  onChange,
  onConfirm,
  onCancel,
  confirmLabel = "作成",
  cancelLabel = "キャンセル",
}: Props) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-background border rounded-lg shadow-xl w-72 p-4">
        <p className="text-sm font-semibold mb-3">{title}</p>
        <input
          autoFocus
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") onConfirm(); if (e.key === "Escape") onCancel(); }}
          placeholder={placeholder}
          className="w-full text-xs bg-background border rounded px-2 py-1.5 mb-3 outline-none focus:ring-1 focus:ring-primary"
        />
        <div className="flex justify-end gap-2">
          <Button size="sm" variant="outline" className="text-xs h-7" onClick={onCancel}>
            {cancelLabel}
          </Button>
          <Button size="sm" className="text-xs h-7" disabled={!value.trim()} onClick={onConfirm}>
            {confirmLabel}
          </Button>
        </div>
      </div>
    </div>
  );
}
