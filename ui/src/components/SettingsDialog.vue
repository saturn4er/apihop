<script setup lang="ts">
import BaseDialog from "./BaseDialog.vue";
import ConnectionsManager from "./ConnectionsManager.vue";
import { useSettings } from "@/composables/useSettings";
import { useAppStore } from "@/stores/appStore";
import { ref } from "vue";

const emit = defineEmits<{
  close: [];
  "connections-changed": [];
}>();

const appStore = useAppStore();
const { settings, defaults } = useSettings();

const activeTab = ref<'general' | 'connections'>('general');
const showConnectionsTab = !appStore.isWebOrganization;

function resetDefaults() {
  Object.assign(settings, { ...defaults });
}
</script>

<template>
  <BaseDialog title="Settings" width="480px" show-footer @close="emit('close')">
    <div v-if="showConnectionsTab" class="settings-tabs">
      <button :class="{ active: activeTab === 'general' }" @click="activeTab = 'general'">General</button>
      <button :class="{ active: activeTab === 'connections' }" @click="activeTab = 'connections'">Connections</button>
    </div>

    <ConnectionsManager v-if="activeTab === 'connections'" @connections-changed="emit('connections-changed')" />

    <section v-if="activeTab === 'general'" class="settings-section">
      <h4 class="section-title">General</h4>
      <div class="setting-row">
        <label class="setting-label">Request timeout (ms)</label>
        <input
          v-model.number="settings.requestTimeout"
          type="number"
          class="setting-input"
          min="0"
          step="1000"
        />
      </div>
      <div class="setting-row">
        <label class="setting-label">Follow redirects</label>
        <input
          v-model="settings.followRedirects"
          type="checkbox"
          class="setting-checkbox"
        />
      </div>
      <div class="setting-row">
        <label class="setting-label">Max redirects</label>
        <input
          v-model.number="settings.maxRedirects"
          type="number"
          class="setting-input"
          min="0"
          max="50"
        />
      </div>
    </section>

    <section v-if="activeTab === 'general'" class="settings-section">
      <h4 class="section-title">History</h4>
      <div class="setting-row">
        <label class="setting-label">Retention period (days)</label>
        <input
          v-model.number="settings.historyRetentionDays"
          type="number"
          class="setting-input"
          min="1"
        />
      </div>
      <div class="setting-row">
        <label class="setting-label">Max entries</label>
        <input
          v-model.number="settings.maxHistoryEntries"
          type="number"
          class="setting-input"
          min="10"
        />
      </div>
    </section>

    <section v-if="activeTab === 'general'" class="settings-section">
      <h4 class="section-title">Appearance</h4>
      <div class="setting-row">
        <label class="setting-label">Theme</label>
        <select v-model="settings.theme" class="setting-input">
          <option value="dark">Dark</option>
          <option value="light">Light</option>
          <option value="system">System</option>
        </select>
      </div>
      <div class="setting-row">
        <label class="setting-label">Font size (px)</label>
        <input
          v-model.number="settings.fontSize"
          type="number"
          class="setting-input"
          min="10"
          max="24"
        />
      </div>
    </section>

    <template #footer>
      <button class="reset-btn" @click="resetDefaults">Reset to defaults</button>
    </template>
  </BaseDialog>
</template>

<style scoped>
.settings-tabs {
  display: flex;
  gap: 2px;
  margin-bottom: 16px;
  border-bottom: 1px solid var(--border-subtle);
  padding-bottom: 8px;
}

.settings-tabs button {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  padding: 4px 12px;
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: all var(--transition);
}

.settings-tabs button.active {
  background: var(--accent);
  color: #fff;
}

.settings-tabs button:hover:not(.active) {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.settings-section {
  margin-bottom: 20px;
}

.settings-section:last-child {
  margin-bottom: 0;
}

.section-title {
  margin: 0 0 10px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
}

.setting-label {
  font-size: 13px;
  color: var(--text-primary);
}

.setting-input {
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  padding: 5px 10px;
  font-size: 13px;
  width: 140px;
}

.setting-input:focus {
  outline: none;
  border-color: var(--accent);
}

select.setting-input {
  cursor: pointer;
}

.setting-checkbox {
  accent-color: var(--accent);
  width: 16px;
  height: 16px;
  cursor: pointer;
}

.reset-btn {
  background: none;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  padding: 6px 14px;
  font-size: 12px;
  cursor: pointer;
  transition: all var(--transition);
}

.reset-btn:hover {
  color: var(--text-primary);
  border-color: var(--accent);
  background: var(--bg-hover);
}
</style>
