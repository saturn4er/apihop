// Re-export all types from types.ts for backward compatibility
export type {
  ApiRequest,
  ApiResponse,
  Environment,
  Variable,
  ApiKeyLocation,
  AuthConfig,
  ConsoleEntry,
  TestResult,
  ScriptExecutionResult,
  SendRequestPayload,
  SendRequestResponse,
  Collection,
  Folder,
  KeyValueParam,
  SavedRequest,
  WsDirection,
  WsStatus,
  WsMessage,
  WsConnectPayload,
  WsConnectResult,
  WsSessionSummary,
  HistoryEntry,
  CurlImportResult,
  GraphQLSchema,
  GraphQLType,
  GraphQLField,
  GraphQLInputValue,
  GraphQLTypeRef,
  GraphQLEnumValue,
  GraphQLDirective,
  GraphQLIntrospectPayload,
  ExtractionSource,
  ExtractionRule,
  ExtractedVariable,
  ServerInfo,
  AuthTokens,
  AuthUser,
  ServerConnection,
  Workspace,
  WorkspaceMember,
  WorkspaceInvite,
  DataContext,
  SidebarGroup,
  SidebarWorkspace,
  AppMode,
} from "./types";

import type {
  ApiResponse,
  Environment,
  Variable,
  AuthConfig,
  SendRequestPayload,
  SendRequestResponse,
  Collection,
  Folder,
  SavedRequest,
  HistoryEntry,
  CurlImportResult,
  WsConnectPayload,
  WsConnectResult,
  WsStatus,
  WsMessage,
  KeyValueParam,
  GraphQLSchema,
  GraphQLIntrospectPayload,
  AuthTokens,
  ServerInfo,
  ServerConnection,
  AuthUser,
  Workspace,
  WorkspaceMember,
  WorkspaceInvite,
} from "./types";

const IS_TAURI = "__TAURI_INTERNALS__" in window;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

async function tryRefreshToken(): Promise<boolean> {
  const refreshToken = localStorage.getItem('apihop_refresh_token');
  if (!refreshToken) return false;
  try {
    const res = await fetch('/api/v1/auth/refresh', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refresh_token: refreshToken }),
    });
    if (!res.ok) {
      localStorage.removeItem('apihop_access_token');
      localStorage.removeItem('apihop_refresh_token');
      return false;
    }
    const tokens: AuthTokens = await res.json();
    localStorage.setItem('apihop_access_token', tokens.access_token);
    localStorage.setItem('apihop_refresh_token', tokens.refresh_token);
    return true;
  } catch {
    return false;
  }
}

async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const headers: Record<string, string> = { "Content-Type": "application/json", ...options?.headers as Record<string, string> };

  // Attach auth token if available
  const token = localStorage.getItem('apihop_access_token');
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  const res = await fetch(path, {
    ...options,
    headers,
  });

  // Handle 401 - try refresh
  if (res.status === 401 && !path.includes('/auth/')) {
    const refreshed = await tryRefreshToken();
    if (refreshed) {
      // Retry with new token
      const newToken = localStorage.getItem('apihop_access_token');
      if (newToken) {
        headers['Authorization'] = `Bearer ${newToken}`;
      }
      const retryRes = await fetch(path, { ...options, headers });
      if (!retryRes.ok) {
        throw new Error(`Server error: ${retryRes.status} ${await retryRes.text()}`);
      }
      if (retryRes.status === 204) return undefined as unknown as T;
      return retryRes.json();
    }
  }

  if (!res.ok) {
    throw new Error(`Server error: ${res.status} ${await res.text()}`);
  }
  if (res.status === 204) return undefined as unknown as T;
  return res.json();
}

// --- Send Request ---

export async function sendRequest(payload: SendRequestPayload): Promise<SendRequestResponse> {
  if (IS_TAURI) {
    return tauriInvoke<SendRequestResponse>("send_full_request", { payload });
  }
  return apiFetch<SendRequestResponse>("/api/v1/send", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

// --- Collections ---

export async function listCollections(): Promise<Collection[]> {
  if (IS_TAURI) {
    return tauriInvoke<Collection[]>("list_collections");
  }
  return apiFetch<Collection[]>("/api/v1/collections");
}

export async function getCollection(id: string): Promise<Collection> {
  if (IS_TAURI) {
    return tauriInvoke<Collection>("get_collection", { id });
  }
  return apiFetch<Collection>(`/api/v1/collections/${id}`);
}

export async function createCollection(name: string, description?: string, auth?: AuthConfig, workspaceId?: string): Promise<Collection> {
  if (IS_TAURI) {
    return tauriInvoke<Collection>("create_collection", { name, description, auth, workspaceId });
  }
  return apiFetch<Collection>("/api/v1/collections", {
    method: "POST",
    body: JSON.stringify({ name, description, auth, workspace_id: workspaceId }),
  });
}

export async function updateCollection(id: string, name: string, description?: string, auth?: AuthConfig): Promise<Collection> {
  if (IS_TAURI) {
    return tauriInvoke<Collection>("update_collection", { id, name, description, auth });
  }
  return apiFetch<Collection>(`/api/v1/collections/${id}`, {
    method: "PUT",
    body: JSON.stringify({ name, description, auth }),
  });
}

export async function deleteCollection(id: string): Promise<void> {
  if (IS_TAURI) {
    return tauriInvoke<void>("delete_collection", { id });
  }
  return apiFetch<void>(`/api/v1/collections/${id}`, { method: "DELETE" });
}

// --- Folders ---

export async function listFolders(collectionId: string, parentFolderId?: string): Promise<Folder[]> {
  if (IS_TAURI) {
    return tauriInvoke<Folder[]>("list_folders", { collectionId, parentFolderId });
  }
  const params = new URLSearchParams();
  if (parentFolderId) params.set("parent_folder_id", parentFolderId);
  return apiFetch<Folder[]>(`/api/v1/collections/${collectionId}/folders?${params}`);
}

export async function createFolder(collectionId: string, name: string, parentFolderId?: string): Promise<Folder> {
  if (IS_TAURI) {
    return tauriInvoke<Folder>("create_folder", { collectionId, parentFolderId, name });
  }
  return apiFetch<Folder>(`/api/v1/collections/${collectionId}/folders`, {
    method: "POST",
    body: JSON.stringify({ name, parent_folder_id: parentFolderId }),
  });
}

export async function updateFolder(collectionId: string, folderId: string, name: string): Promise<Folder> {
  if (IS_TAURI) {
    return tauriInvoke<Folder>("update_folder", { id: folderId, name });
  }
  return apiFetch<Folder>(`/api/v1/collections/${collectionId}/folders/${folderId}`, {
    method: "PUT",
    body: JSON.stringify({ name }),
  });
}

export async function deleteFolder(collectionId: string, folderId: string): Promise<void> {
  if (IS_TAURI) {
    return tauriInvoke<void>("delete_folder", { id: folderId });
  }
  return apiFetch<void>(`/api/v1/collections/${collectionId}/folders/${folderId}`, { method: "DELETE" });
}

// --- Saved Requests ---

export async function listRequests(collectionId: string, folderId?: string): Promise<SavedRequest[]> {
  if (IS_TAURI) {
    return tauriInvoke<SavedRequest[]>("list_requests", { collectionId, folderId });
  }
  const params = new URLSearchParams({ collection_id: collectionId });
  if (folderId) params.set("folder_id", folderId);
  return apiFetch<SavedRequest[]>(`/api/v1/requests?${params}`);
}

export async function getRequest(id: string): Promise<SavedRequest> {
  if (IS_TAURI) {
    return tauriInvoke<SavedRequest>("get_request", { id });
  }
  return apiFetch<SavedRequest>(`/api/v1/requests/${id}`);
}

export async function createRequest(req: SavedRequest): Promise<SavedRequest> {
  if (IS_TAURI) {
    return tauriInvoke<SavedRequest>("create_request", { req });
  }
  return apiFetch<SavedRequest>("/api/v1/requests", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export async function updateRequest(req: SavedRequest): Promise<SavedRequest> {
  if (IS_TAURI) {
    return tauriInvoke<SavedRequest>("update_request", { req });
  }
  return apiFetch<SavedRequest>(`/api/v1/requests/${req.id}`, {
    method: "PUT",
    body: JSON.stringify(req),
  });
}

export async function deleteRequest(id: string): Promise<void> {
  if (IS_TAURI) {
    return tauriInvoke<void>("delete_request", { id });
  }
  return apiFetch<void>(`/api/v1/requests/${id}`, { method: "DELETE" });
}

// --- History ---

export async function listHistory(limit?: number, offset?: number): Promise<HistoryEntry[]> {
  if (IS_TAURI) {
    return tauriInvoke<HistoryEntry[]>("list_history", { limit, offset });
  }
  const params = new URLSearchParams();
  if (limit !== undefined) params.set("limit", String(limit));
  if (offset !== undefined) params.set("offset", String(offset));
  return apiFetch<HistoryEntry[]>(`/api/v1/history?${params}`);
}

export async function createHistoryEntry(entry: HistoryEntry): Promise<HistoryEntry> {
  if (IS_TAURI) {
    return tauriInvoke<HistoryEntry>("create_history_entry", { entry });
  }
  return apiFetch<HistoryEntry>("/api/v1/history", {
    method: "POST",
    body: JSON.stringify(entry),
  });
}

export async function deleteHistoryEntry(id: string): Promise<void> {
  if (IS_TAURI) {
    return tauriInvoke<void>("delete_history_entry", { id });
  }
  return apiFetch<void>(`/api/v1/history/${id}`, { method: "DELETE" });
}

export async function clearHistory(): Promise<void> {
  if (IS_TAURI) {
    return tauriInvoke<void>("clear_history");
  }
  return apiFetch<void>("/api/v1/history", { method: "DELETE" });
}

// --- Environments ---

export async function listEnvironments(): Promise<Environment[]> {
  if (IS_TAURI) {
    return tauriInvoke<Environment[]>("list_environments");
  }
  return apiFetch<Environment[]>("/api/v1/environments");
}

export async function getEnvironment(id: string): Promise<Environment> {
  if (IS_TAURI) {
    return tauriInvoke<Environment>("get_environment", { id });
  }
  return apiFetch<Environment>(`/api/v1/environments/${id}`);
}

export async function createEnvironment(name: string, workspaceId?: string): Promise<Environment> {
  if (IS_TAURI) {
    return tauriInvoke<Environment>("create_environment", { name, workspaceId });
  }
  return apiFetch<Environment>("/api/v1/environments", {
    method: "POST",
    body: JSON.stringify({ name, workspace_id: workspaceId }),
  });
}

export async function updateEnvironment(id: string, name: string): Promise<Environment> {
  if (IS_TAURI) {
    return tauriInvoke<Environment>("update_environment", { id, name });
  }
  return apiFetch<Environment>(`/api/v1/environments/${id}`, {
    method: "PUT",
    body: JSON.stringify({ name }),
  });
}

export async function deleteEnvironment(id: string): Promise<void> {
  if (IS_TAURI) {
    return tauriInvoke<void>("delete_environment", { id });
  }
  return apiFetch<void>(`/api/v1/environments/${id}`, { method: "DELETE" });
}

// --- Variables ---

export async function listVariables(environmentId?: string): Promise<Variable[]> {
  if (IS_TAURI) {
    return tauriInvoke<Variable[]>("list_variables", { environmentId });
  }
  if (environmentId) {
    return apiFetch<Variable[]>(`/api/v1/environments/${environmentId}/variables`);
  }
  return apiFetch<Variable[]>("/api/v1/variables");
}

export async function setVariable(variable: Variable): Promise<Variable> {
  if (IS_TAURI) {
    return tauriInvoke<Variable>("set_variable", { variable });
  }
  return apiFetch<Variable>("/api/v1/variables", {
    method: "POST",
    body: JSON.stringify(variable),
  });
}

export async function deleteVariable(id: string): Promise<void> {
  if (IS_TAURI) {
    return tauriInvoke<void>("delete_variable", { id });
  }
  return apiFetch<void>(`/api/v1/variables/${id}`, { method: "DELETE" });
}

// --- Import/Export ---

export async function importPostman(data: string): Promise<Collection> {
  if (IS_TAURI) {
    return tauriInvoke<Collection>("import_postman_collection", { data });
  }
  return apiFetch<Collection>("/api/v1/import/postman", {
    method: "POST",
    body: JSON.stringify({ data }),
  });
}

export async function importOpenApi(data: string): Promise<Collection> {
  if (IS_TAURI) {
    return tauriInvoke<Collection>("import_openapi_spec", { data });
  }
  return apiFetch<Collection>("/api/v1/import/openapi", {
    method: "POST",
    body: JSON.stringify({ data }),
  });
}

export async function importCurl(data: string): Promise<CurlImportResult> {
  if (IS_TAURI) {
    return tauriInvoke<CurlImportResult>("import_curl_command", { data });
  }
  return apiFetch<CurlImportResult>("/api/v1/import/curl", {
    method: "POST",
    body: JSON.stringify({ data }),
  });
}

export async function exportCollectionApihop(id: string): Promise<object> {
  if (IS_TAURI) {
    return tauriInvoke<object>("export_collection_apihop", { id });
  }
  return apiFetch<object>(`/api/v1/export/collection/${id}/apihop`);
}

export async function exportCollectionPostman(id: string): Promise<object> {
  if (IS_TAURI) {
    return tauriInvoke<object>("export_collection_postman", { id });
  }
  return apiFetch<object>(`/api/v1/export/collection/${id}/postman`);
}

export async function exportRequestCurl(id: string): Promise<string> {
  if (IS_TAURI) {
    return tauriInvoke<string>("export_request_curl", { id });
  }
  return apiFetch<string>(`/api/v1/export/request/${id}/curl`);
}

// --- WebSocket ---

export async function wsConnect(payload: WsConnectPayload): Promise<WsConnectResult> {
  if (IS_TAURI) {
    return tauriInvoke<WsConnectResult>("ws_connect", {
      url: payload.url,
      headers: payload.headers,
      auth: payload.auth,
      environmentId: payload.environment_id,
    });
  }
  return apiFetch<WsConnectResult>("/api/v1/ws/connect", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function wsSend(connectionId: string, payload: string, isBinary: boolean, environmentId?: string): Promise<WsMessage> {
  if (IS_TAURI) {
    return tauriInvoke<WsMessage>("ws_send", { id: connectionId, payload, isBinary, environmentId });
  }
  return apiFetch<WsMessage>(`/api/v1/ws/${connectionId}/send`, {
    method: "POST",
    body: JSON.stringify({ payload, is_binary: isBinary, environment_id: environmentId }),
  });
}

export async function wsDisconnect(connectionId: string): Promise<void> {
  if (IS_TAURI) {
    return tauriInvoke<void>("ws_disconnect", { id: connectionId });
  }
  return apiFetch<void>(`/api/v1/ws/${connectionId}/disconnect`, {
    method: "POST",
  });
}

export async function wsStatus(connectionId: string): Promise<WsStatus> {
  if (IS_TAURI) {
    return tauriInvoke<WsStatus>("ws_status", { id: connectionId });
  }
  return apiFetch<WsStatus>(`/api/v1/ws/${connectionId}/status`);
}

// --- GraphQL ---

export async function graphqlIntrospect(payload: GraphQLIntrospectPayload): Promise<GraphQLSchema> {
  if (IS_TAURI) {
    return tauriInvoke<GraphQLSchema>("graphql_introspect", {
      url: payload.url,
      headers: payload.headers,
      auth: payload.auth,
      environmentId: payload.environment_id,
    });
  }
  return apiFetch<GraphQLSchema>("/api/v1/graphql/introspect", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export function wsSubscribe(connectionId: string, onMessage: (msg: WsMessage) => void): () => void {
  if (IS_TAURI) {
    let unlisten: (() => void) | null = null;
    import("@tauri-apps/api/event").then(({ listen }) => {
      listen<WsMessage>(`ws-message-${connectionId}`, (event) => {
        onMessage(event.payload);
      }).then((fn) => {
        unlisten = fn;
      });
    });
    // Also invoke ws_subscribe to tell backend to start emitting
    tauriInvoke("ws_subscribe", { id: connectionId }).catch(() => {});
    return () => {
      if (unlisten) unlisten();
    };
  }

  // Browser: use SSE
  const es = new EventSource(`/api/v1/ws/${connectionId}/messages`);
  es.onmessage = (event) => {
    try {
      const msg: WsMessage = JSON.parse(event.data);
      onMessage(msg);
    } catch {
      // ignore parse errors
    }
  };
  return () => {
    es.close();
  };
}

// --- Auth ---

export async function fetchServerInfo(): Promise<ServerInfo> {
  return apiFetch<ServerInfo>("/api/v1/server/info");
}

export async function loginUser(email: string, password: string): Promise<AuthTokens> {
  const tokens = await apiFetch<AuthTokens>("/api/v1/auth/login", {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });
  localStorage.setItem('apihop_access_token', tokens.access_token);
  localStorage.setItem('apihop_refresh_token', tokens.refresh_token);
  return tokens;
}

export async function registerUser(email: string, password: string, displayName?: string): Promise<AuthTokens> {
  const tokens = await apiFetch<AuthTokens>("/api/v1/auth/register", {
    method: "POST",
    body: JSON.stringify({ email, password, display_name: displayName }),
  });
  localStorage.setItem('apihop_access_token', tokens.access_token);
  localStorage.setItem('apihop_refresh_token', tokens.refresh_token);
  return tokens;
}

export async function logoutUser(): Promise<void> {
  const refreshToken = localStorage.getItem('apihop_refresh_token');
  if (refreshToken) {
    try {
      await apiFetch<void>("/api/v1/auth/logout", {
        method: "POST",
        body: JSON.stringify({ refresh_token: refreshToken }),
      });
    } catch {
      // ignore logout errors
    }
  }
  localStorage.removeItem('apihop_access_token');
  localStorage.removeItem('apihop_refresh_token');
}

export async function getMe(): Promise<AuthUser> {
  return apiFetch<AuthUser>("/api/v1/auth/me");
}

// --- Connections ---

export async function listConnections(): Promise<ServerConnection[]> {
  if (IS_TAURI) {
    return tauriInvoke<ServerConnection[]>("list_connections");
  }
  return apiFetch<ServerConnection[]>("/api/v1/connections");
}

export async function addConnection(serverUrl: string, displayName: string): Promise<ServerConnection> {
  if (IS_TAURI) {
    return tauriInvoke<ServerConnection>("add_connection", { serverUrl, displayName });
  }
  return apiFetch<ServerConnection>("/api/v1/connections", {
    method: "POST",
    body: JSON.stringify({ server_url: serverUrl, display_name: displayName }),
  });
}

export async function removeConnection(id: string): Promise<void> {
  if (IS_TAURI) {
    return tauriInvoke<void>("remove_connection", { id });
  }
  return apiFetch<void>(`/api/v1/connections/${id}`, { method: "DELETE" });
}

export async function testConnection(serverUrl: string): Promise<ServerInfo> {
  if (IS_TAURI) {
    return tauriInvoke<ServerInfo>("test_connection", { serverUrl });
  }
  // In web mode, proxy through server
  return apiFetch<ServerInfo>(`/api/v1/connections/test?url=${encodeURIComponent(serverUrl)}`);
}

export async function connectionLogin(id: string, email: string, password: string): Promise<unknown> {
  if (IS_TAURI) {
    return tauriInvoke<unknown>("connection_login", { id, email, password });
  }
  return apiFetch<unknown>(`/api/v1/connections/${id}/login`, {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });
}

// --- Connection Proxy ---

export async function connectionProxy(connectionId: string, method: string, path: string, body?: unknown): Promise<unknown> {
  if (IS_TAURI) {
    return tauriInvoke("connection_proxy", { connectionId, method, path, body: body ?? null });
  }
  return apiFetch(`/api/v1/connections/${connectionId}/proxy`, {
    method: "POST",
    body: JSON.stringify({ method, path, body }),
  });
}

// --- Workspaces ---

export async function listWorkspaces(): Promise<Workspace[]> {
  return apiFetch<Workspace[]>("/api/v1/workspaces");
}

export async function createWorkspace(name: string, description?: string): Promise<Workspace> {
  return apiFetch<Workspace>("/api/v1/workspaces", {
    method: "POST",
    body: JSON.stringify({ name, description }),
  });
}

export async function getWorkspace(id: string): Promise<Workspace> {
  return apiFetch<Workspace>(`/api/v1/workspaces/${id}`);
}

export async function updateWorkspace(id: string, name: string, description?: string): Promise<Workspace> {
  return apiFetch<Workspace>(`/api/v1/workspaces/${id}`, {
    method: "PUT",
    body: JSON.stringify({ name, description }),
  });
}

export async function deleteWorkspace(id: string): Promise<void> {
  return apiFetch<void>(`/api/v1/workspaces/${id}`, { method: "DELETE" });
}

export async function listWorkspaceCollections(workspaceId: string): Promise<Collection[]> {
  return apiFetch<Collection[]>(`/api/v1/workspaces/${workspaceId}/collections`);
}

export async function listWorkspaceEnvironments(workspaceId: string): Promise<Environment[]> {
  return apiFetch<Environment[]>(`/api/v1/workspaces/${workspaceId}/environments`);
}

// --- Workspace Members ---

export async function listWorkspaceMembers(workspaceId: string): Promise<WorkspaceMember[]> {
  return apiFetch<WorkspaceMember[]>(`/api/v1/workspaces/${workspaceId}/members`);
}

export async function addWorkspaceMember(workspaceId: string, userId: string, role: string): Promise<WorkspaceMember> {
  return apiFetch<WorkspaceMember>(`/api/v1/workspaces/${workspaceId}/members`, {
    method: "POST",
    body: JSON.stringify({ user_id: userId, role }),
  });
}

export async function updateWorkspaceMemberRole(workspaceId: string, userId: string, role: string): Promise<WorkspaceMember> {
  return apiFetch<WorkspaceMember>(`/api/v1/workspaces/${workspaceId}/members/${userId}`, {
    method: "PUT",
    body: JSON.stringify({ role }),
  });
}

export async function removeWorkspaceMember(workspaceId: string, userId: string): Promise<void> {
  return apiFetch<void>(`/api/v1/workspaces/${workspaceId}/members/${userId}`, { method: "DELETE" });
}

export async function leaveWorkspace(workspaceId: string): Promise<void> {
  return apiFetch<void>(`/api/v1/workspaces/${workspaceId}/leave`, { method: "POST" });
}

// --- Workspace Invites ---

export async function createWorkspaceInvite(workspaceId: string, email: string, role: string): Promise<WorkspaceInvite> {
  return apiFetch<WorkspaceInvite>(`/api/v1/workspaces/${workspaceId}/invites`, {
    method: "POST",
    body: JSON.stringify({ email, role }),
  });
}

export async function listWorkspaceInvites(workspaceId: string): Promise<WorkspaceInvite[]> {
  return apiFetch<WorkspaceInvite[]>(`/api/v1/workspaces/${workspaceId}/invites`);
}

export async function revokeWorkspaceInvite(workspaceId: string, inviteId: string): Promise<void> {
  return apiFetch<void>(`/api/v1/workspaces/${workspaceId}/invites/${inviteId}`, { method: "DELETE" });
}

export async function acceptWorkspaceInvite(token: string): Promise<Workspace> {
  return apiFetch<Workspace>("/api/v1/workspaces/accept-invite", {
    method: "POST",
    body: JSON.stringify({ token }),
  });
}
