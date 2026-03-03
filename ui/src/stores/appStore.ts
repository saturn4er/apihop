import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { AppMode, ServerInfo } from '@/api/types';

const IS_TAURI = "__TAURI_INTERNALS__" in window;

export const useAppStore = defineStore('app', () => {
  const mode = ref<AppMode>({ kind: 'desktop' });
  const loading = ref(true);

  const isDesktop = computed(() => mode.value.kind === 'desktop');
  const isWebPersonal = computed(() => mode.value.kind === 'web-personal');
  const isWebOrganization = computed(() => mode.value.kind === 'web-organization');
  const needsAuth = computed(() => mode.value.kind === 'web-organization');

  async function detectMode() {
    if (IS_TAURI) {
      mode.value = { kind: 'desktop' };
      loading.value = false;
      return;
    }

    try {
      const { fetchServerInfo } = await import('@/api/client');
      const info: ServerInfo = await fetchServerInfo();
      if (info.mode === 'organization') {
        mode.value = { kind: 'web-organization', serverInfo: info };
      } else {
        mode.value = { kind: 'web-personal' };
      }
    } catch {
      // If server info fails, assume personal mode
      mode.value = { kind: 'web-personal' };
    }
    loading.value = false;
  }

  return { mode, loading, isDesktop, isWebPersonal, isWebOrganization, needsAuth, detectMode };
});
