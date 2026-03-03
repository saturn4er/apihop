<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { listConnections, addConnection, removeConnection, testConnection, connectionLogin } from '@/api/client';
import type { ServerConnection, ServerInfo } from '@/api/types';

const emit = defineEmits<{
  "connections-changed": [];
}>();

const connections = ref<ServerConnection[]>([]);
const showAddForm = ref(false);
const newUrl = ref('');
const newName = ref('');
const addError = ref('');
const addLoading = ref(false);
const testResult = ref<ServerInfo | null>(null);

// Login form state
const loginConnId = ref<string | null>(null);
const loginEmail = ref('');
const loginPassword = ref('');
const loginError = ref('');
const loginLoading = ref(false);

async function load() {
  try {
    connections.value = await listConnections();
  } catch {
    // ignore
  }
}

async function onTestUrl() {
  addError.value = '';
  testResult.value = null;
  addLoading.value = true;
  try {
    const info = await testConnection(newUrl.value);
    testResult.value = info;
    if (!newName.value) {
      newName.value = info.name;
    }
  } catch (e: unknown) {
    addError.value = e instanceof Error ? e.message : 'Connection failed';
  } finally {
    addLoading.value = false;
  }
}

async function onAdd() {
  addError.value = '';
  addLoading.value = true;
  try {
    await addConnection(newUrl.value, newName.value || newUrl.value);
    showAddForm.value = false;
    newUrl.value = '';
    newName.value = '';
    testResult.value = null;
    await load();
    emit("connections-changed");
  } catch (e: unknown) {
    addError.value = e instanceof Error ? e.message : 'Failed to add connection';
  } finally {
    addLoading.value = false;
  }
}

async function onRemove(id: string) {
  try {
    await removeConnection(id);
    await load();
    emit("connections-changed");
  } catch {
    // ignore
  }
}

function showLogin(connId: string) {
  loginConnId.value = connId;
  loginEmail.value = '';
  loginPassword.value = '';
  loginError.value = '';
}

async function onLogin() {
  if (!loginConnId.value) return;
  loginError.value = '';
  loginLoading.value = true;
  try {
    await connectionLogin(loginConnId.value, loginEmail.value, loginPassword.value);
    loginConnId.value = null;
    await load();
    emit("connections-changed");
  } catch (e: unknown) {
    loginError.value = e instanceof Error ? e.message : 'Login failed';
  } finally {
    loginLoading.value = false;
  }
}

onMounted(load);
</script>

<template>
  <div class="connections-manager">
    <div class="connections-header">
      <h3>Server Connections</h3>
      <button class="add-btn" @click="showAddForm = !showAddForm">
        {{ showAddForm ? 'Cancel' : '+ Add' }}
      </button>
    </div>

    <div v-if="showAddForm" class="add-form">
      <div class="form-row">
        <input v-model="newUrl" placeholder="Server URL (https://...)" class="form-input" />
        <button class="test-btn" @click="onTestUrl" :disabled="!newUrl || addLoading">Test</button>
      </div>
      <div v-if="testResult" class="test-result">
        Connected to {{ testResult.name }} ({{ testResult.mode }} mode)
      </div>
      <div v-if="testResult" class="form-row">
        <input v-model="newName" placeholder="Display name" class="form-input" />
        <button class="save-btn" @click="onAdd" :disabled="addLoading">Add</button>
      </div>
      <p v-if="addError" class="error-msg">{{ addError }}</p>
    </div>

    <div v-if="connections.length === 0 && !showAddForm" class="empty-state">
      No connections configured. Add a connection to sync with an organization server.
    </div>

    <div v-for="conn in connections" :key="conn.id" class="connection-item">
      <div class="conn-info">
        <span class="conn-name">{{ conn.display_name }}</span>
        <span class="conn-url">{{ conn.server_url }}</span>
        <span class="conn-status" :class="conn.status">{{ conn.status }}</span>
      </div>
      <div class="conn-actions">
        <button v-if="conn.status === 'disconnected'" class="login-btn" @click="showLogin(conn.id)">Login</button>
        <button class="remove-btn" @click="onRemove(conn.id)">Remove</button>
      </div>
      <div v-if="loginConnId === conn.id" class="login-form">
        <input v-model="loginEmail" type="email" placeholder="Email" class="form-input" />
        <input v-model="loginPassword" type="password" placeholder="Password" class="form-input" />
        <div class="login-form-actions">
          <button class="save-btn" @click="onLogin" :disabled="loginLoading || !loginEmail || !loginPassword">
            {{ loginLoading ? 'Logging in...' : 'Login' }}
          </button>
          <button class="remove-btn" @click="loginConnId = null">Cancel</button>
        </div>
        <p v-if="loginError" class="error-msg">{{ loginError }}</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.connections-manager {
  padding: 12px 0;
}

.connections-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.connections-header h3 {
  margin: 0;
  font-size: 14px;
}

.add-btn, .test-btn, .save-btn {
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: var(--radius-sm, 4px);
  padding: 4px 12px;
  font-size: 12px;
  cursor: pointer;
}

.add-btn:hover, .test-btn:hover, .save-btn:hover {
  opacity: 0.9;
}

.add-form {
  background: var(--bg-primary);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm, 4px);
  padding: 12px;
  margin-bottom: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.form-row {
  display: flex;
  gap: 8px;
}

.form-input {
  flex: 1;
  background: var(--bg-secondary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm, 4px);
  padding: 6px 8px;
  font-size: 12px;
  color: var(--text-primary);
}

.test-result {
  font-size: 12px;
  color: var(--color-success, #27ae60);
}

.error-msg {
  font-size: 12px;
  color: var(--color-error, #e74c3c);
  margin: 0;
}

.empty-state {
  color: var(--text-secondary);
  font-size: 12px;
  text-align: center;
  padding: 24px;
}

.connection-item {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  padding: 8px;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm, 4px);
  margin-bottom: 4px;
}

.conn-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.conn-name {
  font-size: 13px;
  font-weight: 600;
}

.conn-url {
  font-size: 11px;
  color: var(--text-secondary);
}

.conn-status {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
}

.conn-status.connected {
  color: var(--color-success, #27ae60);
}

.conn-status.disconnected {
  color: var(--text-secondary);
}

.remove-btn {
  background: none;
  border: 1px solid var(--border-default);
  color: var(--text-secondary);
  border-radius: var(--radius-sm, 4px);
  padding: 4px 8px;
  font-size: 11px;
  cursor: pointer;
}

.remove-btn:hover {
  color: var(--color-error, #e74c3c);
  border-color: var(--color-error, #e74c3c);
}

.conn-actions {
  display: flex;
  gap: 4px;
}

.login-btn {
  background: none;
  border: 1px solid var(--accent);
  color: var(--accent);
  border-radius: var(--radius-sm, 4px);
  padding: 4px 8px;
  font-size: 11px;
  cursor: pointer;
}

.login-btn:hover {
  background: var(--accent);
  color: #fff;
}

.login-form {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 8px;
  padding: 8px;
  background: var(--bg-primary);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm, 4px);
  width: 100%;
}

.login-form-actions {
  display: flex;
  gap: 6px;
}
</style>
