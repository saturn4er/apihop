<script setup lang="ts">
import { ref, watch } from "vue";
import { useWebSocket } from "@/composables/useWebSocket";
import WsBar from "./WsBar.vue";
import WsComposer from "./WsComposer.vue";
import WsMessageLog from "./WsMessageLog.vue";
import type { KeyValueRow } from "@/api/types";
import KeyValueTable from "./KeyValueTable.vue";
import type { AuthConfig } from "@/api/client";

const props = defineProps<{
  environmentId?: string;
  savedHeaders?: Record<string, string>;
  savedUrl?: string;
  auth?: AuthConfig;
  collectionAuth?: AuthConfig;
}>();

const {
  status,
  messages,
  filterText,
  connectError,
  filteredMessages,
  connect,
  send,
  disconnect,
  clearLog,
} = useWebSocket();

const wsUrl = ref(props.savedUrl || "wss://");
const activeTab = ref<"headers" | "messages">("messages");
const headerRows = ref<KeyValueRow[]>([{ key: "", value: "", enabled: true }]);

// Initialize headers from saved data
watch(
  () => props.savedHeaders,
  (h) => {
    if (h && Object.keys(h).length > 0) {
      headerRows.value = [
        ...Object.entries(h).map(([key, value]) => ({ key, value, enabled: true })),
        { key: "", value: "", enabled: true },
      ];
    }
  },
  { immediate: true }
);

watch(
  () => props.savedUrl,
  (u) => {
    if (u) wsUrl.value = u;
  }
);

function onConnect() {
  const headers: Record<string, string> = {};
  for (const row of headerRows.value) {
    if (row.enabled && row.key.trim()) {
      headers[row.key] = row.value;
    }
  }
  // Determine effective auth: request auth > collection auth > none
  const effectiveAuth = (props.auth && props.auth.type !== "none" && props.auth.type !== "inherit")
    ? props.auth
    : (props.auth?.type === "inherit" && props.collectionAuth)
      ? props.collectionAuth
      : undefined;
  connect(wsUrl.value, headers, effectiveAuth, props.environmentId);
}

function onSend(payload: string, isBinary: boolean) {
  send(payload, isBinary);
}

function getFormData() {
  const headers: Record<string, string> = {};
  for (const row of headerRows.value) {
    if (row.key.trim()) {
      headers[row.key] = row.value;
    }
  }
  return { url: wsUrl.value, headers };
}

defineExpose({ getFormData, disconnect });
</script>

<template>
  <div class="ws-panel">
    <WsBar
      v-model="wsUrl"
      :status="status"
      :connect-error="connectError"
      @connect="onConnect"
      @disconnect="disconnect"
    />
    <div class="ws-tabs">
      <button
        :class="{ active: activeTab === 'messages' }"
        @click="activeTab = 'messages'"
      >
        Messages
        <span v-if="messages.length" class="msg-count">{{ messages.length }}</span>
      </button>
      <button
        :class="{ active: activeTab === 'headers' }"
        @click="activeTab = 'headers'"
      >
        Headers
      </button>
      <button
        v-if="messages.length > 0"
        class="clear-btn"
        @click="clearLog"
      >
        Clear
      </button>
    </div>
    <div class="ws-content">
      <div v-if="activeTab === 'messages'" class="messages-panel">
        <WsMessageLog
          v-model="filterText"
          :messages="filteredMessages"
        />
        <WsComposer
          :disabled="status !== 'connected'"
          @send="onSend"
        />
      </div>
      <div v-else class="headers-panel">
        <KeyValueTable v-model="headerRows" key-placeholder="Header name" value-placeholder="Value" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.ws-panel {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.ws-tabs {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-secondary);
  padding: 0 12px;
}

.ws-tabs button {
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  color: var(--text-secondary);
  font-size: 12px;
  padding: 8px 12px;
  cursor: pointer;
  transition: all var(--transition);
  display: flex;
  align-items: center;
  gap: 6px;
}

.ws-tabs button:hover {
  color: var(--text-primary);
}

.ws-tabs button.active {
  color: var(--text-primary);
  border-bottom-color: var(--accent);
}

.ws-tabs .clear-btn {
  margin-left: auto;
  color: var(--text-muted);
  font-size: 11px;
}

.ws-tabs .clear-btn:hover {
  color: var(--error);
}

.msg-count {
  background: var(--accent-muted);
  color: var(--accent);
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 10px;
  font-weight: 600;
}

.ws-content {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.messages-panel {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.headers-panel {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
}
</style>
