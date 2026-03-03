<script setup lang="ts">
import type { WsStatus } from "@/api/client";

const props = defineProps<{
  status: WsStatus;
  modelValue: string;
  connectError: string | null;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
  connect: [];
  disconnect: [];
}>();

const statusColors: Record<WsStatus, string> = {
  connected: "var(--success)",
  connecting: "var(--warning)",
  disconnected: "var(--text-muted)",
};
</script>

<template>
  <div class="ws-bar">
    <div class="ws-url-row">
      <span class="status-dot" :style="{ background: statusColors[status] }" :title="status" />
      <input
        class="url-input"
        type="text"
        :value="modelValue"
        @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
        placeholder="wss://example.com/ws"
        :disabled="status !== 'disconnected'"
      />
      <button
        v-if="status === 'disconnected'"
        class="connect-btn"
        @click="emit('connect')"
      >
        Connect
      </button>
      <button
        v-else
        class="disconnect-btn"
        @click="emit('disconnect')"
        :disabled="status === 'connecting'"
      >
        Disconnect
      </button>
    </div>
    <div v-if="connectError" class="connect-error">{{ connectError }}</div>
  </div>
</template>

<style scoped>
.ws-bar {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color);
}

.ws-url-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
  transition: background var(--transition);
}

.url-input {
  flex: 1;
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 13px;
  padding: 8px 12px;
  outline: none;
  font-family: monospace;
  transition: border-color var(--transition);
}

.url-input:focus {
  border-color: var(--accent);
}

.url-input:disabled {
  opacity: 0.6;
}

.connect-btn,
.disconnect-btn {
  padding: 8px 16px;
  border: none;
  border-radius: var(--radius-sm);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  flex-shrink: 0;
  transition: all var(--transition);
}

.connect-btn {
  background: var(--accent);
  color: #fff;
}

.connect-btn:hover {
  background: var(--accent-hover);
}

.disconnect-btn {
  background: var(--error);
  color: #fff;
}

.disconnect-btn:hover {
  opacity: 0.9;
}

.disconnect-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.connect-error {
  color: var(--error);
  font-size: 12px;
  margin-top: 6px;
  padding-left: 18px;
}
</style>
