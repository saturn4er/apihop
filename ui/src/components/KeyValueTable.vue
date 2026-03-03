<script setup lang="ts">
import type { KeyValueRow } from "@/api/types";
import VariableInput from "./VariableInput.vue";

let _nextId = 0;

function ensureId(row: KeyValueRow): KeyValueRow & { _id: number } {
  if (row._id == null) {
    row._id = _nextId++;
  }
  return row as KeyValueRow & { _id: number };
}

const props = withDefaults(
  defineProps<{
    modelValue: KeyValueRow[];
    keyPlaceholder?: string;
    valuePlaceholder?: string;
    variableNames?: string[];
  }>(),
  {
    keyPlaceholder: "Key",
    valuePlaceholder: "Value",
    variableNames: () => [],
  }
);

const emit = defineEmits<{
  "update:modelValue": [value: KeyValueRow[]];
}>();

function update(index: number, field: keyof KeyValueRow, val: string | boolean) {
  const rows = props.modelValue.map((r) => ({ ...r }));
  (rows[index] as any)[field] = val;
  // Auto-append empty row when typing in the last row
  if (index === rows.length - 1 && (rows[index].key !== "" || rows[index].value !== "")) {
    rows.push({ _id: _nextId++, key: "", value: "", enabled: true });
  }
  emit("update:modelValue", rows);
}

function removeRow(index: number) {
  const rows = props.modelValue.filter((_, i) => i !== index);
  if (rows.length === 0) {
    rows.push({ _id: _nextId++, key: "", value: "", enabled: true });
  }
  emit("update:modelValue", rows);
}
</script>

<template>
  <div class="kv-table">
    <div class="kv-header">
      <span class="kv-check-col"></span>
      <span class="kv-key-col">{{ keyPlaceholder }}</span>
      <span class="kv-val-col">{{ valuePlaceholder }}</span>
      <span class="kv-action-col"></span>
    </div>
    <div
      v-for="(row, i) in modelValue"
      :key="ensureId(row)._id"
      class="kv-row"
      :class="{ disabled: !row.enabled }"
    >
      <span class="kv-check-col">
        <input
          type="checkbox"
          :checked="row.enabled"
          @change="update(i, 'enabled', !row.enabled)"
        />
      </span>
      <span class="kv-key-col">
        <VariableInput
          :model-value="row.key"
          :placeholder="keyPlaceholder"
          :variable-names="variableNames"
          @update:model-value="update(i, 'key', $event)"
        />
      </span>
      <span class="kv-val-col">
        <VariableInput
          :model-value="row.value"
          :placeholder="valuePlaceholder"
          :variable-names="variableNames"
          @update:model-value="update(i, 'value', $event)"
        />
      </span>
      <span class="kv-action-col">
        <button
          v-if="modelValue.length > 1 || row.key || row.value"
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
.kv-table {
  width: 100%;
  font-size: 13px;
}

.kv-header {
  display: flex;
  gap: 0;
  padding: 6px 0;
  border-bottom: 1px solid var(--border-subtle);
  color: var(--text-secondary);
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.kv-row {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--border-subtle);
  transition: background var(--transition);
}

.kv-row:hover {
  background: var(--bg-hover);
}

.kv-row.disabled {
  opacity: 0.45;
}

.kv-check-col {
  width: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.kv-check-col input[type="checkbox"] {
  appearance: none;
  -webkit-appearance: none;
  width: 14px;
  height: 14px;
  border: 1.5px solid var(--border-color);
  border-radius: 3px;
  background: transparent;
  cursor: pointer;
  position: relative;
  transition: all var(--transition);
}

.kv-check-col input[type="checkbox"]:checked {
  background: var(--accent);
  border-color: var(--accent);
}

.kv-check-col input[type="checkbox"]:checked::after {
  content: '';
  position: absolute;
  left: 3.5px;
  top: 1px;
  width: 4px;
  height: 8px;
  border: solid #fff;
  border-width: 0 1.5px 1.5px 0;
  transform: rotate(45deg);
}

.kv-key-col,
.kv-val-col {
  flex: 1;
  display: flex;
  align-items: center;
}

.kv-key-col :deep(input),
.kv-val-col :deep(input) {
  width: 100%;
  background: transparent;
  border: none;
  padding: 8px 12px;
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
}

.kv-key-col :deep(input)::placeholder,
.kv-val-col :deep(input)::placeholder {
  color: var(--text-muted);
}

.kv-action-col {
  width: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
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

.kv-row:hover .delete-btn {
  opacity: 1;
}

.delete-btn:hover {
  color: var(--error);
  background: rgba(244, 67, 54, 0.1);
}
</style>
