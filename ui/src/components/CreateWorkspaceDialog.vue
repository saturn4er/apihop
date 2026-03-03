<script setup lang="ts">
import { ref } from "vue";
import { createWorkspace, connectionProxy } from "@/api/client";
import type { Workspace } from "@/api/types";

const props = defineProps<{
  connectionId?: string | null;
}>();

const emit = defineEmits<{
  created: [workspace: Workspace];
  close: [];
}>();

const name = ref("");
const description = ref("");
const loading = ref(false);
const error = ref("");

async function submit() {
  if (!name.value.trim()) return;
  loading.value = true;
  error.value = "";
  try {
    let ws: Workspace;
    if (props.connectionId) {
      // Create on remote server via proxy
      ws = (await connectionProxy(props.connectionId, "POST", "/api/v1/workspaces", {
        name: name.value.trim(),
        description: description.value.trim() || undefined,
      })) as Workspace;
    } else {
      ws = await createWorkspace(name.value.trim(), description.value.trim() || undefined);
    }
    emit("created", ws);
  } catch (e: any) {
    error.value = e.message || "Failed to create workspace";
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="dialog-overlay" @click.self="emit('close')">
    <div class="dialog">
      <h3>Create Workspace</h3>
      <div class="field">
        <label>Name</label>
        <input v-model="name" placeholder="Workspace name" @keydown.enter="submit" />
      </div>
      <div class="field">
        <label>Description (optional)</label>
        <input v-model="description" placeholder="Description" />
      </div>
      <div v-if="error" class="error">{{ error }}</div>
      <div class="actions">
        <button class="btn-secondary" @click="emit('close')">Cancel</button>
        <button class="btn-primary" :disabled="!name.trim() || loading" @click="submit">
          {{ loading ? 'Creating...' : 'Create' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}
.dialog {
  background: var(--bg-secondary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md, 8px);
  padding: 24px;
  min-width: 400px;
  max-width: 500px;
}
.dialog h3 {
  margin: 0 0 16px;
  font-size: 16px;
}
.field {
  margin-bottom: 12px;
}
.field label {
  display: block;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}
.field input {
  width: 100%;
  padding: 8px 12px;
  background: var(--bg-input);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm, 4px);
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
}
.field input:focus {
  border-color: var(--accent);
}
.error {
  color: var(--color-error, #e74c3c);
  font-size: 12px;
  margin-bottom: 12px;
}
.actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
.btn-secondary {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-default);
  color: var(--text-primary);
  padding: 6px 16px;
  border-radius: var(--radius-sm, 4px);
  cursor: pointer;
  font-size: 13px;
}
.btn-primary {
  background: var(--accent);
  border: none;
  color: #fff;
  padding: 6px 16px;
  border-radius: var(--radius-sm, 4px);
  cursor: pointer;
  font-size: 13px;
}
.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
