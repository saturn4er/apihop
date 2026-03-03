<script setup lang="ts">
import { ref } from "vue";
import BaseDialog from "./BaseDialog.vue";
import {
  importPostman,
  importOpenApi,
  importCurl,
  type Collection,
  type CurlImportResult,
} from "@/api/client";

const emit = defineEmits<{
  close: [];
  "imported-collection": [collection: Collection];
  "imported-curl": [result: CurlImportResult];
}>();

const activeTab = ref<"postman" | "openapi" | "curl">("postman");
const textContent = ref("");
const importing = ref(false);
const errorMessage = ref<string | null>(null);

function onFileSelected(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  const reader = new FileReader();
  reader.onload = () => {
    textContent.value = reader.result as string;
  };
  reader.onerror = () => {
    errorMessage.value = "Failed to read file.";
  };
  reader.readAsText(file);
}

function switchTab(tab: "postman" | "openapi" | "curl") {
  activeTab.value = tab;
  textContent.value = "";
  errorMessage.value = null;
}

async function doImport() {
  const data = textContent.value.trim();
  if (!data) {
    errorMessage.value = "Please provide content to import.";
    return;
  }
  importing.value = true;
  errorMessage.value = null;
  try {
    if (activeTab.value === "postman") {
      const col = await importPostman(data);
      emit("imported-collection", col);
    } else if (activeTab.value === "openapi") {
      const col = await importOpenApi(data);
      emit("imported-collection", col);
    } else {
      const result = await importCurl(data);
      emit("imported-curl", result);
    }
  } catch (e: any) {
    errorMessage.value = e?.message || String(e);
  } finally {
    importing.value = false;
  }
}
</script>

<template>
  <BaseDialog title="Import" width="540px" show-footer @close="emit('close')">
    <div class="import-tabs">
      <button
        class="import-tab"
        :class="{ active: activeTab === 'postman' }"
        @click="switchTab('postman')"
      >
        Postman
      </button>
      <button
        class="import-tab"
        :class="{ active: activeTab === 'openapi' }"
        @click="switchTab('openapi')"
      >
        OpenAPI
      </button>
      <button
        class="import-tab"
        :class="{ active: activeTab === 'curl' }"
        @click="switchTab('curl')"
      >
        cURL
      </button>
    </div>

    <div class="import-body">
      <div v-if="activeTab !== 'curl'" class="file-picker">
        <label class="file-picker-btn">
          Choose File
          <input
            type="file"
            :accept="activeTab === 'postman' ? '.json' : '.json,.yaml,.yml'"
            class="file-input-hidden"
            @change="onFileSelected"
          />
        </label>
        <span class="file-hint">or paste content below</span>
      </div>

      <textarea
        v-model="textContent"
        class="import-textarea"
        :placeholder="
          activeTab === 'curl'
            ? 'Paste curl command here...'
            : activeTab === 'postman'
              ? 'Paste Postman collection JSON here...'
              : 'Paste OpenAPI spec (JSON or YAML) here...'
        "
        spellcheck="false"
      ></textarea>

      <div v-if="errorMessage" class="import-error">{{ errorMessage }}</div>
    </div>

    <template #footer>
      <button class="btn btn-secondary" @click="emit('close')">Cancel</button>
      <button class="btn btn-primary" :disabled="importing" @click="doImport">
        {{ importing ? "Importing..." : "Import" }}
      </button>
    </template>
  </BaseDialog>
</template>

<style scoped>
.import-tabs {
  display: flex;
  border-bottom: 1px solid var(--border-subtle);
  margin: -16px -20px 12px;
  padding: 0 20px;
}

.import-tab {
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  padding: 10px 16px;
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition);
}

.import-tab:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.import-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.import-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.file-picker {
  display: flex;
  align-items: center;
  gap: 10px;
}

.file-picker-btn {
  display: inline-flex;
  align-items: center;
  padding: 6px 14px;
  background: var(--bg-tertiary);
  color: var(--text-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  font-size: 13px;
  cursor: pointer;
  transition: all var(--transition);
}

.file-picker-btn:hover {
  background: var(--bg-hover);
  border-color: var(--accent);
}

.file-input-hidden {
  display: none;
}

.file-hint {
  color: var(--text-muted);
  font-size: 12px;
}

.import-textarea {
  width: 100%;
  min-height: 200px;
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-family: "JetBrains Mono", "Fira Code", monospace;
  font-size: 12px;
  padding: 10px 12px;
  resize: vertical;
  outline: none;
  transition: border-color var(--transition);
}

.import-textarea::placeholder {
  color: var(--text-muted);
}

.import-textarea:focus {
  border-color: var(--accent);
}

.import-error {
  background: rgba(244, 67, 54, 0.1);
  border: 1px solid var(--error);
  border-radius: var(--radius-sm);
  color: var(--error);
  font-size: 12px;
  padding: 8px 12px;
  word-break: break-word;
}

.btn {
  padding: 7px 16px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  cursor: pointer;
  border: none;
  transition: all var(--transition);
}

.btn-secondary {
  background: var(--bg-tertiary);
  color: var(--text-secondary);
}

.btn-secondary:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.btn-primary {
  background: var(--accent);
  color: white;
}

.btn-primary:hover {
  background: var(--accent-hover);
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
