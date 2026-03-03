<script setup lang="ts">
import type { KeyValueRow } from "@/api/types";
import KeyValueTable from "./KeyValueTable.vue";
import CodeEditor from "./CodeEditor.vue";
import { formatJson, formatXml } from "@/utils/format";

type ContentType = "none" | "json" | "xml" | "text" | "form-urlencoded";

const props = withDefaults(
  defineProps<{
    contentType: ContentType;
    body: string;
    formData: KeyValueRow[];
    variableNames?: string[];
  }>(),
  { variableNames: () => [] }
);

const emit = defineEmits<{
  "update:contentType": [value: ContentType];
  "update:body": [value: string];
  "update:formData": [value: KeyValueRow[]];
}>();

function handleFormat() {
  const formatted =
    props.contentType === "json" ? formatJson(props.body) : formatXml(props.body);
  emit("update:body", formatted);
}

const contentTypes: { label: string; value: ContentType }[] = [
  { label: "None", value: "none" },
  { label: "JSON", value: "json" },
  { label: "XML", value: "xml" },
  { label: "Text", value: "text" },
  { label: "Form URL-Encoded", value: "form-urlencoded" },
];
</script>

<template>
  <div class="body-editor">
    <div class="content-type-bar">
      <label
        v-for="ct in contentTypes"
        :key="ct.value"
        class="ct-option"
        :class="{ active: contentType === ct.value }"
      >
        <input
          type="radio"
          name="contentType"
          :value="ct.value"
          :checked="contentType === ct.value"
          @change="emit('update:contentType', ct.value)"
        />
        {{ ct.label }}
      </label>
    </div>
    <div v-if="contentType === 'json' || contentType === 'xml'" class="editor-toolbar">
      <button class="format-btn" @click="handleFormat">Format</button>
    </div>
    <div class="body-content">
      <div v-if="contentType === 'none'" class="body-none">
        This request does not have a body
      </div>
      <CodeEditor
        v-else-if="contentType !== 'form-urlencoded'"
        :modelValue="body"
        :language="contentType === 'json' ? 'json' : contentType === 'xml' ? 'xml' : 'text'"
        :placeholder="contentType === 'json' ? '{\n  \n}' : ''"
        :variable-names="variableNames"
        @update:modelValue="emit('update:body', $event)"
      />
      <KeyValueTable
        v-else
        :model-value="formData"
        key-placeholder="Key"
        value-placeholder="Value"
        @update:model-value="emit('update:formData', $event)"
      />
    </div>
  </div>
</template>

<style scoped>
.body-editor {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.content-type-bar {
  display: flex;
  gap: 6px;
  padding: 8px 0;
  border-bottom: 1px solid var(--border-subtle);
  flex-wrap: wrap;
}

.ct-option {
  padding: 5px 14px;
  border-radius: 20px;
  font-size: 12px;
  cursor: pointer;
  color: var(--text-secondary);
  transition: all var(--transition);
  user-select: none;
}

.ct-option:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.ct-option.active {
  background: var(--accent-muted);
  color: var(--accent);
  font-weight: 600;
}

.ct-option input[type="radio"] {
  display: none;
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

.body-none {
  padding: 24px;
  color: var(--text-muted);
  text-align: center;
  font-size: 13px;
}

.body-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 150px;
}
</style>
