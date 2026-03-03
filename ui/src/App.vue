<script setup lang="ts">
import { ref, nextTick, onMounted } from "vue";
import { useSettings } from "@/composables/useSettings";
import { useHotkey } from "@/composables/useHotkey";
import { useApiCall } from "@/composables/useApiCall";
import { useTabStore } from "@/stores/tabStore";
import { useAppStore } from "@/stores/appStore";
import { useAuthStore } from "@/stores/authStore";
import { useWorkspaceStore } from "@/stores/workspaceStore";
import LoginPage from "./components/LoginPage.vue";

const { resolvedTheme } = useSettings();
const tabStore = useTabStore();
const appStore = useAppStore();
const authStore = useAuthStore();
const workspaceStore = useWorkspaceStore();
const { execute } = useApiCall();

onMounted(async () => {
  await appStore.detectMode();
  if (appStore.needsAuth) {
    await authStore.loadUser();
  }

  // Handle workspace invite acceptance
  const urlParams = new URLSearchParams(window.location.search);
  const inviteToken = urlParams.get('invite');
  if (inviteToken && authStore.isAuthenticated) {
    try {
      await acceptWorkspaceInvite(inviteToken);
      window.history.replaceState({}, '', window.location.pathname);
      showToast("Workspace invite accepted!", "success");
    } catch {
      showToast("Failed to accept invite", "error");
    }
  }
});

import RequestBuilder from "./components/RequestBuilder.vue";
import WsPanel from "./components/WsPanel.vue";
import GraphQLPanel from "./components/GraphQLPanel.vue";
import SidebarTabs from "./components/sidebar/SidebarTabs.vue";
import SaveDialog from "./components/sidebar/SaveDialog.vue";
import EnvironmentSelector from "./components/EnvironmentSelector.vue";
import ManageEnvironments from "./components/ManageEnvironments.vue";
import CollectionAuthDialog from "./components/CollectionAuthDialog.vue";
import ImportDialog from "./components/ImportDialog.vue";
import TabBar from "./components/TabBar.vue";
import ShortcutsDialog from "./components/ShortcutsDialog.vue";
import SettingsDialog from "./components/SettingsDialog.vue";
import CreateWorkspaceDialog from "./components/CreateWorkspaceDialog.vue";
import WorkspaceSettingsDialog from "./components/WorkspaceSettingsDialog.vue";
import ToastContainer from "./components/ToastContainer.vue";
import { useToast } from "@/composables/useToast";

const { showToast } = useToast();
import {
  updateRequest,
  createRequest,
  acceptWorkspaceInvite,
  type SavedRequest,
  type HistoryEntry,
  type Collection,
  type CurlImportResult,
} from "@/api/client";

const sidebarOpen = ref(true);
const showSaveDialog = ref(false);
const showManageEnvs = ref(false);
const sidebarTabs = ref<InstanceType<typeof SidebarTabs> | null>(null);
const requestBuilder = ref<InstanceType<typeof RequestBuilder> | null>(null);
const envSelector = ref<InstanceType<typeof EnvironmentSelector> | null>(null);
const wsPanel = ref<InstanceType<typeof WsPanel> | null>(null);
const graphqlPanel = ref<InstanceType<typeof GraphQLPanel> | null>(null);

// Environment state
const activeEnvironmentId = ref<string | undefined>(undefined);

// Import dialog
const showImportDialog = ref(false);

// Settings dialog
const showSettings = ref(false);

// Collection auth dialog
const editingCollectionAuthId = ref<string | null>(null);

// Shortcuts dialog
const showShortcuts = ref(false);

// Workspace dialogs
const createWorkspaceConnectionId = ref<string | null>(null);
const showCreateWorkspace = ref(false);
const workspaceSettingsId = ref<string | null>(null);
const workspaceSettingsConnectionId = ref<string | null>(null);

function saveCurrentTabState() {
  const tab = tabStore.activeTab;
  if (tab.requestType === "websocket") {
    const wsData = wsPanel.value?.getFormData();
    if (wsData) {
      tabStore.saveCurrentTabFormState({ type: "websocket", url: wsData.url, headers: wsData.headers });
    }
  } else if (tab.requestType === "graphql") {
    const gqlData = graphqlPanel.value?.getFormData();
    if (gqlData) {
      tabStore.saveCurrentTabFormState({
        type: "graphql",
        url: gqlData.url,
        headers: gqlData.headers,
        auth: gqlData.auth,
        graphql_query: gqlData.graphql_query,
        graphql_variables: gqlData.graphql_variables,
        graphql_operation_name: gqlData.graphql_operation_name,
      });
    }
  } else {
    const formData = requestBuilder.value?.getFormData();
    if (formData) {
      tabStore.saveCurrentTabFormState({
        type: "http",
        method: formData.method,
        url: formData.url,
        headers: formData.headers,
        body: formData.body,
        params: formData.params,
        auth: formData.auth,
        pre_request_script: formData.pre_request_script,
        test_script: formData.test_script,
      });
    }
  }
}

function restoreTabState() {
  const tab = tabStore.activeTab;
  nextTick(() => {
    if (tab.formState && tab.formState.type === "http" && requestBuilder.value) {
      requestBuilder.value.loadCurlImport({
        name: tab.name,
        method: tab.formState.method,
        url: tab.formState.url,
        headers: tab.formState.headers || {},
        body: tab.formState.body,
        params: tab.formState.params || [],
        auth: tab.formState.auth || { type: "none" },
      });
    }
  });
}

function switchToTab(index: number) {
  if (index === tabStore.activeTabIndex) return;
  saveCurrentTabState();
  tabStore.switchToTab(index);
  restoreTabState();
}

function addNewTab() {
  saveCurrentTabState();
  tabStore.addNewTab();
}

function closeTab(index: number) {
  if (tabStore.tabs.length === 1) {
    tabStore.closeTab(index);
    return;
  }
  const wasActive = index === tabStore.activeTabIndex;
  if (wasActive) saveCurrentTabState();
  tabStore.closeTab(index);
  if (wasActive) restoreTabState();
}

function onEditCollectionAuth(collectionId: string) {
  editingCollectionAuthId.value = collectionId;
}

function onCollectionAuthSaved(col: Collection) {
  editingCollectionAuthId.value = null;
  tabStore.onCollectionAuthSaved(col);
}

async function onLoadSavedRequest(req: SavedRequest) {
  await tabStore.loadSavedRequest(req);
}

function onLoadHistoryEntry(entry: HistoryEntry) {
  tabStore.loadHistoryEntry(entry);
}

function onRequestDeleted(id: string) {
  tabStore.onRequestDeleted(id);
}

function onHistoryRecorded() {
  sidebarTabs.value?.refreshHistory();
}

async function onSaveRequested() {
  const tab = tabStore.activeTab;
  if (tab.savedRequest) {
    await execute(async () => {
      if (tab.requestType === "websocket") {
        const wsData = wsPanel.value?.getFormData();
        if (!wsData) return;
        const updated = await updateRequest({
          ...tab.savedRequest!,
          url: wsData.url,
          headers: wsData.headers,
          request_type: "websocket",
        });
        tabStore.updateCurrentSavedRequest(updated);
        sidebarTabs.value?.updateCachedRequest(updated);
        showToast("Request saved", "success");
      } else if (tab.requestType === "graphql") {
        const gqlData = graphqlPanel.value?.getFormData();
        if (!gqlData) return;
        const updated = await updateRequest({
          ...tab.savedRequest!,
          url: gqlData.url,
          headers: gqlData.headers,
          auth: gqlData.auth,
          graphql_query: gqlData.graphql_query,
          graphql_variables: gqlData.graphql_variables,
          graphql_operation_name: gqlData.graphql_operation_name,
          request_type: "graphql",
        });
        tabStore.updateCurrentSavedRequest(updated);
        sidebarTabs.value?.updateCachedRequest(updated);
        showToast("Request saved", "success");
      } else {
        const formData = requestBuilder.value?.getFormData();
        if (!formData) return;
        const updated = await updateRequest({
          ...tab.savedRequest!,
          method: formData.method as SavedRequest["method"],
          url: formData.url,
          headers: formData.headers,
          body: formData.body,
          params: formData.params,
          auth: formData.auth,
          pre_request_script: formData.pre_request_script,
          test_script: formData.test_script,
          request_type: "http",
        });
        tabStore.updateCurrentSavedRequest(updated);
        sidebarTabs.value?.updateCachedRequest(updated);
        showToast("Request saved", "success");
      }
    });
  } else {
    showSaveDialog.value = true;
  }
}

async function onSaveDialogSave(data: { name: string; collectionId: string; folderId?: string }) {
  const tab = tabStore.activeTab;
  await execute(async () => {
    if (tab.requestType === "websocket") {
      const wsData = wsPanel.value?.getFormData();
      if (!wsData) return;
      const req: SavedRequest = {
        id: "",
        collection_id: data.collectionId,
        folder_id: data.folderId,
        name: data.name,
        method: "GET",
        url: wsData.url,
        headers: wsData.headers,
        body: undefined,
        params: [],
        sort_order: 0,
        created_at: "",
        updated_at: "",
        request_type: "websocket",
      };
      const created = await createRequest(req);
      tabStore.updateCurrentSavedRequest(created);
      showSaveDialog.value = false;
      showToast("Request saved", "success");
    } else if (tab.requestType === "graphql") {
      const gqlData = graphqlPanel.value?.getFormData();
      if (!gqlData) return;
      const req: SavedRequest = {
        id: "",
        collection_id: data.collectionId,
        folder_id: data.folderId,
        name: data.name,
        method: "POST",
        url: gqlData.url,
        headers: gqlData.headers,
        auth: gqlData.auth,
        graphql_query: gqlData.graphql_query,
        graphql_variables: gqlData.graphql_variables,
        graphql_operation_name: gqlData.graphql_operation_name,
        params: [],
        sort_order: 0,
        created_at: "",
        updated_at: "",
        request_type: "graphql",
      };
      const created = await createRequest(req);
      tabStore.updateCurrentSavedRequest(created);
      showSaveDialog.value = false;
      showToast("Request saved", "success");
    } else {
      const formData = requestBuilder.value?.getFormData();
      if (!formData) return;
      const req: SavedRequest = {
        id: "",
        collection_id: data.collectionId,
        folder_id: data.folderId,
        name: data.name,
        method: formData.method as SavedRequest["method"],
        url: formData.url,
        headers: formData.headers,
        body: formData.body,
        params: formData.params,
        auth: formData.auth,
        pre_request_script: formData.pre_request_script,
        test_script: formData.test_script,
        sort_order: 0,
        created_at: "",
        updated_at: "",
        request_type: "http",
      };
      const created = await createRequest(req);
      tabStore.updateCurrentSavedRequest(created);
      showSaveDialog.value = false;
      showToast("Request saved", "success");
    }
  });
}

function onImportedCollection(_col: Collection) {
  showImportDialog.value = false;
  sidebarTabs.value?.refreshCollections();
  showToast("Collection imported", "success");
}

function onImportedCurl(result: CurlImportResult) {
  showImportDialog.value = false;
  tabStore.clearCurrentRequest();
  requestBuilder.value?.loadCurlImport(result);
  showToast("cURL imported", "success");
}

function onManageEnvsClosed() {
  showManageEnvs.value = false;
  envSelector.value?.refresh();
}

// Keyboard shortcuts
useHotkey([
  { key: "s", mod: true, handler: () => onSaveRequested() },
  { key: "Enter", mod: true, handler: () => {
    if (tabStore.activeTab.requestType === "graphql") graphqlPanel.value?.send();
    else requestBuilder.value?.send();
  } },
  { key: "n", mod: true, handler: () => addNewTab() },
  { key: "w", mod: true, handler: () => closeTab(tabStore.activeTabIndex) },
  { key: "l", mod: true, handler: () => {
    if (tabStore.activeTab.requestType === "graphql") graphqlPanel.value?.focusUrl();
    else requestBuilder.value?.focusUrl();
  } },
  { key: ",", mod: true, handler: () => { showSettings.value = true; } },
  { key: "?", mod: false, nonInput: true, handler: () => { showShortcuts.value = true; } },
]);
</script>

<template>
  <div v-if="appStore.loading" class="loading-screen">Loading...</div>
  <LoginPage v-else-if="appStore.needsAuth && !authStore.isAuthenticated" />
  <div v-else id="apihop" class="app-layout" :data-theme="resolvedTheme">
    <aside class="sidebar" :class="{ collapsed: !sidebarOpen }">
      <div class="sidebar-header">
        <span v-if="sidebarOpen" class="sidebar-title">apihop</span>
        <span v-if="sidebarOpen && appStore.isWebOrganization && authStore.user" class="sidebar-user">
          {{ authStore.user.display_name || authStore.user.email }}
          <button class="logout-btn" @click="authStore.logout()">Logout</button>
        </span>
        <button class="sidebar-toggle" @click="sidebarOpen = !sidebarOpen">
          {{ sidebarOpen ? "\u2039" : "\u203A" }}
        </button>
      </div>
      <div v-if="sidebarOpen" class="sidebar-env">
        <EnvironmentSelector
          ref="envSelector"
          :active-environment-id="activeEnvironmentId"
          @update:active-environment-id="activeEnvironmentId = $event"
          @open-manage="showManageEnvs = true"
        />
      </div>
      <div v-if="sidebarOpen" class="sidebar-content">
        <SidebarTabs
          ref="sidebarTabs"
          :active-request-id="tabStore.activeTab.savedRequest?.id"
          @load-saved-request="onLoadSavedRequest"
          @load-history-entry="onLoadHistoryEntry"
          @request-deleted="onRequestDeleted"
          @edit-collection-auth="onEditCollectionAuth"
          @open-import="showImportDialog = true"
          @create-workspace="(connId?: string) => { createWorkspaceConnectionId = connId ?? null; showCreateWorkspace = true; }"
          @workspace-settings="(wsId: string, connId?: string) => { workspaceSettingsId = wsId; workspaceSettingsConnectionId = connId ?? null; }"
        />
      </div>
    </aside>
    <main class="main-content">
      <TabBar
        :tabs="tabStore.tabs"
        :active-tab-index="tabStore.activeTabIndex"
        @select="switchToTab"
        @close="closeTab"
        @new-tab="addNewTab"
      />
      <div v-if="!tabStore.activeTab.savedRequest" class="request-type-toggle">
        <button
          :class="{ active: tabStore.activeTab.requestType === 'http' }"
          @click="tabStore.setRequestType('http')"
        >HTTP</button>
        <button
          :class="{ active: tabStore.activeTab.requestType === 'websocket' }"
          @click="tabStore.setRequestType('websocket')"
        >WS</button>
        <button
          :class="{ active: tabStore.activeTab.requestType === 'graphql' }"
          @click="tabStore.setRequestType('graphql')"
        >GQL</button>
      </div>
      <RequestBuilder
        v-if="tabStore.activeTab.requestType === 'http'"
        ref="requestBuilder"
        :saved-request="tabStore.activeTab.savedRequest"
        :history-entry="tabStore.activeTab.historyEntry"
        :collection-auth="tabStore.activeTab.collectionAuth"
        :environment-id="activeEnvironmentId"
        @save="onSaveRequested"
        @history-recorded="onHistoryRecorded"
      />
      <GraphQLPanel
        v-else-if="tabStore.activeTab.requestType === 'graphql'"
        ref="graphqlPanel"
        :saved-request="tabStore.activeTab.savedRequest"
        :collection-auth="tabStore.activeTab.collectionAuth"
        :environment-id="activeEnvironmentId"
        @save="onSaveRequested"
        @history-recorded="onHistoryRecorded"
      />
      <WsPanel
        v-else
        ref="wsPanel"
        :environment-id="activeEnvironmentId"
        :saved-headers="tabStore.activeTab.savedRequest?.headers"
        :saved-url="tabStore.activeTab.savedRequest?.url || 'wss://'"
      />
    </main>

    <SaveDialog
      v-if="showSaveDialog"
      :initial-name="undefined"
      :initial-collection-id="tabStore.activeTab.savedRequest?.collection_id"
      :initial-folder-id="tabStore.activeTab.savedRequest?.folder_id"
      @save="onSaveDialogSave"
      @cancel="showSaveDialog = false"
    />

    <ManageEnvironments
      v-if="showManageEnvs"
      @close="onManageEnvsClosed"
    />

    <CollectionAuthDialog
      v-if="editingCollectionAuthId"
      :collection-id="editingCollectionAuthId"
      @close="editingCollectionAuthId = null"
      @saved="onCollectionAuthSaved"
    />

    <ImportDialog
      v-if="showImportDialog"
      @close="showImportDialog = false"
      @imported-collection="onImportedCollection"
      @imported-curl="onImportedCurl"
    />

    <SettingsDialog
      v-if="showSettings"
      @close="showSettings = false"
      @connections-changed="workspaceStore.loadSidebarGroups(); sidebarTabs?.refreshCollections()"
    />

    <ShortcutsDialog
      v-if="showShortcuts"
      @close="showShortcuts = false"
    />

    <CreateWorkspaceDialog
      v-if="showCreateWorkspace"
      :connection-id="createWorkspaceConnectionId"
      @created="showCreateWorkspace = false; sidebarTabs?.refreshCollections()"
      @close="showCreateWorkspace = false"
    />

    <WorkspaceSettingsDialog
      v-if="workspaceSettingsId"
      :workspace-id="workspaceSettingsId"
      :connection-id="workspaceSettingsConnectionId"
      @close="workspaceSettingsId = null"
      @updated="workspaceSettingsId = null; sidebarTabs?.refreshCollections()"
      @deleted="workspaceSettingsId = null; sidebarTabs?.refreshCollections()"
    />

    <ToastContainer />
  </div>
</template>

<style scoped>
.loading-screen {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100vh;
  color: var(--text-secondary);
  font-size: 14px;
}

.app-layout {
  display: flex;
  height: 100vh;
  overflow: hidden;
}

.sidebar {
  width: 250px;
  background: linear-gradient(180deg, var(--bg-secondary) 0%, var(--bg-primary) 100%);
  box-shadow: var(--shadow-sm);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  transition: width 0.2s;
  z-index: 1;
}

.sidebar.collapsed {
  width: 40px;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px;
  border-bottom: 1px solid var(--border-subtle);
}

.sidebar-title {
  font-weight: 700;
  font-size: 14px;
  letter-spacing: 0.3px;
}

.sidebar-toggle {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 18px;
  cursor: pointer;
  padding: 2px 6px;
  line-height: 1;
  border-radius: var(--radius-sm);
  transition: all var(--transition);
}

.sidebar-toggle:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.sidebar-env {
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-subtle);
}

.sidebar-content {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.main-content {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.request-type-toggle {
  display: flex;
  gap: 2px;
  padding: 6px 16px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-subtle);
}

.request-type-toggle button {
  background: var(--bg-tertiary);
  border: none;
  color: var(--text-secondary);
  font-size: 11px;
  font-weight: 600;
  padding: 4px 12px;
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: all var(--transition);
}

.request-type-toggle button.active {
  background: var(--accent);
  color: #fff;
}

.request-type-toggle button:hover:not(.active) {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.sidebar-user {
  font-size: 11px;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: 8px;
}

.logout-btn {
  background: none;
  border: 1px solid var(--border-default);
  color: var(--text-secondary);
  font-size: 10px;
  padding: 2px 6px;
  border-radius: var(--radius-sm, 4px);
  cursor: pointer;
}

.logout-btn:hover {
  color: var(--color-error, #e74c3c);
  border-color: var(--color-error, #e74c3c);
}
</style>
