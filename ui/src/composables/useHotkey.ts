import { onMounted, onUnmounted } from "vue";

export interface HotkeyBinding {
  /** Key name (e.g. "s", "Enter", "n") */
  key: string;
  /** Require Ctrl/Cmd modifier */
  mod?: boolean;
  /** Handler function */
  handler: (e: KeyboardEvent) => void;
  /** If true, only fire when target is NOT an input/textarea/contentEditable */
  nonInput?: boolean;
}

export function useHotkey(bindings: HotkeyBinding[]) {
  function onKeydown(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;

    for (const binding of bindings) {
      if (binding.mod && !mod) continue;
      if (!binding.mod && mod) continue;
      if (e.key !== binding.key) continue;

      if (binding.nonInput) {
        const tag = (e.target as HTMLElement)?.tagName;
        if (tag === "INPUT" || tag === "TEXTAREA" || (e.target as HTMLElement)?.isContentEditable) {
          continue;
        }
      }

      e.preventDefault();
      binding.handler(e);
      return;
    }
  }

  onMounted(() => {
    document.addEventListener("keydown", onKeydown);
  });

  onUnmounted(() => {
    document.removeEventListener("keydown", onKeydown);
  });
}
