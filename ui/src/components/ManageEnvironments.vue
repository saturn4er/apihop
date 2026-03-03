<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import BaseDialog from "./BaseDialog.vue";
import ConfirmDialog from "./ConfirmDialog.vue";
import {
  listEnvironments,
  createEnvironment,
  updateEnvironment,
  deleteEnvironment,
  listVariables,
  setVariable,
  deleteVariable,
  type Environment,
  type Variable,
} from "@/api/client";

const emit = defineEmits<{
  close: [];
}>();

const environments = ref<Environment[]>([]);
const selectedEnvId = ref<string | null>(null);
const variables = ref<Variable[]>([]);
const globalVariables = ref<Variable[]>([]);

const showGlobals = ref(false);
const newEnvName = ref("");
const creatingEnv = ref(false);

// Editing rows
const editingVars = ref<{ key: string; value: string; is_secret: boolean; id: string }[]>([]);

// Confirm dialog for delete
const deleteConfirm = ref<{ envId: string } | null>(null);

onMounted(async () => {
  environments.value = await listEnvironments();
  globalVariables.value = await listVariables(undefined);
  if (environments.value.length > 0) {
    selectedEnvId.value = environments.value[0].id;
  }
});

watch(selectedEnvId, async (id) => {
  showGlobals.value = false;
  if (!id) {
    variables.value = [];
    editingVars.value = [];
    return;
  }
  variables.value = await listVariables(id);
  syncEditingVars();
});

watch(showGlobals, (val) => {
  if (val) {
    selectedEnvId.value = null;
    editingVars.value = globalVariables.value.map((v) => ({
      key: v.key,
      value: v.value,
      is_secret: v.is_secret,
      id: v.id,
    }));
    editingVars.value.push({ key: "", value: "", is_secret: false, id: "" });
  }
});

function syncEditingVars() {
  editingVars.value = variables.value.map((v) => ({
    key: v.key,
    value: v.value,
    is_secret: v.is_secret,
    id: v.id,
  }));
  editingVars.value.push({ key: "", value: "", is_secret: false, id: "" });
}

async function onCreateEnv() {
  const name = newEnvName.value.trim();
  if (!name) return;
  const env = await createEnvironment(name);
  environments.value.push(env);
  selectedEnvId.value = env.id;
  creatingEnv.value = false;
  newEnvName.value = "";
}

function onDeleteEnv(id: string) {
  deleteConfirm.value = { envId: id };
}

async function confirmDeleteEnv() {
  if (!deleteConfirm.value) return;
  const id = deleteConfirm.value.envId;
  deleteConfirm.value = null;
  await deleteEnvironment(id);
  environments.value = environments.value.filter((e) => e.id !== id);
  if (selectedEnvId.value === id) {
    selectedEnvId.value = environments.value.length > 0 ? environments.value[0].id : null;
  }
}

async function onRenameEnv(env: Environment, newName: string) {
  if (!newName.trim() || newName === env.name) return;
  const updated = await updateEnvironment(env.id, newName.trim());
  const idx = environments.value.findIndex((e) => e.id === env.id);
  if (idx >= 0) environments.value[idx] = updated;
}

async function saveVariable(row: { key: string; value: string; is_secret: boolean; id: string }) {
  if (!row.key.trim()) return;
  const envId = showGlobals.value ? undefined : selectedEnvId.value;
  const variable: Variable = {
    id: row.id || "",
    environment_id: envId || undefined,
    key: row.key.trim(),
    value: row.value,
    is_secret: row.is_secret,
  };
  const saved = await setVariable(variable);
  row.id = saved.id;

  // Add new empty row if this was the last one
  const last = editingVars.value[editingVars.value.length - 1];
  if (last.key.trim()) {
    editingVars.value.push({ key: "", value: "", is_secret: false, id: "" });
  }
}

async function removeVariable(row: { id: string }, index: number) {
  if (row.id) {
    await deleteVariable(row.id);
  }
  editingVars.value.splice(index, 1);
  if (editingVars.value.length === 0 || editingVars.value[editingVars.value.length - 1].key) {
    editingVars.value.push({ key: "", value: "", is_secret: false, id: "" });
  }
}
</script>

<template>
  <BaseDialog title="Manage Environments" width="700px" @close="emit('close')">
    <div class="manage-layout">
      <div class="env-list">
        <div class="env-list-header">
          <span class="section-label">Environments</span>
          <button class="add-btn" @click="creatingEnv = true">+</button>
        </div>

        <div v-if="creatingEnv" class="new-env-row">
          <input
            v-model="newEnvName"
            class="env-name-input"
            placeholder="Environment name..."
            @keydown.enter="onCreateEnv"
            @keydown.escape="creatingEnv = false"
            @blur="onCreateEnv"
          />
        </div>

        <button
          class="env-item"
          :class="{ active: showGlobals }"
          @click="showGlobals = true"
        >
          Globals
        </button>

        <button
          v-for="env in environments"
          :key="env.id"
          class="env-item"
          :class="{ active: selectedEnvId === env.id && !showGlobals }"
          @click="selectedEnvId = env.id; showGlobals = false"
        >
          <span class="env-name">{{ env.name }}</span>
          <span
            class="delete-env-btn"
            role="button"
            tabindex="0"
            @click.stop="onDeleteEnv(env.id)"
            @keydown.enter.stop="onDeleteEnv(env.id)"
            title="Delete"
          >
            &times;
          </span>
        </button>
      </div>

      <div class="vars-panel">
        <div class="vars-header">
          <span class="section-label">
            {{ showGlobals ? "Global Variables" : "Variables" }}
          </span>
        </div>

        <div v-if="!selectedEnvId && !showGlobals" class="vars-empty">
          Select an environment to edit variables
        </div>

        <div v-else class="vars-table">
          <div class="vars-table-header">
            <span class="col-key">Key</span>
            <span class="col-value">Value</span>
            <span class="col-secret">Secret</span>
            <span class="col-actions"></span>
          </div>
          <div
            v-for="(row, i) in editingVars"
            :key="i"
            class="var-row"
          >
            <input
              v-model="row.key"
              class="var-input"
              placeholder="variable_name"
              @blur="saveVariable(row)"
            />
            <input
              v-model="row.value"
              class="var-input"
              :type="row.is_secret ? 'password' : 'text'"
              placeholder="value"
              @blur="saveVariable(row)"
            />
            <input
              type="checkbox"
              v-model="row.is_secret"
              class="secret-checkbox"
              @change="saveVariable(row)"
            />
            <button
              v-if="row.id"
              class="remove-var-btn"
              @click="removeVariable(row, i)"
            >
              &times;
            </button>
            <span v-else class="remove-var-btn"></span>
          </div>
        </div>
      </div>
    </div>
  </BaseDialog>

  <ConfirmDialog
    v-if="deleteConfirm"
    title="Delete Environment"
    message="Delete this environment and all its variables?"
    confirm-label="Delete"
    :danger="true"
    @confirm="confirmDeleteEnv"
    @cancel="deleteConfirm = null"
  />
</template>

<style scoped>
.manage-layout {
  display: flex;
  margin: -16px -20px;
  min-height: 350px;
}

.env-list {
  width: 180px;
  border-right: 1px solid var(--border-subtle);
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.env-list-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border-subtle);
}

.section-label {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--text-muted);
  letter-spacing: 0.5px;
}

.add-btn {
  background: none;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 14px;
  width: 22px;
  height: 22px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--transition);
}

.add-btn:hover {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--accent-muted);
}

.new-env-row {
  padding: 4px 8px;
}

.env-name-input {
  width: 100%;
  background: var(--bg-input);
  border: 1px solid var(--accent);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 12px;
  padding: 4px 8px;
  outline: none;
}

.env-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  background: none;
  border: none;
  padding: 8px 12px;
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
  text-align: left;
  transition: background var(--transition);
  border-radius: 0;
}

.env-item:hover {
  background: var(--bg-hover);
}

.env-item.active {
  background: var(--accent-muted);
}

.env-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.delete-env-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 16px;
  cursor: pointer;
  padding: 0 2px;
  flex-shrink: 0;
  opacity: 0;
  transition: all var(--transition);
}

.env-item:hover .delete-env-btn {
  opacity: 1;
}

.delete-env-btn:hover {
  color: var(--error);
}

.vars-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.vars-header {
  padding: 10px 16px;
  border-bottom: 1px solid var(--border-subtle);
}

.vars-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
  color: var(--text-muted);
  font-size: 13px;
}

.vars-table {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
}

.vars-table-header {
  display: flex;
  gap: 8px;
  padding: 4px 0;
  font-size: 11px;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.col-key { flex: 1; }
.col-value { flex: 1; }
.col-secret { width: 50px; text-align: center; }
.col-actions { width: 24px; }

.var-row {
  display: flex;
  gap: 8px;
  align-items: center;
  padding: 3px 0;
  border-radius: var(--radius-sm);
  transition: background var(--transition);
}

.var-row:hover {
  background: var(--bg-hover);
}

.var-input {
  flex: 1;
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 12px;
  padding: 5px 8px;
  outline: none;
  font-family: "SF Mono", "Fira Code", monospace;
  transition: border-color var(--transition);
}

.var-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-muted);
}

.secret-checkbox {
  appearance: none;
  -webkit-appearance: none;
  width: 34px;
  height: 18px;
  border-radius: 9px;
  background: var(--border-color);
  cursor: pointer;
  position: relative;
  transition: background var(--transition);
}

.secret-checkbox::after {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--text-secondary);
  transition: transform var(--transition);
}

.secret-checkbox:checked {
  background: var(--accent);
}

.secret-checkbox:checked::after {
  transform: translateX(16px);
  background: #fff;
}

.remove-var-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 16px;
  cursor: pointer;
  width: 24px;
  text-align: center;
  border-radius: var(--radius-sm);
  transition: all var(--transition);
}

.remove-var-btn:hover {
  color: var(--error);
  background: rgba(244, 67, 54, 0.1);
}
</style>
