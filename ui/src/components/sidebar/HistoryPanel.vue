<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import HistoryEntryRow from "./HistoryEntryRow.vue";
import ConfirmDialog from "../ConfirmDialog.vue";
import {
  listHistory,
  clearHistory,
  type HistoryEntry,
} from "@/api/client";

const emit = defineEmits<{
  "load-history-entry": [entry: HistoryEntry];
}>();

const entries = ref<HistoryEntry[]>([]);
const searchQuery = ref("");
const methodFilter = ref("ALL");
const statusFilter = ref("ALL");
const activeEntryId = ref<string | null>(null);
const loading = ref(false);
const showClearConfirm = ref(false);

const methods = ["ALL", "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
const statuses = ["ALL", "2xx", "3xx", "4xx", "5xx"];

onMounted(async () => {
  await refresh();
});

async function refresh() {
  loading.value = true;
  try {
    entries.value = await listHistory(200);
  } finally {
    loading.value = false;
  }
}

const filteredEntries = computed(() => {
  return entries.value.filter((e) => {
    if (searchQuery.value && !e.url.toLowerCase().includes(searchQuery.value.toLowerCase())) {
      return false;
    }
    if (methodFilter.value !== "ALL" && e.method !== methodFilter.value) {
      return false;
    }
    if (statusFilter.value !== "ALL") {
      const prefix = statusFilter.value[0];
      if (String(e.response_status)[0] !== prefix) return false;
    }
    return true;
  });
});

const groupedEntries = computed(() => {
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const yesterday = new Date(today.getTime() - 86400000);
  const weekAgo = new Date(today.getTime() - 7 * 86400000);

  const groups: { label: string; entries: HistoryEntry[] }[] = [
    { label: "Today", entries: [] },
    { label: "Yesterday", entries: [] },
    { label: "Last 7 Days", entries: [] },
    { label: "Older", entries: [] },
  ];

  for (const entry of filteredEntries.value) {
    const d = new Date(entry.timestamp);
    if (d >= today) {
      groups[0].entries.push(entry);
    } else if (d >= yesterday) {
      groups[1].entries.push(entry);
    } else if (d >= weekAgo) {
      groups[2].entries.push(entry);
    } else {
      groups[3].entries.push(entry);
    }
  }

  return groups.filter((g) => g.entries.length > 0);
});

function onEntryClick(entry: HistoryEntry) {
  activeEntryId.value = entry.id;
  emit("load-history-entry", entry);
}

function onClearAll() {
  showClearConfirm.value = true;
}

async function confirmClearAll() {
  showClearConfirm.value = false;
  await clearHistory();
  entries.value = [];
}

defineExpose({ refresh });
</script>

<template>
  <div class="history-panel">
    <div class="filters">
      <input
        v-model="searchQuery"
        class="search-input"
        placeholder="Filter by URL..."
      />
      <div class="filter-row">
        <select v-model="methodFilter" class="filter-select">
          <option v-for="m in methods" :key="m" :value="m">{{ m }}</option>
        </select>
        <select v-model="statusFilter" class="filter-select">
          <option v-for="s in statuses" :key="s" :value="s">{{ s }}</option>
        </select>
        <button class="clear-btn" @click="onClearAll" title="Clear all history">
          Clear
        </button>
      </div>
    </div>

    <div class="entries-list">
      <div v-if="loading" class="loading-state">
        <span class="spinner"></span> Loading history...
      </div>
      <div v-else-if="groupedEntries.length === 0" class="empty-state">
        No history entries
      </div>
      <template v-for="group in groupedEntries" :key="group.label">
        <div class="group-label">{{ group.label }}</div>
        <HistoryEntryRow
          v-for="entry in group.entries"
          :key="entry.id"
          :entry="entry"
          :active="entry.id === activeEntryId"
          @click="onEntryClick(entry)"
        />
      </template>
    </div>

    <ConfirmDialog
      v-if="showClearConfirm"
      title="Clear History"
      message="Clear all history? This action cannot be undone."
      confirm-label="Clear All"
      :danger="true"
      @confirm="confirmClearAll"
      @cancel="showClearConfirm = false"
    />
  </div>
</template>

<style scoped>
.history-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.filters {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 8px;
}

.search-input {
  width: 100%;
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-size: 12px;
  padding: 6px 10px;
  outline: none;
  transition: border-color var(--transition);
}

.search-input:focus {
  border-color: var(--accent);
}

.search-input::placeholder {
  color: var(--text-muted);
}

.filter-row {
  display: flex;
  gap: 4px;
}

.filter-select {
  flex: 1;
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 11px;
  padding: 3px 4px;
  outline: none;
}

.filter-select option {
  background: var(--bg-secondary);
}

.clear-btn {
  background: none;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 11px;
  padding: 3px 8px;
  cursor: pointer;
  transition: all var(--transition);
}

.clear-btn:hover {
  color: var(--error);
  border-color: var(--error);
}

.entries-list {
  flex: 1;
  overflow-y: auto;
}

.loading-state {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: 12px;
  padding: 12px 8px;
}

.spinner {
  display: inline-block;
  width: 12px;
  height: 12px;
  border: 2px solid var(--border-color);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.group-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
  padding: 8px 8px 4px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.empty-state {
  color: var(--text-muted);
  font-size: 13px;
  text-align: center;
  margin-top: 24px;
}
</style>
