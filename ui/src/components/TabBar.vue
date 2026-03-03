<script setup lang="ts">
import { METHOD_COLORS } from "@/utils/http";
import type { Tab } from "@/api/types";

defineProps<{
  tabs: Tab[];
  activeTabIndex: number;
}>();

const emit = defineEmits<{
  select: [index: number];
  close: [index: number];
  "new-tab": [];
}>();

function methodColor(method: string): string {
  return METHOD_COLORS[method.toUpperCase()] || "var(--text-secondary)";
}

function onMousedown(e: MouseEvent, index: number) {
  if (e.button === 1) {
    e.preventDefault();
    emit("close", index);
  }
}
</script>

<template>
  <div class="tab-bar">
    <div class="tab-list" role="tablist" aria-label="Request tabs">
      <div
        v-for="(tab, index) in tabs"
        :key="tab.id"
        class="tab-item"
        :class="{ active: index === activeTabIndex }"
        role="tab"
        :aria-selected="index === activeTabIndex"
        :aria-controls="`tabpanel-${tab.id}`"
        :tabindex="index === activeTabIndex ? 0 : -1"
        @click="emit('select', index)"
        @mousedown="onMousedown($event, index)"
      >
        <span v-if="tab.requestType === 'websocket'" class="tab-ws-badge">WS</span>
        <span v-else class="tab-method" :style="{ color: methodColor(tab.method) }">
          {{ tab.method }}
        </span>
        <span class="tab-dirty" v-if="tab.isDirty">&bull;</span>
        <span class="tab-name">{{ tab.name }}</span>
        <button
          class="tab-close"
          @click.stop="emit('close', index)"
          title="Close tab"
          aria-label="Close tab"
        >
          &times;
        </button>
      </div>
    </div>
    <button class="tab-new" @click="emit('new-tab')" title="New tab (Ctrl+N)" aria-label="New tab">
      +
    </button>
  </div>
</template>

<style scoped>
.tab-bar {
  display: flex;
  align-items: stretch;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-color);
  min-height: 36px;
  overflow-x: auto;
  overflow-y: hidden;
}

.tab-list {
  display: flex;
  align-items: stretch;
  flex: 1;
  min-width: 0;
  overflow-x: auto;
}

.tab-list::-webkit-scrollbar {
  height: 2px;
}

.tab-list::-webkit-scrollbar-thumb {
  background: var(--border-color);
}

.tab-item {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 0 12px;
  min-width: 0;
  max-width: 180px;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  color: var(--text-secondary);
  font-size: 12px;
  white-space: nowrap;
  transition: all var(--transition);
  user-select: none;
  flex-shrink: 0;
}

.tab-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.tab-item.active {
  color: var(--text-primary);
  border-bottom-color: var(--accent);
  background: var(--bg-primary);
}

.tab-method {
  font-weight: 700;
  font-size: 10px;
  text-transform: uppercase;
  flex-shrink: 0;
}

.tab-ws-badge {
  font-weight: 700;
  font-size: 9px;
  background: var(--accent);
  color: #fff;
  padding: 1px 5px;
  border-radius: 3px;
  flex-shrink: 0;
}

.tab-dirty {
  color: var(--warning);
  font-size: 14px;
  line-height: 1;
  flex-shrink: 0;
}

.tab-name {
  overflow: hidden;
  text-overflow: ellipsis;
}

.tab-close {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 14px;
  cursor: pointer;
  padding: 0 2px;
  line-height: 1;
  border-radius: 3px;
  flex-shrink: 0;
  margin-left: auto;
}

.tab-close:hover {
  color: var(--text-primary);
  background: var(--bg-tertiary);
}

.tab-new {
  background: none;
  border: none;
  border-left: 1px solid var(--border-color);
  color: var(--text-secondary);
  font-size: 16px;
  cursor: pointer;
  padding: 0 12px;
  flex-shrink: 0;
  transition: all var(--transition);
}

.tab-new:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}
</style>
