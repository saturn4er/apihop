<script setup lang="ts">
import { computed } from "vue";
import type { ExtractionRule, ExtractionSource, ExtractedVariable } from "@/api/types";

let _nextId = 0;

interface RuleRow {
  _id: number;
  sourceType: ExtractionSource["type"];
  path: string;
  targetVariable: string;
}

const props = defineProps<{
  modelValue: ExtractionRule[];
  extractedVariables?: ExtractedVariable[];
}>();

const emit = defineEmits<{
  "update:modelValue": [value: ExtractionRule[]];
}>();

function toRows(rules: ExtractionRule[]): RuleRow[] {
  const rows: RuleRow[] = rules.map((r) => {
    const sourceType = r.source.type;
    let path = "";
    if (r.source.type === "json_path") path = r.source.path;
    else if (r.source.type === "header") path = r.source.name;
    return { _id: _nextId++, sourceType, path, targetVariable: r.target_variable };
  });
  rows.push(emptyRow());
  return rows;
}

function emptyRow(): RuleRow {
  return { _id: _nextId++, sourceType: "json_path", path: "", targetVariable: "" };
}

const rows = computed(() => toRows(props.modelValue));

function emitUpdate(newRows: RuleRow[]) {
  const rules: ExtractionRule[] = newRows
    .filter((r) => r.targetVariable.trim() !== "")
    .map((r) => {
      let source: ExtractionSource;
      switch (r.sourceType) {
        case "json_path":
          source = { type: "json_path", path: r.path };
          break;
        case "header":
          source = { type: "header", name: r.path };
          break;
        case "status":
          source = { type: "status" };
          break;
        case "response_body":
          source = { type: "response_body" };
          break;
        default:
          source = { type: "json_path", path: r.path };
      }
      return { source, target_variable: r.targetVariable };
    });
  emit("update:modelValue", rules);
}

function update(index: number, field: keyof RuleRow, val: string) {
  const newRows = rows.value.map((r) => ({ ...r }));
  (newRows[index] as any)[field] = val;
  // Auto-append empty row when editing the last row
  if (index === newRows.length - 1 && (newRows[index].targetVariable !== "" || newRows[index].path !== "")) {
    newRows.push(emptyRow());
  }
  emitUpdate(newRows);
}

function removeRow(index: number) {
  const newRows = rows.value.filter((_, i) => i !== index);
  emitUpdate(newRows);
}

const needsPath = computed(() => (type: ExtractionSource["type"]) => {
  return type === "json_path" || type === "header";
});

function getExtractedResult(varName: string): ExtractedVariable | undefined {
  return props.extractedVariables?.find((v) => v.variable_name === varName);
}

const sourceTypeOptions: { value: ExtractionSource["type"]; label: string }[] = [
  { value: "json_path", label: "JSONPath" },
  { value: "header", label: "Header" },
  { value: "status", label: "Status" },
  { value: "response_body", label: "Body" },
];
</script>

<template>
  <div class="extraction-editor">
    <div class="ext-header">
      <span class="ext-col ext-source-col">Source</span>
      <span class="ext-col ext-path-col">Path / Name</span>
      <span class="ext-col ext-var-col">Variable</span>
      <span class="ext-col ext-result-col" v-if="extractedVariables">Result</span>
      <span class="ext-action-col"></span>
    </div>
    <div
      v-for="(row, i) in rows"
      :key="row._id"
      class="ext-row"
    >
      <span class="ext-col ext-source-col">
        <select
          :value="row.sourceType"
          @change="update(i, 'sourceType', ($event.target as HTMLSelectElement).value)"
        >
          <option v-for="opt in sourceTypeOptions" :key="opt.value" :value="opt.value">
            {{ opt.label }}
          </option>
        </select>
      </span>
      <span class="ext-col ext-path-col">
        <input
          v-if="needsPath(row.sourceType)"
          type="text"
          :placeholder="row.sourceType === 'json_path' ? '$.data.id' : 'Header-Name'"
          :value="row.path"
          @input="update(i, 'path', ($event.target as HTMLInputElement).value)"
        />
        <span v-else class="path-na">N/A</span>
      </span>
      <span class="ext-col ext-var-col">
        <input
          type="text"
          placeholder="variable_name"
          :value="row.targetVariable"
          @input="update(i, 'targetVariable', ($event.target as HTMLInputElement).value)"
        />
      </span>
      <span class="ext-col ext-result-col" v-if="extractedVariables">
        <template v-if="row.targetVariable && getExtractedResult(row.targetVariable)">
          <span
            v-if="getExtractedResult(row.targetVariable)!.value !== null"
            class="result-value"
          >{{ getExtractedResult(row.targetVariable)!.value }}</span>
          <span
            v-else-if="getExtractedResult(row.targetVariable)!.error !== null"
            class="result-error"
          >{{ getExtractedResult(row.targetVariable)!.error }}</span>
        </template>
      </span>
      <span class="ext-action-col">
        <button
          v-if="i < rows.length - 1"
          class="delete-btn"
          @click="removeRow(i)"
        >
          &times;
        </button>
      </span>
    </div>
  </div>
</template>

<style scoped>
.extraction-editor {
  width: 100%;
  font-size: 13px;
}

.ext-header {
  display: flex;
  gap: 0;
  padding: 6px 0;
  border-bottom: 1px solid var(--border-subtle);
  color: var(--text-secondary);
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.ext-row {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--border-subtle);
  transition: background var(--transition);
}

.ext-row:hover {
  background: var(--bg-hover);
}

.ext-col {
  display: flex;
  align-items: center;
}

.ext-source-col {
  width: 120px;
  flex-shrink: 0;
}

.ext-path-col {
  flex: 1;
}

.ext-var-col {
  flex: 1;
}

.ext-result-col {
  flex: 1;
  padding: 0 8px;
  overflow: hidden;
}

.ext-action-col {
  width: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.ext-source-col select {
  width: 100%;
  background: transparent;
  border: none;
  padding: 8px 8px;
  color: var(--text-primary);
  font-size: 12px;
  outline: none;
  cursor: pointer;
}

.ext-path-col input,
.ext-var-col input {
  width: 100%;
  background: transparent;
  border: none;
  padding: 8px 12px;
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
}

.ext-path-col input::placeholder,
.ext-var-col input::placeholder {
  color: var(--text-muted);
}

.path-na {
  padding: 8px 12px;
  color: var(--text-muted);
  font-size: 12px;
  font-style: italic;
}

.result-value {
  color: var(--success);
  font-size: 12px;
  font-family: "SF Mono", "Fira Code", monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.result-error {
  color: var(--error);
  font-size: 12px;
  font-family: "SF Mono", "Fira Code", monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.delete-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 18px;
  cursor: pointer;
  padding: 4px 8px;
  line-height: 1;
  border-radius: var(--radius-sm);
  opacity: 0;
  transition: all var(--transition);
}

.ext-row:hover .delete-btn {
  opacity: 1;
}

.delete-btn:hover {
  color: var(--error);
  background: rgba(244, 67, 54, 0.1);
}
</style>
