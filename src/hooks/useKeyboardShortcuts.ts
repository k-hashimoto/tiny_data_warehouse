import { useEffect } from "react";
import { useAppStore } from "@/store/appStore";

export function useKeyboardShortcuts() {
  const addTab = useAppStore((s) => s.addTab);
  const closeTab = useAppStore((s) => s.closeTab);
  const tabs = useAppStore((s) => s.tabs);
  const activeTabId = useAppStore((s) => s.activeTabId);
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  const getActiveTab = useAppStore((s) => s.getActiveTab);
  const setSaveDialogPending = useAppStore((s) => s.setSaveDialogPending);
  const setCloseConfirmTab = useAppStore((s) => s.setCloseConfirmTab);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      const meta = e.metaKey || e.ctrlKey;
      if (!meta) return;

      // ⌘+T: 新規タブ
      if (e.key === "t") {
        e.preventDefault();
        addTab();
        return;
      }

      // ⌘+W: 現在タブを閉じる
      if (e.key === "w") {
        e.preventDefault();
        const tab = getActiveTab();
        if (tab.isDirty) {
          setCloseConfirmTab({ id: tab.id, title: tab.title });
        } else {
          closeTab(tab.id);
        }
        return;
      }

      // ⌘+S: スクリプト保存ダイアログを開く
      if (e.key === "s") {
        e.preventDefault();
        setSaveDialogPending(true);
        return;
      }

      // ⌘+[: 前のタブへ
      if (e.key === "[") {
        e.preventDefault();
        const idx = tabs.findIndex((t) => t.id === activeTabId);
        if (idx > 0) setActiveTab(tabs[idx - 1].id);
        return;
      }

      // ⌘+]: 次のタブへ
      if (e.key === "]") {
        e.preventDefault();
        const idx = tabs.findIndex((t) => t.id === activeTabId);
        if (idx < tabs.length - 1) setActiveTab(tabs[idx + 1].id);
        return;
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [addTab, closeTab, tabs, activeTabId, setActiveTab, getActiveTab, setSaveDialogPending, setCloseConfirmTab]);
}
