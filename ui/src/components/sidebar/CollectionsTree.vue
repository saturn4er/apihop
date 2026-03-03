<script setup lang="ts">
import { ref, onMounted, nextTick } from "vue";
import TreeNode from "./TreeNode.vue";
import { triggerDownload } from "@/utils/download";
import ContextMenu from "./ContextMenu.vue";
import { useCollectionStore } from "@/stores/collectionStore";
import { useWorkspaceStore } from "@/stores/workspaceStore";
import { useAppStore } from "@/stores/appStore";
import { useApiCall } from "@/composables/useApiCall";
import { useToast } from "@/composables/useToast";
import type { MenuItem, TreeItem, Workspace } from "@/api/types";
import {
  createCollection,
  updateCollection,
  deleteCollection,
  createFolder,
  updateFolder,
  deleteFolder,
  createRequest,
  updateRequest,
  deleteRequest,
  exportCollectionApihop,
  exportCollectionPostman,
  exportRequestCurl,
  connectionProxy,
  type Collection,
  type SavedRequest,
} from "@/api/client";

const props = defineProps<{
  activeRequestId?: string;
}>();

const emit = defineEmits<{
  "load-saved-request": [request: SavedRequest];
  "request-deleted": [id: string];
  "edit-collection-auth": [collectionId: string];
  "open-import": [];
  "create-workspace": [connectionId?: string];
  "workspace-settings": [workspaceId: string, connectionId?: string];
}>();

const store = useCollectionStore();
const workspaceStore = useWorkspaceStore();
const appStore = useAppStore();
const { execute } = useApiCall();
const { showToast } = useToast();

// Sidebar group collapse tracking
const collapsedGroups = ref(new Set<string>());

function toggleGroup(groupId: string) {
  if (collapsedGroups.value.has(groupId)) {
    collapsedGroups.value.delete(groupId);
  } else {
    collapsedGroups.value.add(groupId);
  }
}

// New collection inline input
const creatingCollection = ref(false);
const newCollectionName = ref("");
const newCollectionInput = ref<HTMLInputElement | null>(null);
// Track workspace context for new collection creation
const creatingInWorkspaceId = ref<string | null>(null);
const creatingInContext = ref<import("@/api/types").DataContext>({ type: "local" });

// Context menu
const contextMenu = ref<{ x: number; y: number; items: MenuItem[]; target: TreeItem } | null>(null);

// Rename tracking
const renamingNodeId = ref<string | null>(null);
const nodeRefs = ref<Record<string, InstanceType<typeof TreeNode>>>({});

// Keyboard navigation
const treeRef = ref<HTMLElement | null>(null);
const focusedNodeId = ref<string | null>(null);

function setNodeRef(id: string) {
  return (el: any) => {
    if (el) nodeRefs.value[id] = el;
  };
}

onMounted(async () => {
  await store.refreshCollections();
  await workspaceStore.loadSidebarGroups();
});

function onNodeClick(item: TreeItem) {
  focusedNodeId.value = item.id;
  if (item.type === "request") {
    emit("load-saved-request", item.data as SavedRequest);
  } else {
    store.toggleNode(item);
  }
}

function onContextMenu(event: MouseEvent, item: TreeItem) {
  const items: MenuItem[] = [];
  if (item.type === "collection") {
    items.push({ label: "New Folder", action: "new-folder" });
    items.push({ label: "New Request", action: "new-request" });
    items.push({ label: "", action: "", separator: true });
    items.push({ label: "Export as JSON", action: "export-apihop" });
    items.push({ label: "Export as Postman", action: "export-postman" });
    items.push({ label: "", action: "", separator: true });
    items.push({ label: "Edit Auth", action: "edit-auth" });
    items.push({ label: "Rename", action: "rename" });
    items.push({ label: "Delete", action: "delete", danger: true });
  } else if (item.type === "folder") {
    items.push({ label: "New Subfolder", action: "new-folder" });
    items.push({ label: "New Request", action: "new-request" });
    items.push({ label: "", action: "", separator: true });
    items.push({ label: "Rename", action: "rename" });
    items.push({ label: "Delete", action: "delete", danger: true });
  } else {
    items.push({ label: "Copy as cURL", action: "copy-curl" });
    items.push({ label: "", action: "", separator: true });
    items.push({ label: "Duplicate", action: "duplicate" });
    items.push({ label: "Rename", action: "rename" });
    items.push({ label: "Delete", action: "delete", danger: true });
  }
  contextMenu.value = { x: event.clientX, y: event.clientY, items, target: item };
}

async function onContextAction(action: string) {
  if (!contextMenu.value) return;
  const target = contextMenu.value.target;
  contextMenu.value = null;

  if (action === "export-apihop") {
    await execute(async () => {
      const json = await exportCollectionApihop(target.id);
      try {
        await triggerDownload(json, `${target.label}.json`, "application/json");
        showToast("Collection exported successfully", "success");
      } catch (err: any) {
        if (err?.name === "AbortError") return; // user cancelled save dialog
        throw err;
      }
    });
    return;
  } else if (action === "export-postman") {
    await execute(async () => {
      const json = await exportCollectionPostman(target.id);
      try {
        await triggerDownload(json, `${target.label}_postman.json`, "application/json");
        showToast("Collection exported successfully", "success");
      } catch (err: any) {
        if (err?.name === "AbortError") return; // user cancelled save dialog
        throw err;
      }
    });
    return;
  } else if (action === "copy-curl") {
    await execute(async () => {
      const curl = await exportRequestCurl(target.id);
      await navigator.clipboard.writeText(curl);
    });
    return;
  } else if (action === "edit-auth") {
    emit("edit-collection-auth", target.type === "collection" ? target.id : target.collectionId);
    return;
  } else if (action === "new-folder") {
    const collId = target.type === "collection" ? target.id : target.collectionId;
    const parentId = target.type === "folder" ? target.id : undefined;
    await execute(async () => {
      await createFolder(collId, "New Folder", parentId);
      await store.refreshNodeChildren(target);
    });
  } else if (action === "new-request") {
    const collId = target.type === "collection" ? target.id : target.collectionId;
    const folderId = target.type === "folder" ? target.id : undefined;
    await execute(async () => {
      const req: SavedRequest = {
        id: "",
        collection_id: collId,
        folder_id: folderId,
        name: "New Request",
        method: "GET",
        url: "",
        headers: {},
        params: [],
        sort_order: 0,
        created_at: "",
        updated_at: "",
      };
      const created = await createRequest(req);
      await store.refreshNodeChildren(target);
      emit("load-saved-request", created);
    });
  } else if (action === "rename") {
    renamingNodeId.value = target.id;
    nextTick(() => {
      nodeRefs.value[target.id]?.startRename();
    });
  } else if (action === "delete") {
    await execute(async () => {
      if (target.type === "collection") {
        await deleteCollection(target.id);
        await store.refreshCollections();
      } else if (target.type === "folder") {
        await deleteFolder(target.collectionId, target.id);
        await store.refreshParentChildren(target);
      } else {
        await deleteRequest(target.id);
        emit("request-deleted", target.id);
        await store.refreshParentChildren(target);
      }
    });
  } else if (action === "workspace-settings") {
    emit("workspace-settings", target.id, contextMenuConnectionId.value);
  } else if (action === "duplicate") {
    await execute(async () => {
      const orig = target.data as SavedRequest;
      const dup: SavedRequest = {
        ...orig,
        id: "",
        name: orig.name + " (copy)",
        created_at: "",
        updated_at: "",
      };
      const created = await createRequest(dup);
      await store.refreshParentChildren(target);
      emit("load-saved-request", created);
    });
  }
}

async function onRename(item: TreeItem, newName: string) {
  renamingNodeId.value = null;
  await execute(async () => {
    if (item.type === "collection") {
      await updateCollection(item.id, newName);
      await store.refreshCollections();
    } else if (item.type === "folder") {
      await updateFolder(item.collectionId, item.id, newName);
      await store.refreshParentChildren(item);
    } else {
      const req = item.data as SavedRequest;
      await updateRequest({ ...req, name: newName });
      await store.refreshParentChildren(item);
    }
  });
}

function startNewCollection(workspaceId?: string, context?: import("@/api/types").DataContext) {
  creatingCollection.value = true;
  newCollectionName.value = "";
  creatingInWorkspaceId.value = workspaceId || null;
  creatingInContext.value = context || { type: "local" };
  nextTick(() => {
    newCollectionInput.value?.focus();
  });
}

let commitInProgress = false;
async function commitNewCollection() {
  if (commitInProgress) return;
  const name = newCollectionName.value.trim();
  const wsId = creatingInWorkspaceId.value;
  const ctx = creatingInContext.value;
  creatingCollection.value = false;
  creatingInWorkspaceId.value = null;
  if (!name) return;
  commitInProgress = true;
  try {
    await execute(async () => {
      if (ctx.type === "remote" && wsId) {
        // Create collection on remote server via proxy
        await connectionProxy(ctx.connectionId, "POST", "/api/v1/collections", {
          name,
          workspace_id: wsId,
        });
        // Refresh workspace collections
        delete workspaceStore.workspaceCollections[wsId];
        workspaceStore.expandedWorkspaces.delete(wsId);
        await workspaceStore.toggleWorkspace(
          { id: wsId } as Workspace,
          ctx
        );
      } else {
        await createCollection(name, undefined, undefined, wsId || undefined);
        await store.refreshCollections();
        if (wsId) {
          delete workspaceStore.workspaceCollections[wsId];
          workspaceStore.expandedWorkspaces.delete(wsId);
          await workspaceStore.toggleWorkspace(
            { id: wsId } as Workspace,
            ctx
          );
        }
      }
    });
  } finally {
    commitInProgress = false;
  }
}

// Keyboard navigation for tree
function getAllVisibleItems(): TreeItem[] {
  const items: TreeItem[] = [];
  for (const col of store.collections) {
    const colItem = store.collectionAsTreeItem(col);
    items.push(colItem);
    if (store.isExpanded(colItem)) {
      collectVisibleChildren(colItem, items);
    }
  }
  return items;
}

function collectVisibleChildren(parent: TreeItem, items: TreeItem[]) {
  const children = store.getChildren(parent);
  for (const child of children) {
    items.push(child);
    if (child.type !== "request" && store.isExpanded(child)) {
      collectVisibleChildren(child, items);
    }
  }
}

function onTreeKeydown(e: KeyboardEvent) {
  const visible = getAllVisibleItems();
  if (visible.length === 0) return;

  const currentIdx = visible.findIndex((item) => item.id === focusedNodeId.value);

  if (e.key === "ArrowDown") {
    e.preventDefault();
    const next = currentIdx < visible.length - 1 ? currentIdx + 1 : 0;
    focusedNodeId.value = visible[next].id;
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    const prev = currentIdx > 0 ? currentIdx - 1 : visible.length - 1;
    focusedNodeId.value = visible[prev].id;
  } else if (e.key === "ArrowRight") {
    e.preventDefault();
    if (currentIdx >= 0) {
      const item = visible[currentIdx];
      if (item.type !== "request" && !store.isExpanded(item)) {
        store.toggleNode(item);
      }
    }
  } else if (e.key === "ArrowLeft") {
    e.preventDefault();
    if (currentIdx >= 0) {
      const item = visible[currentIdx];
      if (item.type !== "request" && store.isExpanded(item)) {
        store.toggleNode(item);
      }
    }
  } else if (e.key === "Enter") {
    e.preventDefault();
    if (currentIdx >= 0) {
      onNodeClick(visible[currentIdx]);
    }
  }
}

// Workspace interactions
// Track connection context for workspace context menu actions
const contextMenuConnectionId = ref<string | undefined>(undefined);

function onWorkspaceClick(ws: Workspace, context?: import("@/api/types").DataContext) {
  workspaceStore.toggleWorkspace(ws, context || { type: "local" });
}

function onWorkspaceContextMenu(event: MouseEvent, ws: Workspace, connectionId?: string) {
  const items: MenuItem[] = [];
  if (!ws.is_personal) {
    items.push({ label: "Workspace Settings", action: "workspace-settings" });
  }
  if (items.length > 0) {
    contextMenuConnectionId.value = connectionId;
    contextMenu.value = {
      x: event.clientX,
      y: event.clientY,
      items,
      target: { id: ws.id, type: "collection" as const, label: ws.name, data: ws as any, collectionId: ws.id },
    };
  }
}

defineExpose({
  updateCachedRequest: (req: SavedRequest) => store.updateCachedRequest(req),
  refreshCollections: () => store.refreshCollections(),
});
</script>

<template>
  <div
    ref="treeRef"
    class="collections-tree"
    role="tree"
    aria-label="Collections"
    tabindex="0"
    @keydown="onTreeKeydown"
  >
    <div class="tree-actions">
      <button v-if="workspaceStore.groups.length <= 1" class="new-collection-btn" @click="startNewCollection()">
        + New Collection
      </button>
      <button v-if="appStore.isWebOrganization" class="import-btn" @click="emit('create-workspace')">
        + Workspace
      </button>
      <button class="import-btn" @click="emit('open-import')">
        Import
      </button>
    </div>

    <div v-if="creatingCollection && !creatingInWorkspaceId" class="new-collection-input-wrap">
      <input
        ref="newCollectionInput"
        v-model="newCollectionName"
        class="new-collection-input"
        placeholder="Collection name..."
        @keydown.enter="commitNewCollection"
        @keydown.escape="creatingCollection = false"
        @blur="commitNewCollection"
      />
    </div>

    <div v-if="store.loading" class="loading-state">
      <span class="spinner"></span> Loading collections...
    </div>

    <!-- Workspace-grouped rendering (multiple groups: local + connections or org workspaces) -->
    <template v-if="workspaceStore.groups.length > 1">
      <div v-for="group in workspaceStore.groups" :key="group.id" class="sidebar-group">
        <div class="group-header" @click="toggleGroup(group.id)">
          <span class="group-chevron">{{ collapsedGroups.has(group.id) ? '\u25B8' : '\u25BE' }}</span>
          <span class="group-label">{{ group.label }}</span>
          <button
            v-if="group.type === 'connection'"
            class="group-action-btn"
            title="Create Workspace"
            @click.stop="emit('create-workspace', group.connectionId!)"
          >+</button>
        </div>
        <div v-if="!collapsedGroups.has(group.id)" class="group-content">
          <template v-if="group.type === 'local' && group.workspaces.length === 0">
            <!-- Local group with no workspaces: show collections directly -->
            <button class="new-collection-btn group-new-btn" @click="startNewCollection()">
              + New Collection
            </button>
            <div v-if="creatingCollection && !creatingInWorkspaceId" class="new-collection-input-wrap">
              <input
                ref="newCollectionInput"
                v-model="newCollectionName"
                class="new-collection-input"
                placeholder="Collection name..."
                @keydown.enter="commitNewCollection"
                @keydown.escape="creatingCollection = false"
                @blur="commitNewCollection"
              />
            </div>
            <template v-for="col in store.collections" :key="col.id">
              <TreeNode
                :ref="setNodeRef(col.id)"
                :type="'collection'"
                :label="col.name"
                :depth="0"
                :expanded="store.isExpanded(store.collectionAsTreeItem(col))"
                :active="false"
                :children="store.isExpanded(store.collectionAsTreeItem(col)) ? store.getChildren(store.collectionAsTreeItem(col)) : undefined"
                :active-request-id="activeRequestId"
                @toggle="store.toggleNode(store.collectionAsTreeItem(col))"
                @click="onNodeClick(store.collectionAsTreeItem(col))"
                @contextmenu="onContextMenu($event, store.collectionAsTreeItem(col))"
                @rename="onRename(store.collectionAsTreeItem(col), $event)"
                @rename-cancel="renamingNodeId = null"
                @child-toggle="store.toggleNode($event)"
                @child-click="onNodeClick($event)"
                @child-contextmenu="(ev: MouseEvent, item: TreeItem) => onContextMenu(ev, item)"
                @child-rename="(item: TreeItem, name: string) => onRename(item, name)"
                @child-rename-cancel="renamingNodeId = null"
              />
            </template>
          </template>
          <!-- Connection with workspaces -->
          <template v-else-if="group.workspaces.length > 0">
            <div v-for="sw in group.workspaces" :key="sw.workspace.id">
              <div class="workspace-item"
                @click="onWorkspaceClick(sw.workspace, sw.context)"
                @contextmenu.prevent="onWorkspaceContextMenu($event, sw.workspace, group.connectionId)"
              >
                <span class="workspace-chevron">{{ workspaceStore.isWorkspaceExpanded(sw.workspace.id) ? '\u25BE' : '\u25B8' }}</span>
                <span class="workspace-icon">{{ sw.workspace.is_personal ? '\uD83D\uDC64' : '\uD83D\uDC65' }}</span>
                <span class="workspace-label">{{ sw.workspace.name }}</span>
                <button
                  class="group-action-btn"
                  title="New Collection"
                  @click.stop="startNewCollection(sw.workspace.id, sw.context)"
                >+</button>
              </div>
              <!-- Inline collection creation for this workspace -->
              <div v-if="creatingCollection && creatingInWorkspaceId === sw.workspace.id" class="workspace-collections">
                <div class="new-collection-input-wrap">
                  <input
                    ref="newCollectionInput"
                    v-model="newCollectionName"
                    class="new-collection-input"
                    placeholder="Collection name..."
                    @keydown.enter="commitNewCollection"
                    @keydown.escape="creatingCollection = false"
                    @blur="commitNewCollection"
                  />
                </div>
              </div>
              <!-- Workspace collections -->
              <div v-if="workspaceStore.isWorkspaceExpanded(sw.workspace.id)" class="workspace-collections">
                <div v-if="workspaceStore.getWorkspaceCollections(sw.workspace.id).length === 0 && !(creatingCollection && creatingInWorkspaceId === sw.workspace.id)" class="workspace-empty">
                  No collections
                </div>
                <div v-for="col in workspaceStore.getWorkspaceCollections(sw.workspace.id)" :key="col.id" class="workspace-collection-item">
                  <span class="ws-col-icon">&#x1F4C1;</span>
                  <span class="ws-col-name">{{ col.name }}</span>
                </div>
              </div>
            </div>
          </template>
          <!-- Connection without workspaces: show remote collections directly -->
          <template v-else-if="group.remoteCollections && group.remoteCollections.length > 0">
            <div v-for="col in group.remoteCollections" :key="col.id" class="workspace-collection-item">
              <span class="ws-col-icon">&#x1F4C1;</span>
              <span class="ws-col-name">{{ col.name }}</span>
            </div>
          </template>
          <template v-else-if="group.type === 'connection'">
            <div class="workspace-empty">No collections on this server</div>
          </template>
        </div>
      </div>
    </template>

    <!-- Default flat rendering (single group or no groups) -->
    <template v-else>
      <div v-if="store.collections.length === 0 && !creatingCollection" class="empty-state">
        No collections yet
      </div>

      <template v-for="col in store.collections" :key="col.id">
        <TreeNode
          :ref="setNodeRef(col.id)"
          :type="'collection'"
          :label="col.name"
          :depth="0"
          :expanded="store.isExpanded(store.collectionAsTreeItem(col))"
          :active="false"
          :children="store.isExpanded(store.collectionAsTreeItem(col)) ? store.getChildren(store.collectionAsTreeItem(col)) : undefined"
          :active-request-id="activeRequestId"
          @toggle="store.toggleNode(store.collectionAsTreeItem(col))"
          @click="onNodeClick(store.collectionAsTreeItem(col))"
          @contextmenu="onContextMenu($event, store.collectionAsTreeItem(col))"
          @rename="onRename(store.collectionAsTreeItem(col), $event)"
          @rename-cancel="renamingNodeId = null"
          @child-toggle="store.toggleNode($event)"
          @child-click="onNodeClick($event)"
          @child-contextmenu="(ev: MouseEvent, item: TreeItem) => onContextMenu(ev, item)"
          @child-rename="(item: TreeItem, name: string) => onRename(item, name)"
          @child-rename-cancel="renamingNodeId = null"
        />
      </template>
    </template>

    <ContextMenu
      v-if="contextMenu"
      :x="contextMenu.x"
      :y="contextMenu.y"
      :items="contextMenu.items"
      @select="onContextAction"
      @close="contextMenu = null"
    />
  </div>
</template>

<style scoped>
.collections-tree {
  display: flex;
  flex-direction: column;
  gap: 2px;
  outline: none;
}

.tree-actions {
  display: flex;
  gap: 4px;
  margin-bottom: 4px;
}

.new-collection-btn {
  flex: 1;
  background: none;
  border: 1px dashed var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 12px;
  padding: 6px 8px;
  cursor: pointer;
  text-align: left;
  transition: all var(--transition);
}

.new-collection-btn:hover {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--accent-muted);
}

.import-btn {
  background: none;
  border: 1px dashed var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 12px;
  padding: 6px 10px;
  cursor: pointer;
  transition: all var(--transition);
  white-space: nowrap;
}

.import-btn:hover {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--accent-muted);
}

.new-collection-input-wrap {
  margin-bottom: 4px;
}

.new-collection-input {
  width: 100%;
  background: var(--bg-input);
  border: 1px solid var(--accent);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 13px;
  padding: 5px 8px;
  outline: none;
}

.loading-state {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: 12px;
  padding: 12px 8px;
}

.spinner {
  display: inline-block;
  width: 12px;
  height: 12px;
  border: 2px solid var(--border-color);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.empty-state {
  color: var(--text-muted);
  font-size: 13px;
  text-align: center;
  margin-top: 24px;
}

.sidebar-group {
  margin-bottom: 2px;
}

.group-header {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 8px;
  cursor: pointer;
  user-select: none;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  transition: all var(--transition);
}

.group-header:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.group-chevron {
  font-size: 10px;
  width: 12px;
  text-align: center;
}

.group-label {
  flex: 1;
}

.group-action-btn {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 14px;
  cursor: pointer;
  padding: 0 4px;
  line-height: 1;
  border-radius: var(--radius-sm);
  opacity: 0;
  transition: all var(--transition);
}

.group-header:hover .group-action-btn {
  opacity: 1;
}

.group-action-btn:hover {
  color: var(--accent);
  background: var(--bg-hover);
}

.group-content {
  padding-left: 4px;
}

.group-new-btn {
  margin: 2px 0 4px;
  width: 100%;
}

.workspace-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px 4px 20px;
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: all var(--transition);
}

.workspace-item:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.workspace-item:hover .group-action-btn {
  opacity: 1;
}

.workspace-icon {
  font-size: 12px;
  width: 16px;
  text-align: center;
}

.workspace-chevron {
  font-size: 10px;
  width: 12px;
  text-align: center;
  color: var(--text-muted);
}

.workspace-label {
  font-size: 12px;
}

.workspace-collections {
  padding-left: 32px;
}

.workspace-empty {
  font-size: 11px;
  color: var(--text-muted);
  padding: 4px 8px;
}

.workspace-collection-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 8px;
  font-size: 12px;
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition);
}

.workspace-collection-item:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.ws-col-icon {
  font-size: 11px;
}

.ws-col-name {
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
