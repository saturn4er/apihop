<script setup lang="ts">
import { ref } from "vue";
import CollectionsTree from "./CollectionsTree.vue";
import HistoryPanel from "./HistoryPanel.vue";
import type { SavedRequest, HistoryEntry } from "@/api/client";

defineProps<{
  activeRequestId?: string;
}>();

const emit = defineEmits<{
  "load-saved-request": [request: SavedRequest];
  "load-history-entry": [entry: HistoryEntry];
  "request-deleted": [id: string];
  "edit-collection-auth": [collectionId: string];
  "open-import": [];
  "create-workspace": [connectionId?: string];
  "workspace-settings": [workspaceId: string, connectionId?: string];
}>();

const activeTab = ref<"collections" | "history">("collections");
const historyPanel = ref<InstanceType<typeof HistoryPanel> | null>(null);
const collectionsTree = ref<InstanceType<typeof CollectionsTree> | null>(null);

function switchTab(tab: "collections" | "history") {
  activeTab.value = tab;
  if (tab === "history") {
    historyPanel.value?.refresh();
  }
}

defineExpose({
  refreshHistory: () => historyPanel.value?.refresh(),
  updateCachedRequest: (req: SavedRequest) => collectionsTree.value?.updateCachedRequest(req),
  refreshCollections: () => collectionsTree.value?.refreshCollections(),
});
</script>

<template>
  <div class="sidebar-tabs">
    <div class="tab-bar" role="tablist" aria-label="Sidebar tabs">
      <button
        class="tab"
        :class="{ active: activeTab === 'collections' }"
        role="tab"
        :aria-selected="activeTab === 'collections'"
        aria-controls="sidebar-panel-collections"
        @click="switchTab('collections')"
      >
        Collections
      </button>
      <button
        class="tab"
        :class="{ active: activeTab === 'history' }"
        role="tab"
        :aria-selected="activeTab === 'history'"
        aria-controls="sidebar-panel-history"
        @click="switchTab('history')"
      >
        History
      </button>
    </div>
    <div class="tab-content">
      <CollectionsTree
        v-if="activeTab === 'collections'"
        ref="collectionsTree"
        :active-request-id="activeRequestId"
        @load-saved-request="emit('load-saved-request', $event)"
        @request-deleted="emit('request-deleted', $event)"
        @edit-collection-auth="emit('edit-collection-auth', $event)"
        @open-import="emit('open-import')"
        @create-workspace="emit('create-workspace', $event)"
        @workspace-settings="(wsId: string, connId?: string) => emit('workspace-settings', wsId, connId)"
      />
      <HistoryPanel
        v-else
        ref="historyPanel"
        @load-history-entry="emit('load-history-entry', $event)"
      />
    </div>
  </div>
</template>

<style scoped>
.sidebar-tabs {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.tab-bar {
  display: flex;
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.tab {
  flex: 1;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  padding: 10px 0;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  cursor: pointer;
  transition: all var(--transition);
}

.tab:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.tab-content {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}
</style>
