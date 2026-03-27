import { Button } from "@/components/ui/button";

interface Props {
  title: string;
  description: string;
  confirmLabel?: string;
  cancelLabel?: string;
  destructive?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({
  title,
  description,
  confirmLabel = "OK",
  cancelLabel = "キャンセル",
  destructive = false,
  onConfirm,
  onCancel,
}: Props) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-background border rounded-lg shadow-xl w-80 p-4">
        <p className="text-sm font-semibold mb-2">{title}</p>
        <p className="text-xs text-muted-foreground mb-4">{description}</p>
        <div className="flex justify-end gap-2">
          <Button size="sm" variant="outline" className="text-xs h-7" onClick={onCancel}>
            {cancelLabel}
          </Button>
          <Button
            size="sm"
            variant={destructive ? "destructive" : "default"}
            className="text-xs h-7"
            onClick={onConfirm}
          >
            {confirmLabel}
          </Button>
        </div>
      </div>
    </div>
  );
}
