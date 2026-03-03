import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { getCollection } from "@/api/client";
import type {
  TabState,
  FormState,
  AuthConfig,
  SavedRequest,
  HistoryEntry,
  Collection,
} from "@/api/types";

function createEmptyTab(): TabState {
  return {
    id: Date.now().toString(),
    savedRequest: null,
    historyEntry: null,
    formState: null,
    isDirty: false,
    name: "New Request",
    method: "GET",
    collectionAuth: undefined,
    requestType: "http",
  };
}

export const useTabStore = defineStore("tabs", () => {
  const tabs = ref<TabState[]>([createEmptyTab()]);
  const activeTabIndex = ref(0);

  const activeTab = computed(() => tabs.value[activeTabIndex.value]);

  function saveCurrentTabFormState(formState: FormState | null) {
    const tab = activeTab.value;
    if (!tab) return;
    if (formState) {
      tab.formState = formState;
      if (formState.type === "http") {
        tab.method = formState.method;
        if (!tab.savedRequest) {
          const urlPart = formState.url ? formState.url.split("?")[0].split("/").pop() || "" : "";
          tab.name = urlPart || "New Request";
        }
      } else if (formState.type === "graphql") {
        tab.method = "GQL";
        if (!tab.savedRequest) {
          const urlPart = formState.url ? formState.url.split("?")[0].split("/").pop() || "" : "";
          tab.name = urlPart || "GraphQL";
        }
      } else {
        if (!tab.savedRequest) {
          tab.name = formState.url || "WebSocket";
        }
      }
    }
  }

  function switchToTab(index: number) {
    if (index === activeTabIndex.value) return;
    activeTabIndex.value = index;
  }

  function addNewTab() {
    const newTab = createEmptyTab();
    tabs.value.push(newTab);
    activeTabIndex.value = tabs.value.length - 1;
  }

  function closeTab(index: number) {
    if (tabs.value.length === 1) {
      tabs.value[0] = createEmptyTab();
      activeTabIndex.value = 0;
      return;
    }
    if (index !== activeTabIndex.value) {
      tabs.value.splice(index, 1);
      if (activeTabIndex.value > index) {
        activeTabIndex.value--;
      }
      return;
    }
    tabs.value.splice(index, 1);
    if (activeTabIndex.value >= tabs.value.length) {
      activeTabIndex.value = tabs.value.length - 1;
    }
  }

  async function loadSavedRequest(req: SavedRequest) {
    const tab = activeTab.value;
    tab.savedRequest = req;
    tab.historyEntry = null;
    tab.name = req.name;
    tab.method = req.method;
    tab.requestType = req.request_type === "websocket" ? "websocket" : req.request_type === "graphql" ? "graphql" : "http";
    tab.formState = null;
    try {
      const col = await getCollection(req.collection_id);
      tab.collectionAuth = col.auth;
    } catch {
      tab.collectionAuth = undefined;
    }
  }

  function loadHistoryEntry(entry: HistoryEntry) {
    const tab = activeTab.value;
    tab.savedRequest = null;
    tab.historyEntry = entry;
    tab.name = entry.url.split("?")[0].split("/").pop() || entry.method;
    tab.method = entry.method;
    tab.formState = null;
  }

  function onRequestDeleted(id: string) {
    const tab = activeTab.value;
    if (tab.savedRequest?.id === id) {
      tab.savedRequest = null;
    }
  }

  function updateCurrentSavedRequest(req: SavedRequest) {
    activeTab.value.savedRequest = req;
  }

  function clearCurrentRequest() {
    const tab = activeTab.value;
    tab.savedRequest = null;
    tab.historyEntry = null;
    tab.collectionAuth = undefined;
    tab.formState = null;
  }

  function updateCollectionAuth(auth: AuthConfig | undefined) {
    activeTab.value.collectionAuth = auth;
  }

  function onCollectionAuthSaved(col: Collection) {
    const tab = activeTab.value;
    if (tab.savedRequest && tab.savedRequest.collection_id === col.id) {
      tab.collectionAuth = col.auth;
    }
  }

  function setRequestType(type: "http" | "websocket" | "graphql") {
    const tab = activeTab.value;
    tab.requestType = type;
    if (type === "websocket") {
      tab.method = "WS";
    } else if (type === "graphql") {
      tab.method = "GQL";
    }
  }

  return {
    tabs,
    activeTabIndex,
    activeTab,
    saveCurrentTabFormState,
    switchToTab,
    addNewTab,
    closeTab,
    loadSavedRequest,
    loadHistoryEntry,
    onRequestDeleted,
    updateCurrentSavedRequest,
    clearCurrentRequest,
    updateCollectionAuth,
    onCollectionAuthSaved,
    setRequestType,
  };
});
