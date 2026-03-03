<script setup lang="ts">
import { computed, ref } from "vue";
import type { ApiResponse, ScriptExecutionResult, ExtractedVariable } from "@/api/client";
import CodeEditor from "./CodeEditor.vue";
import { useToast } from "@/composables/useToast";

const { showToast } = useToast();

const props = defineProps<{
  response: ApiResponse | null;
  error: string | null;
  loading: boolean;
  scriptResult?: ScriptExecutionResult | null;
  extractedVariables?: ExtractedVariable[];
}>();

const activeTab = ref<"body" | "headers" | "meta" | "tests" | "console" | "extract">("body");

const testsPassed = computed(() => {
  if (!props.scriptResult) return 0;
  return props.scriptResult.test_results.filter((t) => t.passed).length;
});

const testsTotal = computed(() => {
  return props.scriptResult?.test_results.length ?? 0;
});

const allConsoleEntries = computed(() => {
  if (!props.scriptResult) return [];
  return [
    ...props.scriptResult.pre_request_console.map((e) => ({ ...e, phase: "pre-request" })),
    ...props.scriptResult.test_console.map((e) => ({ ...e, phase: "test" })),
  ];
});

const statusClass = computed(() => {
  if (!props.response) return "";
  const s = props.response.status;
  if (s >= 200 && s < 300) return "status-success";
  if (s >= 300 && s < 400) return "status-redirect";
  if (s >= 400 && s < 500) return "status-client-error";
  return "status-server-error";
});

const formattedSize = computed(() => {
  if (!props.response) return "";
  const bytes = props.response.size_bytes ?? new TextEncoder().encode(props.response.body).length;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
});

const formattedDuration = computed(() => {
  if (!props.response) return "";
  const ms = props.response.duration_ms;
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(2)} s`;
});

const statusDescription = computed(() => {
  if (!props.response) return "";
  const descriptions: Record<number, string> = {
    200: "OK",
    201: "Created",
    204: "No Content",
    301: "Moved Permanently",
    302: "Found",
    304: "Not Modified",
    400: "Bad Request",
    401: "Unauthorized",
    403: "Forbidden",
    404: "Not Found",
    405: "Method Not Allowed",
    409: "Conflict",
    422: "Unprocessable Entity",
    429: "Too Many Requests",
    500: "Internal Server Error",
    502: "Bad Gateway",
    503: "Service Unavailable",
    504: "Gateway Timeout",
  };
  return descriptions[props.response.status] || "";
});

const responseContentType = computed(() => {
  if (!props.response) return "";
  return props.response.content_type || props.response.headers["content-type"] || "";
});

const isJson = computed(() => responseContentType.value.includes("json"));

const responseLanguage = computed<"json" | "xml" | "html" | "text">(() => {
  const ct = responseContentType.value;
  if (ct.includes("json")) return "json";
  if (ct.includes("xml")) return "xml";
  if (ct.includes("html")) return "html";
  return "text";
});

const formattedBody = computed(() => {
  if (!props.response) return "";
  if (isJson.value) {
    try {
      return JSON.stringify(JSON.parse(props.response.body), null, 2);
    } catch {
      return props.response.body;
    }
  }
  return props.response.body;
});

const headerEntries = computed(() => {
  if (!props.response) return [];
  return Object.entries(props.response.headers).sort(([a], [b]) =>
    a.localeCompare(b)
  );
});

const copied = ref(false);
async function copyBody() {
  if (!props.response) return;
  await navigator.clipboard.writeText(formattedBody.value);
  copied.value = true;
  setTimeout(() => (copied.value = false), 1500);
  showToast("Copied to clipboard", "success");
}

async function copyHeaderValue(value: string) {
  await navigator.clipboard.writeText(value);
  showToast("Copied to clipboard", "success");
}
</script>

<template>
  <div class="response-panel">
    <div class="response-header">
      <div class="response-tabs" role="tablist" aria-label="Response tabs">
        <button
          class="rtab"
          :class="{ active: activeTab === 'body' }"
          role="tab"
          :aria-selected="activeTab === 'body'"
          @click="activeTab = 'body'"
        >
          Body
        </button>
        <button
          class="rtab"
          :class="{ active: activeTab === 'headers' }"
          role="tab"
          :aria-selected="activeTab === 'headers'"
          @click="activeTab = 'headers'"
        >
          Headers
          <span v-if="response" class="tab-badge">{{ headerEntries.length }}</span>
        </button>
        <button
          class="rtab"
          :class="{ active: activeTab === 'meta' }"
          role="tab"
          :aria-selected="activeTab === 'meta'"
          @click="activeTab = 'meta'"
        >
          Meta
        </button>
        <button
          class="rtab"
          :class="{ active: activeTab === 'tests' }"
          role="tab"
          :aria-selected="activeTab === 'tests'"
          @click="activeTab = 'tests'"
        >
          Tests
          <span v-if="scriptResult && testsTotal > 0" class="tab-badge" :class="{ 'badge-pass': testsPassed === testsTotal, 'badge-fail': testsPassed !== testsTotal }">
            {{ testsPassed }}/{{ testsTotal }}
          </span>
        </button>
        <button
          class="rtab"
          :class="{ active: activeTab === 'console' }"
          role="tab"
          :aria-selected="activeTab === 'console'"
          @click="activeTab = 'console'"
        >
          Console
          <span v-if="allConsoleEntries.length > 0" class="tab-badge">{{ allConsoleEntries.length }}</span>
        </button>
        <button
          class="rtab"
          :class="{ active: activeTab === 'extract' }"
          role="tab"
          :aria-selected="activeTab === 'extract'"
          @click="activeTab = 'extract'"
        >
          Extract
          <span v-if="extractedVariables && extractedVariables.length > 0" class="tab-badge">{{ extractedVariables.length }}</span>
        </button>
      </div>
      <div class="response-meta" v-if="response && !loading">
        <span class="status-badge" :class="statusClass">{{ response.status }} {{ statusDescription }}</span>
        <span class="meta">{{ formattedDuration }}</span>
        <span class="meta">{{ formattedSize }}</span>
      </div>
    </div>

    <div class="response-body">
      <div v-if="loading" class="response-placeholder">
        <span class="spinner"></span>
        <span>Sending request...</span>
      </div>
      <div v-else-if="error" class="response-error">{{ error }}</div>
      <div v-else-if="!response" class="response-placeholder">
        Send a request to see the response
      </div>

      <!-- Body tab -->
      <template v-else-if="activeTab === 'body'">
        <div class="body-toolbar">
          <button class="copy-btn" @click="copyBody">
            {{ copied ? "Copied!" : "Copy" }}
          </button>
        </div>
        <CodeEditor
          :modelValue="formattedBody"
          :language="responseLanguage"
          readonly
        />
      </template>

      <!-- Headers tab -->
      <div v-else-if="activeTab === 'headers'" class="headers-table">
        <div
          v-for="[name, value] in headerEntries"
          :key="name"
          class="header-row"
        >
          <span class="header-name">{{ name }}</span>
          <span class="header-value" @click="copyHeaderValue(value)" title="Click to copy">
            {{ value }}
          </span>
        </div>
      </div>

      <!-- Tests tab -->
      <div v-else-if="activeTab === 'tests'" class="tests-panel">
        <template v-if="scriptResult">
          <div v-if="scriptResult.pre_request_error" class="script-error-banner">
            <strong>Pre-request error:</strong> {{ scriptResult.pre_request_error }}
          </div>
          <div v-if="scriptResult.test_error" class="script-error-banner">
            <strong>Test error:</strong> {{ scriptResult.test_error }}
          </div>
          <div v-if="testsTotal > 0" class="tests-summary" :class="testsPassed === testsTotal ? 'summary-pass' : 'summary-fail'">
            {{ testsPassed }}/{{ testsTotal }} tests passed
          </div>
          <div v-for="(t, i) in scriptResult.test_results" :key="i" class="test-row">
            <span class="test-icon" :class="t.passed ? 'icon-pass' : 'icon-fail'">
              {{ t.passed ? "\u2713" : "\u2717" }}
            </span>
            <span class="test-name">{{ t.name }}</span>
            <span v-if="!t.passed && t.error" class="test-error">{{ t.error }}</span>
          </div>
          <div v-if="testsTotal === 0 && !scriptResult.pre_request_error && !scriptResult.test_error" class="response-placeholder">
            No test results
          </div>
        </template>
        <div v-else class="response-placeholder">
          No test results yet
        </div>
      </div>

      <!-- Console tab -->
      <div v-else-if="activeTab === 'console'" class="console-panel">
        <template v-if="allConsoleEntries.length > 0">
          <div v-for="(entry, i) in allConsoleEntries" :key="i" class="console-entry" :class="'console-' + entry.level">
            <span class="console-level">{{ entry.level }}</span>
            <span class="console-message">{{ entry.message }}</span>
          </div>
        </template>
        <div v-else class="response-placeholder">
          No console output
        </div>
      </div>

      <!-- Extract tab -->
      <div v-else-if="activeTab === 'extract'" class="extract-panel">
        <template v-if="extractedVariables && extractedVariables.length > 0">
          <div v-for="(ev, i) in extractedVariables" :key="i" class="extract-row">
            <span class="extract-name">{{ ev.variable_name }}</span>
            <span v-if="ev.value !== null" class="extract-value">{{ ev.value }}</span>
            <span v-else-if="ev.error !== null" class="extract-error">{{ ev.error }}</span>
          </div>
        </template>
        <div v-else class="response-placeholder">
          No extracted variables
        </div>
      </div>

      <!-- Meta tab -->
      <div v-else-if="activeTab === 'meta'" class="meta-panel">
        <div class="meta-row">
          <span class="meta-label">Status</span>
          <span class="status-badge" :class="statusClass">
            {{ response.status }} {{ statusDescription }}
          </span>
        </div>
        <div class="meta-row">
          <span class="meta-label">Duration</span>
          <span class="meta-value">{{ formattedDuration }}</span>
        </div>
        <div class="meta-row">
          <span class="meta-label">Size</span>
          <span class="meta-value">{{ formattedSize }}</span>
        </div>
        <div class="meta-row">
          <span class="meta-label">Content-Type</span>
          <span class="meta-value">{{ response.content_type || response.headers['content-type'] || 'N/A' }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.response-panel {
  border-top: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.response-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}

.response-tabs {
  display: flex;
  gap: 0;
}

.rtab {
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  padding: 8px 14px;
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  transition: all var(--transition);
  display: flex;
  align-items: center;
  gap: 4px;
}

.rtab:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.rtab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.tab-badge {
  background: var(--bg-tertiary);
  color: var(--text-secondary);
  font-size: 10px;
  padding: 1px 5px;
  border-radius: 8px;
}

.response-meta {
  display: flex;
  align-items: center;
  gap: 12px;
}

.status-badge {
  padding: 3px 12px;
  border-radius: 20px;
  font-weight: 700;
  font-size: 12px;
}

.status-success {
  background: rgba(76, 175, 80, 0.15);
  color: var(--success);
}

.status-redirect {
  background: rgba(33, 150, 243, 0.15);
  color: var(--info);
}

.status-client-error {
  background: rgba(255, 152, 0, 0.15);
  color: var(--warning);
}

.status-server-error {
  background: rgba(244, 67, 54, 0.15);
  color: var(--error);
}

.meta {
  font-size: 12px;
  color: var(--text-secondary);
}

.response-body {
  flex: 1;
  overflow: auto;
  min-height: 120px;
}

.response-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  height: 120px;
  color: var(--text-muted);
  font-size: 13px;
}

.response-error {
  padding: 16px;
  color: var(--error);
  font-size: 13px;
  white-space: pre-wrap;
}

.body-toolbar {
  display: flex;
  justify-content: flex-end;
  padding: 6px 16px 0;
}

.copy-btn {
  background: none;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 11px;
  padding: 3px 10px;
  cursor: pointer;
  transition: all var(--transition);
}

.copy-btn:hover {
  color: var(--text-primary);
  border-color: var(--accent);
  background: var(--bg-hover);
}

/* Headers tab */
.headers-table {
  padding: 12px 16px;
}

.header-row {
  display: flex;
  padding: 6px 8px;
  border-bottom: 1px solid var(--border-subtle);
  gap: 16px;
  border-radius: var(--radius-sm);
  transition: background var(--transition);
}

.header-row:hover {
  background: var(--bg-hover);
}

.header-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  min-width: 180px;
  flex-shrink: 0;
  font-family: "SF Mono", "Fira Code", monospace;
}

.header-value {
  font-size: 12px;
  color: var(--text-primary);
  word-break: break-all;
  font-family: "SF Mono", "Fira Code", monospace;
  cursor: pointer;
}

.header-value:hover {
  color: var(--accent);
}

/* Meta tab */
.meta-panel {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.meta-row {
  display: flex;
  align-items: center;
  gap: 16px;
  background: var(--bg-surface);
  border-radius: var(--radius-md);
  padding: 12px 16px;
}

.meta-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  min-width: 100px;
}

.meta-value {
  font-size: 13px;
  color: var(--text-primary);
  font-family: "SF Mono", "Fira Code", monospace;
}

.spinner {
  display: inline-block;
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

/* Tests tab */
.tests-panel {
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.script-error-banner {
  background: rgba(244, 67, 54, 0.12);
  border: 1px solid var(--error);
  border-radius: var(--radius-md);
  color: var(--error);
  font-size: 12px;
  padding: 8px 12px;
  margin-bottom: 8px;
}

.tests-summary {
  font-size: 13px;
  font-weight: 600;
  padding: 6px 0;
  margin-bottom: 4px;
}

.summary-pass {
  color: var(--success);
}

.summary-fail {
  color: var(--error);
}

.test-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  font-size: 13px;
}

.test-row:hover {
  background: var(--bg-hover);
}

.test-icon {
  font-weight: 700;
  flex-shrink: 0;
  width: 16px;
  text-align: center;
}

.icon-pass {
  color: var(--success);
}

.icon-fail {
  color: var(--error);
}

.test-name {
  color: var(--text-primary);
}

.test-error {
  color: var(--error);
  font-size: 12px;
  margin-left: auto;
  font-family: "SF Mono", "Fira Code", monospace;
}

.badge-pass {
  background: rgba(76, 175, 80, 0.2) !important;
  color: var(--success) !important;
}

.badge-fail {
  background: rgba(244, 67, 54, 0.2) !important;
  color: var(--error) !important;
}

/* Console tab */
.console-panel {
  padding: 12px 16px;
  font-family: "SF Mono", "Fira Code", monospace;
  font-size: 12px;
}

.console-entry {
  display: flex;
  gap: 10px;
  padding: 4px 8px;
  border-bottom: 1px solid var(--border-subtle);
}

.console-level {
  flex-shrink: 0;
  min-width: 40px;
  font-weight: 600;
  text-transform: uppercase;
  font-size: 10px;
  padding-top: 2px;
}

.console-log .console-level {
  color: var(--text-secondary);
}

.console-warn .console-level {
  color: var(--warning);
}

.console-error .console-level {
  color: var(--error);
}

.console-message {
  color: var(--text-primary);
  word-break: break-all;
}

/* Extract tab */
.extract-panel {
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.extract-row {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  transition: background var(--transition);
}

.extract-row:hover {
  background: var(--bg-hover);
}

.extract-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  min-width: 150px;
  flex-shrink: 0;
  font-family: "SF Mono", "Fira Code", monospace;
}

.extract-value {
  font-size: 12px;
  color: var(--success);
  word-break: break-all;
  font-family: "SF Mono", "Fira Code", monospace;
}

.extract-error {
  font-size: 12px;
  color: var(--error);
  word-break: break-all;
  font-family: "SF Mono", "Fira Code", monospace;
}
</style>
