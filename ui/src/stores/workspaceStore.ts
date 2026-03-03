import { defineStore } from "pinia";
import { ref } from "vue";
import type { SidebarGroup, SidebarWorkspace, Workspace, DataContext, Collection } from "@/api/types";
import { listWorkspaces, listConnections, connectionProxy, listWorkspaceCollections } from "@/api/client";
import { useAppStore } from "./appStore";

const IS_TAURI = "__TAURI_INTERNALS__" in window;

export const useWorkspaceStore = defineStore("workspaces", () => {
  const groups = ref<SidebarGroup[]>([]);
  const loading = ref(false);
  const activeWorkspaceId = ref<string | null>(null);
  const activeContext = ref<DataContext>({ type: "local" });

  // Workspace collections cache: workspaceId -> collections
  const workspaceCollections = ref<Record<string, Collection[]>>({});
  const expandedWorkspaces = ref(new Set<string>());

  async function loadSidebarGroups() {
    loading.value = true;
    try {
      const appStore = useAppStore();

      if (IS_TAURI || !appStore.isWebOrganization) {
        // Desktop or Web-Personal mode: local group + connection groups
        const localGroup: SidebarGroup = {
          id: "local",
          label: "Local",
          type: "local",
          workspaces: [],
        };
        const newGroups: SidebarGroup[] = [localGroup];

        // Load connected servers
        try {
          const connections = await listConnections();
          for (const conn of connections) {
            if (conn.status !== "connected") continue;

            // Always add the connection group
            const connGroup: SidebarGroup = {
              id: `conn-${conn.id}`,
              label: conn.display_name,
              type: "connection",
              connectionId: conn.id,
              workspaces: [],
            };

            // Try to load workspaces from remote server
            try {
              const remoteWorkspaces = (await connectionProxy(
                conn.id,
                "GET",
                "/api/v1/workspaces"
              )) as Workspace[];
              connGroup.workspaces = remoteWorkspaces.map((ws) => ({
                workspace: ws,
                context: {
                  type: "remote" as const,
                  connectionId: conn.id,
                },
              }));
            } catch (e) {
              console.warn(`Workspaces not available on ${conn.display_name}, loading collections:`, e);
              // Fallback: load collections directly
              try {
                const remoteCollections = (await connectionProxy(
                  conn.id,
                  "GET",
                  "/api/v1/collections"
                )) as Collection[];
                connGroup.remoteCollections = remoteCollections;
              } catch (e2) {
                console.warn(`Failed to load collections from ${conn.display_name}:`, e2);
              }
            }

            newGroups.push(connGroup);
          }
        } catch (e) {
          console.warn("Failed to load connections:", e);
        }

        groups.value = newGroups;
      } else {
        // Web-Organization mode: list workspaces directly
        const workspaces = await listWorkspaces();
        const personalWs = workspaces.filter((ws) => ws.is_personal);
        const sharedWs = workspaces.filter((ws) => !ws.is_personal);

        const sidebarGroups: SidebarGroup[] = [];

        if (personalWs.length > 0) {
          sidebarGroups.push({
            id: "personal",
            label: "Personal",
            type: "local",
            workspaces: personalWs.map((ws) => ({
              workspace: ws,
              context: { type: "local" as const },
            })),
          });
        }

        if (sharedWs.length > 0) {
          sidebarGroups.push({
            id: "shared",
            label: "Shared Workspaces",
            type: "local",
            workspaces: sharedWs.map((ws) => ({
              workspace: ws,
              context: { type: "local" as const },
            })),
          });
        }

        groups.value = sidebarGroups;
      }
    } finally {
      loading.value = false;
    }
  }

  async function toggleWorkspace(workspace: Workspace, context: DataContext) {
    const wsId = workspace.id;
    if (expandedWorkspaces.value.has(wsId)) {
      expandedWorkspaces.value.delete(wsId);
      return;
    }

    expandedWorkspaces.value.add(wsId);

    // Load collections if not cached
    if (!workspaceCollections.value[wsId]) {
      try {
        let collections: Collection[];
        if (context.type === "remote") {
          collections = (await connectionProxy(
            context.connectionId,
            "GET",
            `/api/v1/workspaces/${wsId}/collections`
          )) as Collection[];
        } else {
          collections = await listWorkspaceCollections(wsId);
        }
        workspaceCollections.value[wsId] = collections;
      } catch (e) {
        console.warn(`Failed to load collections for workspace ${workspace.name}:`, e);
        workspaceCollections.value[wsId] = [];
      }
    }
  }

  function isWorkspaceExpanded(wsId: string): boolean {
    return expandedWorkspaces.value.has(wsId);
  }

  function getWorkspaceCollections(wsId: string): Collection[] {
    return workspaceCollections.value[wsId] || [];
  }

  function setActiveWorkspace(
    workspaceId: string | null,
    context: DataContext
  ) {
    activeWorkspaceId.value = workspaceId;
    activeContext.value = context;
  }

  return {
    groups,
    loading,
    activeWorkspaceId,
    activeContext,
    expandedWorkspaces,
    workspaceCollections,
    loadSidebarGroups,
    toggleWorkspace,
    isWorkspaceExpanded,
    getWorkspaceCollections,
    setActiveWorkspace,
  };
});
