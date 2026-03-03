import { ref } from "vue";
import { useToast } from "./useToast";

export function useApiCall() {
  const loading = ref(false);
  const error = ref<string | null>(null);
  const { showToast } = useToast();

  async function execute<T>(
    fn: () => Promise<T>,
    options?: { errorMessage?: string; showError?: boolean },
  ): Promise<T | undefined> {
    const { errorMessage, showError = true } = options ?? {};
    loading.value = true;
    error.value = null;
    try {
      const result = await fn();
      return result;
    } catch (e: any) {
      const msg = errorMessage || e?.message || String(e);
      error.value = msg;
      if (showError) {
        showToast(msg, "error");
      }
      return undefined;
    } finally {
      loading.value = false;
    }
  }

  return { loading, error, execute };
}
