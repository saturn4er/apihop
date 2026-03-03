<script setup lang="ts">
import BaseDialog from "./BaseDialog.vue";

const emit = defineEmits<{
  close: [];
}>();

const isMac = navigator.platform.toUpperCase().includes("MAC");
const mod = isMac ? "Cmd" : "Ctrl";

const shortcuts = [
  { keys: `${mod}+Enter`, description: "Send request" },
  { keys: `${mod}+S`, description: "Save request" },
  { keys: `${mod}+N`, description: "New tab" },
  { keys: `${mod}+W`, description: "Close tab" },
  { keys: `${mod}+L`, description: "Focus URL bar" },
  { keys: `${mod}+,`, description: "Open settings" },
  { keys: "?", description: "Show this dialog" },
];
</script>

<template>
  <BaseDialog title="Keyboard Shortcuts" width="360px" @close="emit('close')">
    <div class="shortcuts-list">
      <div v-for="s in shortcuts" :key="s.keys" class="shortcut-row">
        <kbd class="shortcut-keys">{{ s.keys }}</kbd>
        <span class="shortcut-desc">{{ s.description }}</span>
      </div>
    </div>
  </BaseDialog>
</template>

<style scoped>
.shortcuts-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.shortcut-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.shortcut-keys {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-color);
  border-radius: 4px;
  padding: 3px 8px;
  font-family: monospace;
  font-size: 12px;
  color: var(--text-primary);
  white-space: nowrap;
}

.shortcut-desc {
  color: var(--text-secondary);
  font-size: 13px;
}
</style>
