import { ref } from "vue";

export interface Toast {
  id: number;
  message: string;
  type: "success" | "error" | "info" | "warning";
  duration: number;
}

let nextId = 0;
const toasts = ref<Toast[]>([]);

function showToast(message: string, type: Toast["type"] = "info", duration = 3000) {
  const id = nextId++;
  toasts.value.push({ id, message, type, duration });
  setTimeout(() => {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }, duration);
}

function removeToast(id: number) {
  toasts.value = toasts.value.filter((t) => t.id !== id);
}

export function useToast() {
  return { toasts, showToast, removeToast };
}
