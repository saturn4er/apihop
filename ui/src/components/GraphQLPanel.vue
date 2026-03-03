<script setup lang="ts">
import { ref, watch, computed } from "vue";
import CodeEditor from "./CodeEditor.vue";
import ResponsePanel from "./ResponsePanel.vue";
import AuthEditor from "./AuthEditor.vue";
import KeyValueTable from "./KeyValueTable.vue";
import ExtractionRulesEditor from "./ExtractionRulesEditor.vue";
import GraphQLSchemaExplorer from "./GraphQLSchemaExplorer.vue";
import GraphQLQueryBuilder from "./GraphQLQueryBuilder.vue";
import { formatGraphql, formatJson } from "@/utils/format";
import type { KeyValueRow, ExtractionRule, ExtractedVariable } from "@/api/types";
import {
  sendRequest,
  graphqlIntrospect,
  listVariables,
  type ApiResponse,
  type SavedRequest,
  type AuthConfig,
  type SendRequestPayload,
  type ScriptExecutionResult,
  type GraphQLSchema,
} from "@/api/client";

const props = defineProps<{
  savedRequest?: SavedRequest | null;
  collectionAuth?: AuthConfig;
  environmentId?: string;
}>();

const emit = defineEmits<{
  save: [];
  "history-recorded": [];
}>();

const url = ref("");
const query = ref("{\n  \n}");
const variables = ref("");
const operationName = ref("");
const activeTab = ref<"query" | "variables" | "headers" | "auth" | "schema" | "extract">("query");
const queryMode = ref<"text" | "builder">("text");
const auth = ref<AuthConfig>({ type: "inherit" });
const headers = ref<KeyValueRow[]>([{ key: "", value: "", enabled: true }]);

const loading = ref(false);
const response = ref<ApiResponse | null>(null);
const error = ref<string | null>(null);
const unresolvedVars = ref<string[]>([]);
const scriptResult = ref<ScriptExecutionResult | null>(null);

// Schema introspection state
const schema = ref<GraphQLSchema | null>(null);
const schemaLoading = ref(false);
const schemaError = ref<string | null>(null);
const extractionRules = ref<ExtractionRule[]>([]);
const extractedVariables = ref<ExtractedVariable[]>([]);

// Load environment variable names for autocomplete
const variableNames = ref<string[]>([]);
async function loadVariableNames() {
  try {
    const [globals, envVars] = await Promise.all([
      listVariables(undefined),
      props.environmentId ? listVariables(props.environmentId) : Promise.resolve([]),
    ]);
    variableNames.value = [...new Set([...globals, ...envVars].map((v) => v.key))];
  } catch {
    variableNames.value = [];
  }
}
loadVariableNames();
watch(() => props.environmentId, loadVariableNames);

// GraphQL response parsing
const graphqlErrors = computed(() => {
  if (!response.value) return null;
  try {
    const parsed = JSON.parse(response.value.body);
    if (parsed.errors && Array.isArray(parsed.errors) && parsed.errors.length > 0) {
      return parsed.errors;
    }
  } catch {
    // not JSON
  }
  return null;
});

// Watch savedRequest prop to populate form
watch(
  () => props.savedRequest,
  (req) => {
    if (!req) return;
    url.value = req.url;
    query.value = req.graphql_query || "{\n  \n}";
    variables.value = req.graphql_variables || "";
    operationName.value = req.graphql_operation_name || "";
    response.value = null;
    error.value = null;

    const hRows: KeyValueRow[] = Object.entries(req.headers).map(([key, value]) => ({
      key,
      value,
      enabled: true,
    }));
    hRows.push({ key: "", value: "", enabled: true });
    headers.value = hRows;

    const savedAuth = req.auth || { type: "none" };
    auth.value = savedAuth.type === "none" ? { type: "inherit" } : savedAuth;

    // Extraction rules
    if (req.extraction_rules) {
      try {
        extractionRules.value = JSON.parse(req.extraction_rules);
      } catch {
        extractionRules.value = [];
      }
    } else {
      extractionRules.value = [];
    }

    // Auto-introspect if we have a URL and no schema yet
    if (req.url && !schema.value) {
      introspect();
    }
  },
  { immediate: true }
);

function getFormData() {
  const reqHeaders: Record<string, string> = {};
  headers.value.forEach((h) => {
    if (h.enabled && h.key) reqHeaders[h.key] = h.value;
  });
  const storedAuth: AuthConfig = auth.value.type === "inherit" ? { type: "none" } : auth.value;

  return {
    url: url.value,
    headers: reqHeaders,
    auth: storedAuth,
    graphql_query: query.value,
    graphql_variables: variables.value || undefined,
    graphql_operation_name: operationName.value || undefined,
    extraction_rules: extractionRules.value.length > 0 ? JSON.stringify(extractionRules.value) : undefined,
  };
}

defineExpose({ getFormData, send, focusUrl });

function focusUrl() {
  // Focus the URL input
}

async function send() {
  if (!url.value.trim()) return;
  loading.value = true;
  response.value = null;
  error.value = null;
  unresolvedVars.value = [];
  scriptResult.value = null;
  extractedVariables.value = [];

  const reqHeaders: Record<string, string> = {};
  headers.value.forEach((h) => {
    if (h.enabled && h.key) reqHeaders[h.key] = h.value;
  });

  let effectiveAuth: AuthConfig;
  if (auth.value.type === "inherit") {
    effectiveAuth =
      props.collectionAuth && props.collectionAuth.type !== "none"
        ? props.collectionAuth
        : { type: "none" };
  } else {
    effectiveAuth = auth.value;
  }

  const payload: SendRequestPayload = {
    method: "POST",
    url: url.value,
    headers: reqHeaders,
    params: [],
    auth: effectiveAuth,
    environment_id: props.environmentId,
    collection_id: props.savedRequest?.collection_id,
    request_type: "graphql",
    graphql_query: query.value,
    graphql_variables: variables.value || undefined,
    graphql_operation_name: operationName.value || undefined,
    extraction_rules: extractionRules.value.length > 0 ? extractionRules.value : undefined,
  };

  try {
    const result = await sendRequest(payload);
    response.value = result.response;
    unresolvedVars.value = result.unresolved_variables;
    scriptResult.value = result.script_result || null;
    extractedVariables.value = result.extracted_variables || [];
    emit("history-recorded");
  } catch (e: any) {
    error.value = e?.message || String(e);
  } finally {
    loading.value = false;
  }
}

async function introspect() {
  if (!url.value.trim()) return;
  schemaLoading.value = true;
  schemaError.value = null;

  const reqHeaders: Record<string, string> = {};
  headers.value.forEach((h) => {
    if (h.enabled && h.key) reqHeaders[h.key] = h.value;
  });

  let effectiveAuth: AuthConfig | undefined;
  if (auth.value.type === "inherit") {
    effectiveAuth =
      props.collectionAuth && props.collectionAuth.type !== "none"
        ? props.collectionAuth
        : undefined;
  } else if (auth.value.type !== "none") {
    effectiveAuth = auth.value;
  }

  try {
    schema.value = await graphqlIntrospect({
      url: url.value,
      headers: reqHeaders,
      auth: effectiveAuth,
      environment_id: props.environmentId,
    });
    activeTab.value = "schema";
  } catch (e: any) {
    schemaError.value = e?.message || String(e);
  } finally {
    schemaLoading.value = false;
  }
}

function onInsertFromSchema(text: string) {
  query.value = text;
  activeTab.value = "query";
}
</script>

<template>
  <div class="graphql-panel">
    <div class="request-section">
      <!-- URL bar -->
      <div class="url-bar">
        <span class="method-badge">GQL</span>
        <input
          v-model="url"
          class="url-input"
          placeholder="https://api.example.com/graphql"
          @keydown.enter.meta="send"
          @keydown.enter.ctrl="send"
        />
        <button class="introspect-btn" :disabled="schemaLoading" @click="introspect">
          {{ schemaLoading ? "Loading..." : "Introspect" }}
        </button>
        <button class="send-btn" :disabled="loading" @click="send">
          {{ loading ? "Sending..." : "Send" }}
        </button>
      </div>

      <div v-if="unresolvedVars.length > 0" class="unresolved-warning">
        Unresolved variables: {{ unresolvedVars.map((v) => "\u007B\u007B" + v + "\u007D\u007D").join(", ") }}
      </div>

      <div v-if="schemaError" class="schema-error">
        Schema introspection failed: {{ schemaError }}
      </div>

      <!-- Tabs -->
      <div class="tabs">
        <button class="tab" :class="{ active: activeTab === 'query' }" @click="activeTab = 'query'">
          Query
        </button>
        <button
          class="tab"
          :class="{ active: activeTab === 'variables' }"
          @click="activeTab = 'variables'"
        >
          Variables
        </button>
        <button
          class="tab"
          :class="{ active: activeTab === 'headers' }"
          @click="activeTab = 'headers'"
        >
          Headers
        </button>
        <button class="tab" :class="{ active: activeTab === 'auth' }" @click="activeTab = 'auth'">
          Auth
        </button>
        <button
          class="tab"
          :class="{ active: activeTab === 'schema' }"
          @click="activeTab = 'schema'"
        >
          Schema
          <span v-if="schema" class="tab-badge">loaded</span>
        </button>
        <button
          class="tab"
          :class="{ active: activeTab === 'extract' }"
          @click="activeTab = 'extract'"
        >
          Extract
        </button>
      </div>

      <div class="tab-content">
        <!-- Query editor -->
        <div v-if="activeTab === 'query'" class="query-editor-wrapper">
          <div class="query-toolbar">
            <div class="query-mode-toggle">
              <button
                class="mode-btn"
                :class="{ active: queryMode === 'text' }"
                @click="queryMode = 'text'"
              >
                Text
              </button>
              <button
                class="mode-btn"
                :class="{ active: queryMode === 'builder' }"
                :disabled="!schema"
                :title="!schema ? 'Introspect schema first' : 'Visual query builder'"
                @click="queryMode = 'builder'"
              >
                Builder
              </button>
            </div>
            <button v-if="queryMode === 'text'" class="format-btn" @click="query = formatGraphql(query)">Format</button>
          </div>
          <div v-if="queryMode === 'text'" class="query-text-editor">
            <CodeEditor v-model:modelValue="query" language="text" placeholder="Enter GraphQL query..." :variable-names="variableNames" />
          </div>
          <div v-else class="query-builder-wrapper">
            <GraphQLQueryBuilder
              v-if="schema"
              :schema="schema"
              :model-value="query"
              @update:model-value="query = $event"
            />
          </div>
          <div class="operation-name">
            <label>Operation Name</label>
            <input v-model="operationName" placeholder="optional" />
          </div>
        </div>

        <!-- Variables editor -->
        <div v-else-if="activeTab === 'variables'" class="variables-editor">
          <div class="editor-toolbar">
            <button class="format-btn" @click="variables = formatJson(variables)">Format</button>
          </div>
          <CodeEditor
            v-model:modelValue="variables"
            language="json"
            placeholder='{ "key": "value" }'
            :variable-names="variableNames"
          />
        </div>

        <!-- Headers -->
        <KeyValueTable
          v-else-if="activeTab === 'headers'"
          v-model="headers"
          key-placeholder="Header"
          value-placeholder="Value"
          :variable-names="variableNames"
        />

        <!-- Auth -->
        <AuthEditor
          v-else-if="activeTab === 'auth'"
          v-model="auth"
          :inherited-auth="collectionAuth"
        />

        <!-- Extract -->
        <ExtractionRulesEditor
          v-else-if="activeTab === 'extract'"
          :model-value="extractionRules"
          :extracted-variables="extractedVariables.length > 0 ? extractedVariables : undefined"
          @update:model-value="extractionRules = $event"
        />

        <!-- Schema explorer -->
        <div v-else-if="activeTab === 'schema'" class="schema-tab">
          <GraphQLSchemaExplorer
            v-if="schema"
            :schema="schema"
            @insert="onInsertFromSchema"
          />
          <div v-else class="response-placeholder">
            Click "Introspect" to load the schema
          </div>
        </div>
      </div>
    </div>

    <ResponsePanel :response="response" :error="error" :loading="loading" :script-result="scriptResult" :extracted-variables="extractedVariables">
      <template #after-body v-if="graphqlErrors">
        <div class="graphql-errors">
          <div class="graphql-errors-header">GraphQL Errors</div>
          <div v-for="(err, i) in graphqlErrors" :key="i" class="graphql-error-item">
            <span class="error-message">{{ err.message }}</span>
            <span v-if="err.path" class="error-path">Path: {{ err.path.join(".") }}</span>
          </div>
        </div>
      </template>
    </ResponsePanel>
  </div>
</template>

<style scoped>
.graphql-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.request-section {
  display: flex;
  flex-direction: column;
  padding: 16px;
  gap: 12px;
  flex: 1;
  min-height: 0;
  overflow: auto;
}

.url-bar {
  display: flex;
  gap: 8px;
  align-items: center;
}

.method-badge {
  background: var(--accent);
  color: #fff;
  font-size: 11px;
  font-weight: 700;
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  flex-shrink: 0;
}

.url-input {
  flex: 1;
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-size: 13px;
  padding: 8px 12px;
  font-family: "SF Mono", "Fira Code", monospace;
  outline: none;
  transition: border-color var(--transition);
}

.url-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-muted);
}

.introspect-btn {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  padding: 8px 14px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition);
  flex-shrink: 0;
}

.introspect-btn:hover:not(:disabled) {
  color: var(--text-primary);
  border-color: var(--accent);
}

.introspect-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.send-btn {
  background: var(--accent);
  border: none;
  color: #fff;
  font-size: 13px;
  font-weight: 600;
  padding: 8px 20px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition);
  flex-shrink: 0;
}

.send-btn:hover:not(:disabled) {
  filter: brightness(1.1);
}

.send-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.unresolved-warning {
  background: rgba(255, 152, 0, 0.12);
  border: 1px solid var(--warning);
  border-radius: var(--radius-md);
  color: var(--warning);
  font-size: 12px;
  padding: 6px 10px;
}

.schema-error {
  background: rgba(244, 67, 54, 0.12);
  border: 1px solid var(--error);
  border-radius: var(--radius-md);
  color: var(--error);
  font-size: 12px;
  padding: 6px 10px;
}

.tabs {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--border-color);
}

.tab {
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  padding: 8px 20px;
  color: var(--text-secondary);
  font-size: 13px;
  cursor: pointer;
  transition: all var(--transition);
  border-radius: var(--radius-sm) var(--radius-sm) 0 0;
  display: flex;
  align-items: center;
  gap: 6px;
}

.tab:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.tab-badge {
  background: rgba(76, 175, 80, 0.2);
  color: var(--success);
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 8px;
}

.tab-content {
  flex: 1;
  min-height: 0;
}

.query-editor-wrapper {
  display: flex;
  flex-direction: column;
  gap: 8px;
  height: 100%;
}

.query-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
}

.editor-toolbar {
  display: flex;
  justify-content: flex-end;
  padding: 4px 0;
}

.format-btn {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
  font-size: 11px;
  font-weight: 600;
  padding: 3px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition);
}

.format-btn:hover {
  color: var(--text-primary);
  border-color: var(--accent);
}

.query-mode-toggle {
  display: flex;
  gap: 0;
  background: var(--bg-tertiary);
  border-radius: var(--radius-sm);
  padding: 2px;
  width: fit-content;
  flex-shrink: 0;
}

.mode-btn {
  background: none;
  border: none;
  padding: 4px 14px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition);
}

.mode-btn:hover:not(:disabled) {
  color: var(--text-primary);
}

.mode-btn.active {
  background: var(--bg-primary);
  color: var(--accent);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.mode-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.query-text-editor {
  flex: 1;
  min-height: 0;
}

.query-builder-wrapper {
  flex: 1;
  min-height: 0;
  overflow: auto;
}

.operation-name {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
}

.operation-name label {
  font-size: 12px;
  color: var(--text-secondary);
  flex-shrink: 0;
}

.operation-name input {
  flex: 1;
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 12px;
  padding: 4px 8px;
  outline: none;
}

.operation-name input:focus {
  border-color: var(--accent);
}

.variables-editor {
  height: 100%;
}

.schema-tab {
  height: 100%;
  overflow: auto;
}

.response-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 120px;
  color: var(--text-muted);
  font-size: 13px;
}

.graphql-errors {
  padding: 12px 16px;
  border-top: 1px solid var(--border-color);
}

.graphql-errors-header {
  font-size: 12px;
  font-weight: 700;
  color: var(--error);
  margin-bottom: 8px;
}

.graphql-error-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 6px 8px;
  background: rgba(244, 67, 54, 0.08);
  border-radius: var(--radius-sm);
  margin-bottom: 4px;
}

.error-message {
  font-size: 12px;
  color: var(--error);
}

.error-path {
  font-size: 11px;
  color: var(--text-muted);
  font-family: "SF Mono", "Fira Code", monospace;
}
</style>
