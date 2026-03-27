import { Button } from "@/components/ui/button";
import { XIcon } from "lucide-react";

const OSS_LICENSES = [
  // Rust
  { name: "DuckDB", version: "1.x", license: "MIT", url: "https://github.com/duckdb/duckdb" },
  { name: "Tauri", version: "2.x", license: "MIT / Apache-2.0", url: "https://github.com/tauri-apps/tauri" },
  { name: "tokio", version: "1.x", license: "MIT", url: "https://github.com/tokio-rs/tokio" },
  { name: "serde / serde_json", version: "1.x", license: "MIT / Apache-2.0", url: "https://github.com/serde-rs/serde" },
  { name: "serde_yaml", version: "0.9", license: "MIT / Apache-2.0", url: "https://github.com/dtolnay/serde-yaml" },
  { name: "notify", version: "6.x", license: "MIT / Apache-2.0", url: "https://github.com/notify-rs/notify" },
  { name: "anyhow", version: "1.x", license: "MIT / Apache-2.0", url: "https://github.com/dtolnay/anyhow" },
  // Frontend
  { name: "React", version: "19.x", license: "MIT", url: "https://github.com/facebook/react" },
  { name: "Monaco Editor (react)", version: "4.x", license: "MIT", url: "https://github.com/suren-atoyan/monaco-react" },
  { name: "TanStack Table", version: "8.x", license: "MIT", url: "https://github.com/TanStack/table" },
  { name: "Zustand", version: "5.x", license: "MIT", url: "https://github.com/pmndrs/zustand" },
  { name: "Lucide React", version: "1.x", license: "ISC", url: "https://github.com/lucide-icons/lucide" },
  { name: "react-resizable-panels", version: "4.x", license: "MIT", url: "https://github.com/bvaughn/react-resizable-panels" },
  { name: "Tailwind CSS", version: "4.x", license: "MIT", url: "https://github.com/tailwindlabs/tailwindcss" },
  { name: "clsx", version: "2.x", license: "MIT", url: "https://github.com/lukeed/clsx" },
  { name: "tailwind-merge", version: "3.x", license: "MIT", url: "https://github.com/dcastil/tailwind-merge" },
  { name: "class-variance-authority", version: "0.7.x", license: "Apache-2.0", url: "https://github.com/joe-bell/cva" },
  { name: "@base-ui/react", version: "1.x", license: "MIT", url: "https://github.com/mui/base-ui" },
  { name: "Vite", version: "7.x", license: "MIT", url: "https://github.com/vitejs/vite" },
  { name: "TypeScript", version: "5.x", license: "Apache-2.0", url: "https://github.com/microsoft/TypeScript" },
];

interface Props {
  onClose: () => void;
}

export function AboutModal({ onClose }: Props) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-background border rounded-lg shadow-xl w-[600px] max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b shrink-0">
          <div>
            <p className="text-sm font-semibold">TinyDataWarehouse</p>
            <p className="text-xs text-muted-foreground mt-0.5">v0.1.0</p>
          </div>
          <button onClick={onClose} className="opacity-50 hover:opacity-100 p-1">
            <XIcon className="h-4 w-4" />
          </button>
        </div>

        {/* OSS Licenses */}
        <div className="flex-1 overflow-y-auto px-5 py-4">
          <p className="text-xs font-semibold text-muted-foreground mb-3">OSS Licenses</p>
          <div className="space-y-0">
            {OSS_LICENSES.map((item) => (
              <div
                key={item.name}
                className="grid grid-cols-[1fr_80px] items-center py-1.5 border-b last:border-0"
              >
                <div className="min-w-0">
                  <span className="text-xs font-medium">{item.name}</span>
                  <span className="text-[10px] text-muted-foreground ml-1.5">{item.version}</span>
                </div>
                <span className="text-[10px] font-mono text-muted-foreground text-right">{item.license}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Footer */}
        <div className="flex justify-end px-5 py-3 border-t shrink-0">
          <Button size="sm" variant="outline" className="text-xs h-7" onClick={onClose}>
            閉じる
          </Button>
        </div>
      </div>
    </div>
  );
}
