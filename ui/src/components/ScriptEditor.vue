<script setup lang="ts">
import { ref } from "vue";
import CodeEditor from "./CodeEditor.vue";

const props = withDefaults(
  defineProps<{
    preRequestScript?: string;
    testScript?: string;
  }>(),
  {
    preRequestScript: "",
    testScript: "",
  }
);

const emit = defineEmits<{
  "update:preRequestScript": [value: string];
  "update:testScript": [value: string];
}>();

const activeTab = ref<"pre-request" | "tests">("pre-request");

const preRequestSnippets = [
  { label: "Set Header", code: 'pm.request.headers.add("X-Custom", "value");' },
  { label: "Set Variable", code: 'pm.variables.set("key", "value");' },
  { label: "Log", code: 'console.log("message");' },
];

const testSnippets = [
  {
    label: "Status Check",
    code: 'pm.test("Status is 200", () => {\n  pm.expect(pm.response.status).to.equal(200);\n});',
  },
  {
    label: "JSON Value",
    code: 'pm.test("Check value", () => {\n  const json = pm.response.json();\n  pm.expect(json.key).to.equal("value");\n});',
  },
  {
    label: "Header Check",
    code: 'pm.test("Has header", () => {\n  pm.expect(pm.response.headers["content-type"]).to.include("json");\n});',
  },
];

function insertSnippet(code: string) {
  if (activeTab.value === "pre-request") {
    const current = props.preRequestScript;
    emit("update:preRequestScript", current ? current + "\n" + code : code);
  } else {
    const current = props.testScript;
    emit("update:testScript", current ? current + "\n" + code : code);
  }
}
</script>

<template>
  <div class="script-editor">
    <div class="script-tabs">
      <button
        class="script-tab"
        :class="{ active: activeTab === 'pre-request' }"
        @click="activeTab = 'pre-request'"
      >
        Pre-request
      </button>
      <button
        class="script-tab"
        :class="{ active: activeTab === 'tests' }"
        @click="activeTab = 'tests'"
      >
        Tests
      </button>
    </div>

    <div class="snippet-bar">
      <span class="snippet-label">Snippets:</span>
      <template v-if="activeTab === 'pre-request'">
        <button
          v-for="s in preRequestSnippets"
          :key="s.label"
          class="snippet-btn"
          @click="insertSnippet(s.code)"
        >
          {{ s.label }}
        </button>
      </template>
      <template v-else>
        <button
          v-for="s in testSnippets"
          :key="s.label"
          class="snippet-btn"
          @click="insertSnippet(s.code)"
        >
          {{ s.label }}
        </button>
      </template>
    </div>

    <div class="script-content">
      <CodeEditor
        v-if="activeTab === 'pre-request'"
        :modelValue="preRequestScript"
        language="javascript"
        placeholder="// Pre-request script runs before the request is sent"
        @update:modelValue="emit('update:preRequestScript', $event)"
      />
      <CodeEditor
        v-else
        :modelValue="testScript"
        language="javascript"
        placeholder="// Write test assertions for the response"
        @update:modelValue="emit('update:testScript', $event)"
      />
    </div>
  </div>
</template>

<style scoped>
.script-editor {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 200px;
}

.script-tabs {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--border-color);
}

.script-tab {
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  padding: 6px 16px;
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  transition: all var(--transition);
}

.script-tab:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.script-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.snippet-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.snippet-label {
  font-size: 11px;
  color: var(--text-muted);
  flex-shrink: 0;
}

.snippet-btn {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 11px;
  padding: 2px 8px;
  cursor: pointer;
  transition: all var(--transition);
  white-space: nowrap;
}

.snippet-btn:hover {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--accent-muted);
}

.script-content {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
</style>
