<script setup lang="ts">
import { ref, onMounted } from "vue";
import type { Workspace, WorkspaceMember, WorkspaceInvite } from "@/api/types";
import {
  getWorkspace,
  updateWorkspace,
  deleteWorkspace,
  listWorkspaceMembers,
  updateWorkspaceMemberRole,
  removeWorkspaceMember,
  listWorkspaceInvites,
  createWorkspaceInvite,
  revokeWorkspaceInvite,
  connectionProxy,
} from "@/api/client";

const props = defineProps<{
  workspaceId: string;
  connectionId?: string | null;
}>();

const emit = defineEmits<{
  close: [];
  updated: [workspace: Workspace];
  deleted: [id: string];
}>();

const activeTab = ref<"general" | "members" | "invites">("general");
const loadingState = ref(true);
const error = ref("");

// General tab
const workspace = ref<Workspace | null>(null);
const editName = ref("");
const editDescription = ref("");
const saving = ref(false);

// Members tab
const members = ref<WorkspaceMember[]>([]);
const membersLoading = ref(false);

// Invites tab
const invites = ref<WorkspaceInvite[]>([]);
const invitesLoading = ref(false);
const inviteEmail = ref("");
const inviteRole = ref<string>("editor");
const inviteCreating = ref(false);

onMounted(async () => {
  try {
    if (props.connectionId) {
      workspace.value = await connectionProxy(props.connectionId, "GET", `/api/v1/workspaces/${props.workspaceId}`) as Workspace;
    } else {
      workspace.value = await getWorkspace(props.workspaceId);
    }
    editName.value = workspace.value.name;
    editDescription.value = workspace.value.description || "";
  } catch (e: any) {
    error.value = e.message || "Failed to load workspace";
  } finally {
    loadingState.value = false;
  }
});

async function saveGeneral() {
  if (!editName.value.trim()) return;
  saving.value = true;
  error.value = "";
  try {
    let updated: Workspace;
    if (props.connectionId) {
      updated = await connectionProxy(props.connectionId, "PUT", `/api/v1/workspaces/${props.workspaceId}`, {
        name: editName.value.trim(),
        description: editDescription.value.trim() || undefined,
      }) as Workspace;
    } else {
      updated = await updateWorkspace(props.workspaceId, editName.value.trim(), editDescription.value.trim() || undefined);
    }
    workspace.value = updated;
    emit("updated", updated);
  } catch (e: any) {
    error.value = e.message || "Failed to update workspace";
  } finally {
    saving.value = false;
  }
}

async function onDelete() {
  if (!confirm("Delete this workspace? This cannot be undone.")) return;
  try {
    if (props.connectionId) {
      await connectionProxy(props.connectionId, "DELETE", `/api/v1/workspaces/${props.workspaceId}`);
    } else {
      await deleteWorkspace(props.workspaceId);
    }
    emit("deleted", props.workspaceId);
  } catch (e: any) {
    error.value = e.message || "Failed to delete workspace";
  }
}

async function loadMembers() {
  membersLoading.value = true;
  try {
    if (props.connectionId) {
      members.value = await connectionProxy(props.connectionId, "GET", `/api/v1/workspaces/${props.workspaceId}/members`) as WorkspaceMember[];
    } else {
      members.value = await listWorkspaceMembers(props.workspaceId);
    }
  } catch {
    // ignore
  } finally {
    membersLoading.value = false;
  }
}

async function changeRole(userId: string, role: string) {
  try {
    if (props.connectionId) {
      await connectionProxy(props.connectionId, "PUT", `/api/v1/workspaces/${props.workspaceId}/members/${userId}`, { role });
    } else {
      await updateWorkspaceMemberRole(props.workspaceId, userId, role);
    }
    await loadMembers();
  } catch {
    // ignore
  }
}

async function removeMember(userId: string) {
  if (!confirm("Remove this member?")) return;
  try {
    if (props.connectionId) {
      await connectionProxy(props.connectionId, "DELETE", `/api/v1/workspaces/${props.workspaceId}/members/${userId}`);
    } else {
      await removeWorkspaceMember(props.workspaceId, userId);
    }
    await loadMembers();
  } catch {
    // ignore
  }
}

async function loadInvites() {
  invitesLoading.value = true;
  try {
    if (props.connectionId) {
      invites.value = await connectionProxy(props.connectionId, "GET", `/api/v1/workspaces/${props.workspaceId}/invites`) as WorkspaceInvite[];
    } else {
      invites.value = await listWorkspaceInvites(props.workspaceId);
    }
  } catch {
    // ignore
  } finally {
    invitesLoading.value = false;
  }
}

async function sendInvite() {
  if (!inviteEmail.value.trim()) return;
  inviteCreating.value = true;
  try {
    if (props.connectionId) {
      await connectionProxy(props.connectionId, "POST", `/api/v1/workspaces/${props.workspaceId}/invites`, {
        email: inviteEmail.value.trim(),
        role: inviteRole.value,
      });
    } else {
      await createWorkspaceInvite(props.workspaceId, inviteEmail.value.trim(), inviteRole.value);
    }
    inviteEmail.value = "";
    await loadInvites();
  } catch {
    // ignore
  } finally {
    inviteCreating.value = false;
  }
}

async function revokeInvite(inviteId: string) {
  try {
    if (props.connectionId) {
      await connectionProxy(props.connectionId, "DELETE", `/api/v1/workspaces/${props.workspaceId}/invites/${inviteId}`);
    } else {
      await revokeWorkspaceInvite(props.workspaceId, inviteId);
    }
    await loadInvites();
  } catch {
    // ignore
  }
}

function switchTab(tab: "general" | "members" | "invites") {
  activeTab.value = tab;
  if (tab === "members" && members.value.length === 0) loadMembers();
  if (tab === "invites" && invites.value.length === 0) loadInvites();
}
</script>

<template>
  <div class="dialog-overlay" @click.self="emit('close')">
    <div class="dialog">
      <div class="dialog-header">
        <h3>Workspace Settings</h3>
        <button class="close-btn" @click="emit('close')">&times;</button>
      </div>

      <div v-if="loadingState" class="loading">Loading...</div>
      <template v-else-if="workspace">
        <div class="tabs">
          <button :class="{ active: activeTab === 'general' }" @click="switchTab('general')">General</button>
          <button :class="{ active: activeTab === 'members' }" @click="switchTab('members')">Members</button>
          <button :class="{ active: activeTab === 'invites' }" @click="switchTab('invites')">Invites</button>
        </div>

        <!-- General Tab -->
        <div v-if="activeTab === 'general'" class="tab-content">
          <div class="field">
            <label>Name</label>
            <input v-model="editName" placeholder="Workspace name" />
          </div>
          <div class="field">
            <label>Description</label>
            <input v-model="editDescription" placeholder="Description (optional)" />
          </div>
          <div v-if="error" class="error">{{ error }}</div>
          <div class="actions">
            <button class="btn-danger" @click="onDelete">Delete Workspace</button>
            <button class="btn-primary" :disabled="!editName.trim() || saving" @click="saveGeneral">
              {{ saving ? 'Saving...' : 'Save' }}
            </button>
          </div>
        </div>

        <!-- Members Tab -->
        <div v-if="activeTab === 'members'" class="tab-content">
          <div v-if="membersLoading" class="loading">Loading members...</div>
          <div v-else-if="members.length === 0" class="empty">No members</div>
          <div v-else class="member-list">
            <div v-for="member in members" :key="member.id" class="member-row">
              <span class="member-id">{{ member.user_id }}</span>
              <select :value="member.role" @change="changeRole(member.user_id, ($event.target as HTMLSelectElement).value)">
                <option value="owner">Owner</option>
                <option value="editor">Editor</option>
                <option value="viewer">Viewer</option>
              </select>
              <button class="btn-sm btn-danger" @click="removeMember(member.user_id)">Remove</button>
            </div>
          </div>
        </div>

        <!-- Invites Tab -->
        <div v-if="activeTab === 'invites'" class="tab-content">
          <div class="invite-form">
            <input v-model="inviteEmail" placeholder="Email address" class="invite-input" />
            <select v-model="inviteRole" class="invite-role">
              <option value="editor">Editor</option>
              <option value="viewer">Viewer</option>
            </select>
            <button class="btn-primary btn-sm" :disabled="!inviteEmail.trim() || inviteCreating" @click="sendInvite">
              Invite
            </button>
          </div>
          <div v-if="invitesLoading" class="loading">Loading invites...</div>
          <div v-else-if="invites.length === 0" class="empty">No pending invites</div>
          <div v-else class="invite-list">
            <div v-for="invite in invites" :key="invite.id" class="invite-row">
              <span class="invite-email">{{ invite.email }}</span>
              <span class="invite-role-badge">{{ invite.role }}</span>
              <button class="btn-sm btn-danger" @click="revokeInvite(invite.id)">Revoke</button>
            </div>
          </div>
        </div>
      </template>
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
  min-width: 500px;
  max-width: 600px;
  max-height: 80vh;
  overflow-y: auto;
}
.dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.dialog-header h3 {
  margin: 0;
  font-size: 16px;
}
.close-btn {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 20px;
  cursor: pointer;
  padding: 0 4px;
}
.close-btn:hover {
  color: var(--text-primary);
}
.tabs {
  display: flex;
  gap: 2px;
  margin-bottom: 16px;
  border-bottom: 1px solid var(--border-subtle);
  padding-bottom: 8px;
}
.tabs button {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 13px;
  padding: 6px 12px;
  cursor: pointer;
  border-radius: var(--radius-sm, 4px);
  transition: all 0.15s;
}
.tabs button.active {
  background: var(--accent);
  color: #fff;
}
.tabs button:hover:not(.active) {
  color: var(--text-primary);
  background: var(--bg-hover);
}
.tab-content {
  min-height: 200px;
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
.loading {
  color: var(--text-secondary);
  font-size: 13px;
  padding: 16px 0;
  text-align: center;
}
.empty {
  color: var(--text-muted);
  font-size: 13px;
  text-align: center;
  padding: 24px 0;
}
.actions {
  display: flex;
  gap: 8px;
  justify-content: space-between;
  margin-top: 16px;
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
.btn-danger {
  background: none;
  border: 1px solid var(--color-error, #e74c3c);
  color: var(--color-error, #e74c3c);
  padding: 6px 16px;
  border-radius: var(--radius-sm, 4px);
  cursor: pointer;
  font-size: 13px;
}
.btn-danger:hover {
  background: var(--color-error, #e74c3c);
  color: #fff;
}
.btn-sm {
  padding: 4px 10px;
  font-size: 12px;
}
.member-list, .invite-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.member-row, .invite-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px;
  background: var(--bg-tertiary);
  border-radius: var(--radius-sm, 4px);
}
.member-id, .invite-email {
  flex: 1;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.member-row select {
  background: var(--bg-input);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm, 4px);
  color: var(--text-primary);
  font-size: 12px;
  padding: 4px 8px;
}
.invite-form {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}
.invite-input {
  flex: 1;
  padding: 6px 10px;
  background: var(--bg-input);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm, 4px);
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
}
.invite-input:focus {
  border-color: var(--accent);
}
.invite-role {
  background: var(--bg-input);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm, 4px);
  color: var(--text-primary);
  font-size: 12px;
  padding: 4px 8px;
}
.invite-role-badge {
  font-size: 11px;
  color: var(--text-secondary);
  text-transform: capitalize;
}
</style>
