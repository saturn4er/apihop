<script setup lang="ts">
import { ref } from "vue";

defineProps<{
  disabled: boolean;
}>();

const emit = defineEmits<{
  send: [payload: string, isBinary: boolean];
}>();

const message = ref("");
const format = ref<"text" | "json">("text");

function onSend() {
  if (!message.value.trim()) return;
  emit("send", message.value, false);
  message.value = "";
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
    e.preventDefault();
    onSend();
  }
}
</script>

<template>
  <div class="ws-composer">
    <div class="composer-header">
      <div class="format-toggle">
        <button
          :class="{ active: format === 'text' }"
          @click="format = 'text'"
        >Text</button>
        <button
          :class="{ active: format === 'json' }"
          @click="format = 'json'"
        >JSON</button>
      </div>
      <button
        class="send-btn"
        :disabled="disabled || !message.trim()"
        @click="onSend"
        title="Send (Ctrl+Enter)"
      >
        Send
      </button>
    </div>
    <textarea
      class="message-input"
      v-model="message"
      :disabled="disabled"
      placeholder="Type a message..."
      rows="3"
      @keydown="onKeydown"
    />
  </div>
</template>

<style scoped>
.ws-composer {
  border-top: 1px solid var(--border-color);
  padding: 8px 12px;
}

.composer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}

.format-toggle {
  display: flex;
  gap: 2px;
  background: var(--bg-tertiary);
  border-radius: var(--radius-sm);
  padding: 2px;
}

.format-toggle button {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 11px;
  padding: 3px 10px;
  cursor: pointer;
  border-radius: 4px;
  transition: all var(--transition);
}

.format-toggle button.active {
  background: var(--bg-input);
  color: var(--text-primary);
}

.send-btn {
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: var(--radius-sm);
  padding: 5px 14px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition);
}

.send-btn:hover:not(:disabled) {
  background: var(--accent-hover);
}

.send-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.message-input {
  width: 100%;
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 13px;
  font-family: monospace;
  padding: 8px;
  resize: vertical;
  outline: none;
  transition: border-color var(--transition);
}

.message-input:focus {
  border-color: var(--accent);
}

.message-input:disabled {
  opacity: 0.5;
}
</style>
