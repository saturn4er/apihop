<script setup lang="ts">
import { computed } from "vue";
import type { HistoryEntry } from "@/api/client";
import { METHOD_COLORS } from "@/utils/http";

const props = defineProps<{
  entry: HistoryEntry;
  active?: boolean;
}>();

const emit = defineEmits<{
  click: [];
}>();

const methodColors = METHOD_COLORS;

const truncatedUrl = computed(() => {
  try {
    const u = new URL(props.entry.url);
    const path = u.pathname + u.search;
    return path.length > 30 ? path.slice(0, 27) + "..." : path;
  } catch {
    const url = props.entry.url;
    return url.length > 30 ? url.slice(0, 27) + "..." : url;
  }
});

const statusClass = computed(() => {
  const s = props.entry.response_status;
  if (s >= 200 && s < 300) return "status-2xx";
  if (s >= 300 && s < 400) return "status-3xx";
  if (s >= 400 && s < 500) return "status-4xx";
  return "status-5xx";
});

const formattedTime = computed(() => {
  const d = new Date(props.entry.timestamp);
  const now = new Date();
  const isToday =
    d.getFullYear() === now.getFullYear() &&
    d.getMonth() === now.getMonth() &&
    d.getDate() === now.getDate();
  if (isToday) {
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }
  return d.toLocaleDateString([], { month: "short", day: "numeric" });
});
</script>

<template>
  <div class="history-row" :class="{ active }" @click="emit('click')">
    <span
      class="method-badge"
      :style="{ color: methodColors[entry.method] || 'var(--text-secondary)' }"
    >
      {{ entry.method.slice(0, 3) }}
    </span>
    <span class="url" :title="entry.url">{{ truncatedUrl }}</span>
    <span class="status" :class="statusClass">{{ entry.response_status }}</span>
    <span class="time">{{ formattedTime }}</span>
  </div>
</template>

<style scoped>
.history-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  cursor: pointer;
  border-radius: var(--radius-sm);
  min-height: 28px;
  transition: background var(--transition);
}

.history-row:hover {
  background: var(--bg-hover);
}

.history-row.active {
  background: var(--accent-muted);
}

.method-badge {
  font-size: 10px;
  font-weight: 700;
  flex-shrink: 0;
  width: 28px;
  text-align: center;
  background: var(--bg-tertiary);
  border-radius: 10px;
  padding: 1px 0;
}

.url {
  flex: 1;
  font-size: 12px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status {
  font-size: 11px;
  font-weight: 700;
  flex-shrink: 0;
  padding: 1px 6px;
  border-radius: 10px;
}

.status-2xx { color: var(--success); background: rgba(76, 175, 80, 0.12); }
.status-3xx { color: var(--info); background: rgba(33, 150, 243, 0.12); }
.status-4xx { color: var(--warning); background: rgba(255, 152, 0, 0.12); }
.status-5xx { color: var(--error); background: rgba(244, 67, 54, 0.12); }

.time {
  font-size: 11px;
  color: var(--text-muted);
  flex-shrink: 0;
  min-width: 45px;
  text-align: right;
}
</style>
