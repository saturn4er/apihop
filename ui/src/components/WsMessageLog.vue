<script setup lang="ts">
import { ref, nextTick, watch } from "vue";
import type { WsMessage } from "@/api/client";

const props = defineProps<{
  messages: WsMessage[];
  modelValue: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const expandedIndex = ref<number | null>(null);
const logContainer = ref<HTMLDivElement | null>(null);

function toggleExpand(index: number) {
  expandedIndex.value = expandedIndex.value === index ? null : index;
}

function formatTimestamp(tsMs: number): string {
  const d = new Date(tsMs);
  if (isNaN(d.getTime())) return "—";
  return d.toTimeString().slice(0, 8) + "." + String(d.getMilliseconds()).padStart(3, "0");
}

function formatPayload(payload: string): string {
  try {
    return JSON.stringify(JSON.parse(payload), null, 2);
  } catch {
    return payload;
  }
}

function previewPayload(payload: string): string {
  const single = payload.replace(/\n/g, " ").slice(0, 120);
  return single.length < payload.length ? single + "..." : single;
}

async function copyPayload(payload: string) {
  try {
    await navigator.clipboard.writeText(payload);
  } catch {
    // ignore
  }
}

// Auto-scroll to bottom on new messages
watch(
  () => props.messages.length,
  () => {
    nextTick(() => {
      if (logContainer.value) {
        logContainer.value.scrollTop = logContainer.value.scrollHeight;
      }
    });
  }
);
</script>

<template>
  <div class="ws-message-log">
    <div class="filter-row">
      <input
        class="filter-input"
        type="text"
        :value="modelValue"
        @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
        placeholder="Filter messages..."
      />
    </div>
    <div ref="logContainer" class="log-list">
      <div v-if="messages.length === 0" class="empty-state">
        No messages yet
      </div>
      <div
        v-for="(msg, i) in messages"
        :key="msg.id"
        class="log-entry"
        :class="{ expanded: expandedIndex === i }"
        @click="toggleExpand(i)"
      >
        <div class="entry-header">
          <span
            class="direction-arrow"
            :class="msg.direction === 'sent' ? 'sent' : msg.direction === 'received' ? 'received' : 'close'"
          >
            {{ msg.direction === "sent" ? "\u2192" : msg.direction === "received" ? "\u2190" : "\u00D7" }}
          </span>
          <span class="timestamp">{{ formatTimestamp(msg.timestamp_ms) }}</span>
          <span class="payload-preview">{{ previewPayload(msg.payload) }}</span>
          <button class="copy-btn" @click.stop="copyPayload(msg.payload)" title="Copy">
            &#128203;
          </button>
        </div>
        <pre v-if="expandedIndex === i" class="payload-full">{{ formatPayload(msg.payload) }}</pre>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ws-message-log {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.filter-row {
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-subtle);
}

.filter-input {
  width: 100%;
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 12px;
  padding: 6px 10px;
  outline: none;
}

.filter-input:focus {
  border-color: var(--accent);
}

.log-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.empty-state {
  color: var(--text-muted);
  font-size: 13px;
  text-align: center;
  padding: 32px 16px;
}

.log-entry {
  padding: 6px 12px;
  cursor: pointer;
  border-bottom: 1px solid var(--border-subtle);
  transition: background var(--transition);
}

.log-entry:hover {
  background: var(--bg-hover);
}

.entry-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}

.direction-arrow {
  font-weight: 700;
  font-size: 14px;
  flex-shrink: 0;
  width: 16px;
  text-align: center;
}

.direction-arrow.sent {
  color: var(--info);
}

.direction-arrow.received {
  color: var(--success);
}

.direction-arrow.close {
  color: var(--error);
}

.timestamp {
  color: var(--text-muted);
  font-size: 11px;
  font-family: monospace;
  flex-shrink: 0;
}

.payload-preview {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-secondary);
  font-family: monospace;
  font-size: 12px;
}

.copy-btn {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 12px;
  padding: 2px;
  opacity: 0.5;
  flex-shrink: 0;
  transition: opacity var(--transition);
}

.copy-btn:hover {
  opacity: 1;
}

.payload-full {
  margin: 6px 0 2px 24px;
  padding: 8px;
  background: var(--bg-tertiary);
  border-radius: var(--radius-sm);
  font-size: 12px;
  font-family: monospace;
  color: var(--text-primary);
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
