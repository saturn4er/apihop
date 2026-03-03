export interface ApiRequest {
  method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";
  url: string;
  headers: Record<string, string>;
  body?: string;
}

export interface ApiResponse {
  status: number;
  headers: Record<string, string>;
  body: string;
  duration_ms: number;
  content_type?: string;
  size_bytes?: number;
}

export interface Environment {
  id: string;
  name: string;
  workspace_id?: string;
  created_at: string;
  updated_at: string;
}

export interface Variable {
  id: string;
  environment_id?: string;
  key: string;
  value: string;
  is_secret: boolean;
}

export type ApiKeyLocation = "header" | "query_param";

export type AuthConfig =
  | { type: "none" }
  | { type: "inherit" }
  | { type: "basic"; username: string; password: string }
  | { type: "bearer"; token: string }
  | { type: "api_key"; key: string; value: string; add_to: ApiKeyLocation }
  | { type: "oauth2_client_credentials"; token_url: string; client_id: string; client_secret: string; scope?: string };

/** Frontend-only auth config that includes the "inherit" option */
export type FrontendAuthConfig = AuthConfig;

export interface ConsoleEntry {
  level: string;
  message: string;
}

export interface TestResult {
  name: string;
  passed: boolean;
  error?: string;
}

export interface ScriptExecutionResult {
  pre_request_console: ConsoleEntry[];
  pre_request_error?: string;
  test_results: TestResult[];
  test_console: ConsoleEntry[];
  test_error?: string;
}

export interface SendRequestPayload {
  method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";
  url: string;
  headers: Record<string, string>;
  body?: string;
  params: KeyValueParam[];
  auth: AuthConfig;
  environment_id?: string;
  pre_request_script?: string;
  test_script?: string;
  collection_id?: string;
  request_type?: "http" | "graphql";
  graphql_query?: string;
  graphql_variables?: string;
  graphql_operation_name?: string;
  extraction_rules?: ExtractionRule[];
}

export interface SendRequestResponse {
  response: ApiResponse;
  unresolved_variables: string[];
  history_id: string;
  script_result?: ScriptExecutionResult;
  extracted_variables?: ExtractedVariable[];
}

export interface Collection {
  id: string;
  name: string;
  description?: string;
  auth?: AuthConfig;
  pre_request_script?: string;
  test_script?: string;
  workspace_id?: string;
  created_at: string;
  updated_at: string;
}

export interface Folder {
  id: string;
  collection_id: string;
  parent_folder_id?: string;
  name: string;
  sort_order: number;
}

export interface KeyValueParam {
  key: string;
  value: string;
  enabled: boolean;
}

export interface SavedRequest {
  id: string;
  collection_id: string;
  folder_id?: string;
  name: string;
  method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";
  url: string;
  headers: Record<string, string>;
  body?: string;
  params: KeyValueParam[];
  auth?: AuthConfig;
  pre_request_script?: string;
  test_script?: string;
  sort_order: number;
  created_at: string;
  updated_at: string;
  request_type?: string;
  graphql_query?: string;
  graphql_variables?: string;
  graphql_operation_name?: string;
  extraction_rules?: string | null;
}

// --- Extraction Types ---

export type ExtractionSource =
  | { type: 'json_path'; path: string }
  | { type: 'header'; name: string }
  | { type: 'status' }
  | { type: 'response_body' };

export interface ExtractionRule {
  source: ExtractionSource;
  target_variable: string;
}

export interface ExtractedVariable {
  variable_name: string;
  value: string | null;
  error: string | null;
}

// --- WebSocket Types ---

export type WsDirection = "sent" | "received" | "close";
export type WsStatus = "connected" | "connecting" | "disconnected";

export interface WsMessage {
  id: string;
  direction: WsDirection;
  payload: string;
  is_binary: boolean;
  timestamp_ms: number;
}

export interface WsConnectPayload {
  url: string;
  headers: Record<string, string>;
  auth?: AuthConfig;
  environment_id?: string;
}

export interface WsConnectResult {
  connection_id: string;
  unresolved_variables: string[];
}

export interface WsSessionSummary {
  id: string;
  url: string;
  status: WsStatus;
  connected_at: string;
}

export interface HistoryEntry {
  id: string;
  method: string;
  url: string;
  request_headers: string;
  request_body?: string;
  response_status: number;
  response_headers: string;
  response_body: string;
  duration_ms: number;
  timestamp: string;
}

export interface CurlImportResult {
  name: string;
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: string;
  params: KeyValueParam[];
  auth: AuthConfig;
}

// --- GraphQL Schema Types ---

export interface GraphQLTypeRef {
  kind: string;
  name?: string;
  of_type?: GraphQLTypeRef;
}

export interface GraphQLInputValue {
  name: string;
  description?: string;
  type: GraphQLTypeRef;
  default_value?: string;
}

export interface GraphQLField {
  name: string;
  description?: string;
  args: GraphQLInputValue[];
  type: GraphQLTypeRef;
  is_deprecated: boolean;
  deprecation_reason?: string;
}

export interface GraphQLEnumValue {
  name: string;
  description?: string;
  is_deprecated: boolean;
  deprecation_reason?: string;
}

export interface GraphQLType {
  kind: string;
  name?: string;
  description?: string;
  fields?: GraphQLField[];
  input_fields?: GraphQLInputValue[];
  interfaces?: GraphQLTypeRef[];
  enum_values?: GraphQLEnumValue[];
  possible_types?: GraphQLTypeRef[];
}

export interface GraphQLDirective {
  name: string;
  description?: string;
  locations: string[];
  args: GraphQLInputValue[];
}

export interface GraphQLSchema {
  query_type?: string;
  mutation_type?: string;
  subscription_type?: string;
  types: GraphQLType[];
  directives: GraphQLDirective[];
}

export interface GraphQLIntrospectPayload {
  url: string;
  headers: Record<string, string>;
  auth?: AuthConfig;
  environment_id?: string;
}

// --- Auth & Connection Types ---

export interface ServerInfo {
  name: string;
  version: string;
  mode: string;
  registration_enabled: boolean;
}

export interface AuthUser {
  id: string;
  email: string;
  display_name?: string;
  created_at: string;
  updated_at: string;
}

export interface AuthTokens {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  user: AuthUser;
}

export interface ServerConnection {
  id: string;
  server_url: string;
  display_name: string;
  user_email?: string;
  user_display_name?: string;
  server_mode: string;
  status: string;
  created_at: string;
  last_used_at?: string;
}

// --- Workspace Types ---

export interface Workspace {
  id: string;
  name: string;
  description?: string;
  owner_id: string;
  is_personal: boolean;
  created_at: string;
  updated_at: string;
}

export interface WorkspaceMember {
  id: string;
  workspace_id: string;
  user_id: string;
  role: 'owner' | 'editor' | 'viewer';
  created_at: string;
}

export interface WorkspaceInvite {
  id: string;
  workspace_id: string;
  email: string;
  role: 'owner' | 'editor' | 'viewer';
  token: string;
  expires_at: string;
  created_at: string;
}

export type DataContext =
  | { type: 'local' }
  | { type: 'remote'; connectionId: string };

export interface SidebarGroup {
  id: string;
  label: string;
  type: 'local' | 'connection';
  connectionId?: string;
  icon?: string;
  workspaces: SidebarWorkspace[];
  /** Flat collections for connections without workspace support */
  remoteCollections?: Collection[];
}

export interface SidebarWorkspace {
  workspace: Workspace;
  context: DataContext;
}

export type AppMode =
  | { kind: 'desktop' }
  | { kind: 'web-personal' }
  | { kind: 'web-organization'; serverInfo: ServerInfo };

// --- UI Types ---

export interface KeyValueRow {
  _id?: number;
  key: string;
  value: string;
  enabled: boolean;
}

export interface MenuItem {
  label: string;
  action: string;
  danger?: boolean;
  separator?: boolean;
}

export interface TreeItem {
  id: string;
  type: "collection" | "folder" | "request";
  label: string;
  collectionId: string;
  parentFolderId?: string;
  method?: string;
  requestType?: string;
  data: Collection | Folder | SavedRequest;
}

export interface Tab {
  id: string;
  name: string;
  method: string;
  isDirty: boolean;
  requestType?: "http" | "websocket" | "graphql";
}

// --- Form State Types ---

export interface HttpFormState {
  type: "http";
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: string;
  params: KeyValueParam[];
  auth?: AuthConfig;
  pre_request_script?: string;
  test_script?: string;
}

export interface WsFormState {
  type: "websocket";
  url: string;
  headers: Record<string, string>;
}

export interface GraphqlFormState {
  type: "graphql";
  url: string;
  headers: Record<string, string>;
  auth?: AuthConfig;
  graphql_query: string;
  graphql_variables?: string;
  graphql_operation_name?: string;
}

export type FormState = HttpFormState | WsFormState | GraphqlFormState;

export interface TabState {
  id: string;
  savedRequest: SavedRequest | null;
  historyEntry: HistoryEntry | null;
  formState: FormState | null;
  isDirty: boolean;
  name: string;
  method: string;
  collectionAuth: AuthConfig | undefined;
  requestType: "http" | "websocket" | "graphql";
}
