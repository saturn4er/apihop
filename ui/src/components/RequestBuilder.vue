<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { watchPausable, watchDebounced } from "@vueuse/core";
import { detectContentType } from "@/utils/http";
import UrlBar from "./UrlBar.vue";
import KeyValueTable from "./KeyValueTable.vue";
import BodyEditor from "./BodyEditor.vue";
import ResponsePanel from "./ResponsePanel.vue";
import AuthEditor from "./AuthEditor.vue";
import ScriptEditor from "./ScriptEditor.vue";
import ExtractionRulesEditor from "./ExtractionRulesEditor.vue";
import type { KeyValueRow, ExtractionRule, ExtractedVariable } from "@/api/types";
import {
  sendRequest,
  listVariables,
  type ApiResponse,
  type SavedRequest,
  type HistoryEntry,
  type KeyValueParam,
  type AuthConfig,
  type SendRequestPayload,
  type CurlImportResult,
  type ScriptExecutionResult,
} from "@/api/client";

const props = defineProps<{
  savedRequest?: SavedRequest | null;
  historyEntry?: HistoryEntry | null;
  collectionAuth?: AuthConfig;
  environmentId?: string;
}>();

const emit = defineEmits<{
  save: [];
  "history-recorded": [];
}>();

const method = ref("GET");
const url = ref("");
const activeTab = ref<"params" | "headers" | "body" | "auth" | "scripts" | "extract">("params");
const auth = ref<AuthConfig>({ type: "inherit" });
const preRequestScript = ref("");
const testScript = ref("");
const scriptResult = ref<ScriptExecutionResult | null>(null);
const extractionRules = ref<ExtractionRule[]>([]);
const extractedVariables = ref<ExtractedVariable[]>([]);

const params = ref<KeyValueRow[]>([{ key: "", value: "", enabled: true }]);
const headers = ref<KeyValueRow[]>([{ key: "", value: "", enabled: true }]);

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

const bodyContentType = ref<"none" | "json" | "xml" | "text" | "form-urlencoded">("none");
const bodyText = ref("");
const formData = ref<KeyValueRow[]>([{ key: "", value: "", enabled: true }]);

const loading = ref(false);
const response = ref<ApiResponse | null>(null);
const error = ref<string | null>(null);

// Encode a param key/value for the URL, but preserve {{variable}} placeholders
function encodeParam(str: string): string {
  return str.replace(/(\{\{[^}]*\}\})|([^{}]+)/g, (_, varMatch, rest) => {
    if (varMatch) return varMatch;
    return encodeURIComponent(rest);
  });
}

// Sync URL query params -> params table (pausable)
const { pause: pauseUrlWatcher, resume: resumeUrlWatcher } = watchPausable(url, (newUrl) => {
  pauseParamsWatcher();
  try {
    const qIndex = newUrl.indexOf("?");
    if (qIndex === -1) {
      params.value = [{ key: "", value: "", enabled: true }];
      return;
    }
    const searchStr = newUrl.slice(qIndex + 1);
    const searchParams = new URLSearchParams(searchStr);
    const rows: KeyValueRow[] = [];
    searchParams.forEach((value, key) => {
      rows.push({ key, value, enabled: true });
    });
    rows.push({ key: "", value: "", enabled: true });
    params.value = rows;
  } finally {
    resumeParamsWatcher();
  }
});

// Sync params table -> URL query params (debounced + pausable)
const { pause: pauseParamsWatcher, resume: resumeParamsWatcher } = watchPausable(params, () => {}, { deep: true });

watchDebounced(
  params,
  (newParams) => {
    pauseUrlWatcher();
    try {
      const enabledParams = newParams.filter((p) => p.enabled && p.key);
      const baseUrl = url.value.split("?")[0];
      if (enabledParams.length === 0) {
        url.value = baseUrl;
        return;
      }
      const qs = enabledParams
        .map((p) => `${encodeParam(p.key)}=${encodeParam(p.value)}`)
        .join("&");
      url.value = `${baseUrl}?${qs}`;
    } finally {
      resumeUrlWatcher();
    }
  },
  { deep: true, debounce: 100 }
);

// Watch savedRequest prop to populate form
watch(
  () => props.savedRequest,
  (req) => {
    if (!req) return;
    method.value = req.method;
    url.value = req.url;
    response.value = null;
    error.value = null;

    // Headers
    const hRows: KeyValueRow[] = Object.entries(req.headers).map(([key, value]) => ({
      key,
      value,
      enabled: true,
    }));
    hRows.push({ key: "", value: "", enabled: true });
    headers.value = hRows;

    // Params
    if (req.params && req.params.length > 0) {
      const pRows: KeyValueRow[] = req.params.map((p) => ({
        key: p.key,
        value: p.value,
        enabled: p.enabled,
      }));
      pRows.push({ key: "", value: "", enabled: true });
      pauseUrlWatcher();
      params.value = pRows;
      resumeUrlWatcher();
    }

    // Body
    if (req.body) {
      bodyText.value = req.body;
      const ct = req.headers["Content-Type"] || req.headers["content-type"] || "";
      bodyContentType.value = detectContentType(ct, "json");
    } else {
      bodyContentType.value = "none";
      bodyText.value = "";
    }

    // Auth - default to "inherit" if the saved request has no auth / none
    const savedAuth = req.auth || { type: "none" };
    auth.value = savedAuth.type === "none" ? { type: "inherit" } : savedAuth;

    // Scripts
    preRequestScript.value = req.pre_request_script || "";
    testScript.value = req.test_script || "";

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
  }
);

// Watch historyEntry prop to populate form
watch(
  () => props.historyEntry,
  (entry) => {
    if (!entry) return;
    method.value = entry.method;
    url.value = entry.url;
    response.value = {
      status: entry.response_status,
      headers: safeParseJson(entry.response_headers),
      body: entry.response_body,
      duration_ms: entry.duration_ms,
    };
    error.value = null;

    // Parse request headers
    const parsedHeaders = safeParseJson(entry.request_headers);
    const hRows: KeyValueRow[] = Object.entries(parsedHeaders).map(([key, value]) => ({
      key,
      value: String(value),
      enabled: true,
    }));
    hRows.push({ key: "", value: "", enabled: true });
    headers.value = hRows;

    // Body
    if (entry.request_body) {
      bodyText.value = entry.request_body;
      const ct = parsedHeaders["Content-Type"] || parsedHeaders["content-type"] || "";
      bodyContentType.value = detectContentType(ct, "text");
    } else {
      bodyContentType.value = "none";
      bodyText.value = "";
    }
  }
);

function safeParseJson(str: string): Record<string, string> {
  try {
    return JSON.parse(str);
  } catch {
    return {};
  }
}

function getFormData() {
  const reqHeaders: Record<string, string> = {};
  headers.value.forEach((h) => {
    if (h.enabled && h.key) reqHeaders[h.key] = h.value;
  });

  const reqParams: KeyValueParam[] = params.value
    .filter((p) => p.key)
    .map((p) => ({ key: p.key, value: p.value, enabled: p.enabled }));

  let bodyStr: string | undefined;
  if (bodyContentType.value === "form-urlencoded") {
    const sp = new URLSearchParams();
    formData.value.forEach((r) => {
      if (r.enabled && r.key) sp.append(r.key, r.value);
    });
    bodyStr = sp.toString();
  } else if (bodyContentType.value !== "none") {
    bodyStr = bodyText.value || undefined;
  }

  // Store "inherit" as "none" in backend - inherit is a frontend-only concept
  const storedAuth: AuthConfig = auth.value.type === "inherit" ? { type: "none" } : auth.value;

  return {
    method: method.value,
    url: url.value,
    headers: reqHeaders,
    body: bodyStr,
    params: reqParams,
    auth: storedAuth,
    pre_request_script: preRequestScript.value || undefined,
    test_script: testScript.value || undefined,
    extraction_rules: extractionRules.value.length > 0 ? JSON.stringify(extractionRules.value) : undefined,
  };
}

function loadCurlImport(data: CurlImportResult) {
  method.value = data.method || "GET";
  url.value = data.url || "";
  response.value = null;
  error.value = null;

  // Headers
  const hRows: KeyValueRow[] = Object.entries(data.headers || {}).map(([key, value]) => ({
    key,
    value,
    enabled: true,
  }));
  hRows.push({ key: "", value: "", enabled: true });
  headers.value = hRows;

  // Params
  if (data.params && data.params.length > 0) {
    const pRows: KeyValueRow[] = data.params.map((p) => ({
      key: p.key,
      value: p.value,
      enabled: p.enabled,
    }));
    pRows.push({ key: "", value: "", enabled: true });
    pauseUrlWatcher();
    params.value = pRows;
    resumeUrlWatcher();
  } else {
    params.value = [{ key: "", value: "", enabled: true }];
  }

  // Body
  if (data.body) {
    bodyText.value = data.body;
    const ct = data.headers["Content-Type"] || data.headers["content-type"] || "";
    bodyContentType.value = detectContentType(ct, "json");
  } else {
    bodyContentType.value = "none";
    bodyText.value = "";
  }

  // Auth
  auth.value = data.auth || { type: "none" };
}

const urlBar = ref<InstanceType<typeof UrlBar> | null>(null);

function focusUrl() {
  urlBar.value?.focus();
}

defineExpose({ getFormData, loadCurlImport, send, focusUrl });

const unresolvedVars = ref<string[]>([]);

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

  // Auto-set Content-Type header
  const ctMap: Record<string, string> = {
    json: "application/json",
    xml: "application/xml",
    text: "text/plain",
    "form-urlencoded": "application/x-www-form-urlencoded",
  };
  if (bodyContentType.value !== "none" && ctMap[bodyContentType.value]) {
    reqHeaders["Content-Type"] = ctMap[bodyContentType.value];
  }

  let bodyStr: string | undefined;
  if (bodyContentType.value === "form-urlencoded") {
    const sp = new URLSearchParams();
    formData.value.forEach((r) => {
      if (r.enabled && r.key) sp.append(r.key, r.value);
    });
    bodyStr = sp.toString();
  } else if (bodyContentType.value !== "none") {
    bodyStr = bodyText.value || undefined;
  }

  const reqParams: KeyValueParam[] = params.value
    .filter((p) => p.key)
    .map((p) => ({ key: p.key, value: p.value, enabled: p.enabled }));

  // Resolve auth: "inherit" uses collection auth, otherwise use request's own auth
  let effectiveAuth: AuthConfig;
  if (auth.value.type === "inherit") {
    effectiveAuth = (props.collectionAuth && props.collectionAuth.type !== "none")
      ? props.collectionAuth
      : { type: "none" };
  } else {
    effectiveAuth = auth.value;
  }

  const payload: SendRequestPayload = {
    method: method.value as SendRequestPayload["method"],
    url: url.value.split("?")[0],
    headers: reqHeaders,
    body: bodyStr,
    params: reqParams,
    auth: effectiveAuth,
    environment_id: props.environmentId,
    pre_request_script: preRequestScript.value || undefined,
    test_script: testScript.value || undefined,
    collection_id: props.savedRequest?.collection_id,
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
</script>

<template>
  <div class="request-builder">
    <div class="request-section">
      <UrlBar
        ref="urlBar"
        :method="method"
        :url="url"
        :loading="loading"
        :variable-names="variableNames"
        @update:method="method = $event"
        @update:url="url = $event"
        @send="send"
      />

      <div v-if="unresolvedVars.length > 0" class="unresolved-warning">
        Unresolved variables: {{ unresolvedVars.map(v => '{' + '{' + v + '}' + '}').join(', ') }}
      </div>

      <div class="tabs">
        <button
          class="tab"
          :class="{ active: activeTab === 'params' }"
          @click="activeTab = 'params'"
        >
          Params
        </button>
        <button
          class="tab"
          :class="{ active: activeTab === 'headers' }"
          @click="activeTab = 'headers'"
        >
          Headers
        </button>
        <button
          class="tab"
          :class="{ active: activeTab === 'body' }"
          @click="activeTab = 'body'"
        >
          Body
        </button>
        <button
          class="tab"
          :class="{ active: activeTab === 'scripts' }"
          @click="activeTab = 'scripts'"
        >
          Scripts
        </button>
        <button
          class="tab"
          :class="{ active: activeTab === 'auth' }"
          @click="activeTab = 'auth'"
        >
          Auth
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
        <KeyValueTable
          v-if="activeTab === 'params'"
          v-model="params"
          key-placeholder="Parameter"
          value-placeholder="Value"
          :variable-names="variableNames"
        />
        <KeyValueTable
          v-else-if="activeTab === 'headers'"
          v-model="headers"
          key-placeholder="Header"
          value-placeholder="Value"
          :variable-names="variableNames"
        />
        <BodyEditor
          v-else-if="activeTab === 'body'"
          :content-type="bodyContentType"
          :body="bodyText"
          :form-data="formData"
          :variable-names="variableNames"
          @update:content-type="bodyContentType = $event"
          @update:body="bodyText = $event"
          @update:form-data="formData = $event"
        />
        <ScriptEditor
          v-else-if="activeTab === 'scripts'"
          :pre-request-script="preRequestScript"
          :test-script="testScript"
          @update:pre-request-script="preRequestScript = $event"
          @update:test-script="testScript = $event"
        />
        <AuthEditor
          v-else-if="activeTab === 'auth'"
          v-model="auth"
          :inherited-auth="collectionAuth"
        />
        <ExtractionRulesEditor
          v-else-if="activeTab === 'extract'"
          :model-value="extractionRules"
          :extracted-variables="extractedVariables.length > 0 ? extractedVariables : undefined"
          @update:model-value="extractionRules = $event"
        />
      </div>
    </div>

    <ResponsePanel :response="response" :error="error" :loading="loading" :script-result="scriptResult" :extracted-variables="extractedVariables" />
  </div>
</template>

<style scoped>
.request-builder {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.unresolved-warning {
  background: rgba(255, 152, 0, 0.12);
  border: 1px solid var(--warning);
  border-radius: var(--radius-md);
  color: var(--warning);
  font-size: 12px;
  padding: 6px 10px;
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
}

.tab:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.tab-content {
  flex: 1;
  min-height: 0;
}
</style>
