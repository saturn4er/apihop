import { defineStore } from "pinia";
import { ref } from "vue";
import {
  listCollections,
  listFolders,
  listRequests,
  type Collection,
  type SavedRequest,
} from "@/api/client";
import type { TreeItem } from "@/api/types";

export const useCollectionStore = defineStore("collections", () => {
  const collections = ref<Collection[]>([]);
  const expandedNodes = ref(new Set<string>());
  const childrenCache = ref(new Map<string, TreeItem[]>());
  const loadingNodes = ref(new Set<string>());
  const loading = ref(false);

  function nodeKey(type: string, id: string, parentId?: string): string {
    return `${type}:${id}:${parentId || "root"}`;
  }

  async function refreshCollections() {
    loading.value = true;
    try {
      collections.value = await listCollections();
    } finally {
      loading.value = false;
    }
  }

  async function fetchChildren(collectionId: string, folderId?: string): Promise<TreeItem[]> {
    const [folders, requests] = await Promise.all([
      listFolders(collectionId, folderId),
      listRequests(collectionId, folderId),
    ]);

    return [
      ...folders.map((f) => ({
        id: f.id,
        type: "folder" as const,
        label: f.name,
        collectionId,
        parentFolderId: folderId,
        data: f,
      })),
      ...requests.map((r) => ({
        id: r.id,
        type: "request" as const,
        label: r.name,
        collectionId,
        parentFolderId: folderId,
        method: r.method,
        requestType: r.request_type,
        data: r,
      })),
    ];
  }

  async function toggleNode(item: TreeItem) {
    const key = nodeKey(item.type, item.id);
    if (expandedNodes.value.has(key)) {
      expandedNodes.value.delete(key);
      return;
    }
    expandedNodes.value.add(key);
    if (!childrenCache.value.has(key)) {
      loadingNodes.value.add(key);
      try {
        const folderId = item.type === "folder" ? item.id : undefined;
        const collId = item.type === "collection" ? item.id : item.collectionId;
        const children = await fetchChildren(collId, folderId);
        childrenCache.value.set(key, children);
      } finally {
        loadingNodes.value.delete(key);
      }
    }
  }

  function getChildren(item: TreeItem): TreeItem[] {
    const key = nodeKey(item.type, item.id);
    return childrenCache.value.get(key) || [];
  }

  function isExpanded(item: TreeItem): boolean {
    return expandedNodes.value.has(nodeKey(item.type, item.id));
  }

  async function refreshNodeChildren(item: TreeItem) {
    const key = nodeKey(item.type, item.id);
    if (!expandedNodes.value.has(key)) {
      expandedNodes.value.add(key);
    }
    const collId = item.type === "collection" ? item.id : item.collectionId;
    const folderId = item.type === "folder" ? item.id : undefined;
    const children = await fetchChildren(collId, folderId);
    childrenCache.value.set(key, children);
  }

  async function refreshParentChildren(item: TreeItem) {
    if (item.parentFolderId) {
      const parentKey = nodeKey("folder", item.parentFolderId);
      const children = await fetchChildren(item.collectionId, item.parentFolderId);
      childrenCache.value.set(parentKey, children);
    } else {
      const parentKey = nodeKey("collection", item.collectionId);
      const children = await fetchChildren(item.collectionId);
      childrenCache.value.set(parentKey, children);
    }
  }

  function updateCachedRequest(req: SavedRequest) {
    for (const [_key, items] of childrenCache.value) {
      for (let i = 0; i < items.length; i++) {
        if (items[i].type === "request" && items[i].id === req.id) {
          items[i] = {
            ...items[i],
            label: req.name,
            method: req.method,
            data: req,
          };
          return;
        }
      }
    }
  }

  function collectionAsTreeItem(c: Collection): TreeItem {
    return {
      id: c.id,
      type: "collection",
      label: c.name,
      collectionId: c.id,
      data: c,
    };
  }

  return {
    collections,
    expandedNodes,
    childrenCache,
    loadingNodes,
    loading,
    nodeKey,
    refreshCollections,
    fetchChildren,
    toggleNode,
    getChildren,
    isExpanded,
    refreshNodeChildren,
    refreshParentChildren,
    updateCachedRequest,
    collectionAsTreeItem,
  };
});
