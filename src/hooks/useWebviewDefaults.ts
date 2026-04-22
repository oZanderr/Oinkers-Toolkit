import { useEffect } from "react";

function isEditable(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  return target.isContentEditable;
}

export function useWebviewDefaults() {
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      const ctrl = e.ctrlKey || e.metaKey;
      const key = e.key;
      const lower = key.toLowerCase();

      // Reload
      if (key === "F5") return e.preventDefault();
      if (ctrl && lower === "r") return e.preventDefault();

      // Find / find-next
      if (ctrl && lower === "f") return e.preventDefault();
      if (ctrl && lower === "g") return e.preventDefault();
      if (key === "F3") return e.preventDefault();

      // Print / save page / downloads / history
      if (ctrl && lower === "p") return e.preventDefault();
      if (ctrl && lower === "s") return e.preventDefault();
      if (ctrl && lower === "j") return e.preventDefault();
      if (ctrl && lower === "h") return e.preventDefault();

      // Caret browsing
      if (key === "F7") return e.preventDefault();

      // History nav
      if (e.altKey && (key === "ArrowLeft" || key === "ArrowRight")) return e.preventDefault();
      if (key === "Backspace" && !isEditable(e.target)) return e.preventDefault();
    }

    function onContextMenu(e: MouseEvent) {
      e.preventDefault();
    }

    function onDragOver(e: DragEvent) {
      e.preventDefault();
    }

    function onDrop(e: DragEvent) {
      e.preventDefault();
    }

    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("contextmenu", onContextMenu);
    window.addEventListener("dragover", onDragOver);
    window.addEventListener("drop", onDrop);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("contextmenu", onContextMenu);
      window.removeEventListener("dragover", onDragOver);
      window.removeEventListener("drop", onDrop);
    };
  }, []);
}
