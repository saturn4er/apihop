<script setup lang="ts">
import { computed } from "vue";
import type { GraphQLSchema, GraphQLType, GraphQLField, GraphQLTypeRef } from "@/api/types";

const props = defineProps<{
  field: GraphQLField;
  path: string;
  schema: GraphQLSchema;
  selectedFields: Set<string>;
  expandedNodes: Set<string>;
  argValues: Map<string, string>;
  depth: number;
}>();

const emit = defineEmits<{
  toggle: [path: string, field: GraphQLField];
  "toggle-expand": [path: string];
  "arg-change": [path: string, argName: string, value: string];
}>();

function resolveTypeName(ref: GraphQLTypeRef): string | undefined {
  if (ref.name) return ref.name;
  if (ref.of_type) return resolveTypeName(ref.of_type);
  return undefined;
}

function isObjectType(typeRef: GraphQLTypeRef): boolean {
  const name = resolveTypeName(typeRef);
  if (!name) return false;
  const t = props.schema.types.find((tt) => tt.name === name);
  return !!t && (t.kind === "OBJECT" || t.kind === "INTERFACE");
}

function getFieldType(typeRef: GraphQLTypeRef): GraphQLType | null {
  const name = resolveTypeName(typeRef);
  if (!name) return null;
  return props.schema.types.find((t) => t.name === name) ?? null;
}

function formatTypeRef(t: GraphQLTypeRef): string {
  if (t.kind === "NON_NULL") return t.of_type ? formatTypeRef(t.of_type) + "!" : "!";
  if (t.kind === "LIST") return t.of_type ? "[" + formatTypeRef(t.of_type) + "]" : "[]";
  return t.name || "Unknown";
}

function isRequired(t: GraphQLTypeRef): boolean {
  return t.kind === "NON_NULL";
}

const isSelected = computed(() => props.selectedFields.has(props.path));
const isExpanded = computed(() => props.expandedNodes.has(props.path));
const hasChildren = computed(() => isObjectType(props.field.type));
const hasArgs = computed(() => props.field.args.length > 0);

const subType = computed(() => (hasChildren.value ? getFieldType(props.field.type) : null));
const subFields = computed(() =>
  subType.value?.fields?.filter((f) => !f.name.startsWith("__")) ?? []
);

function getArgVal(argName: string): string {
  return props.argValues.get(props.path + ":" + argName) || "";
}
</script>

<template>
  <div class="field-node">
    <div class="field-row" :style="{ paddingLeft: depth * 16 + 'px' }">
      <span
        v-if="hasChildren && isSelected"
        class="expand-toggle"
        @click.stop="emit('toggle-expand', path)"
      >
        {{ isExpanded ? "\u25BE" : "\u25B8" }}
      </span>
      <span v-else class="expand-placeholder" />

      <input
        type="checkbox"
        class="field-checkbox"
        :checked="isSelected"
        @change="emit('toggle', path, field)"
      />

      <span class="field-name">{{ field.name }}</span>
      <span class="field-type-label">: {{ formatTypeRef(field.type) }}</span>

      <span v-if="field.is_deprecated" class="deprecated-badge">deprecated</span>
    </div>

    <div v-if="field.description && isSelected" class="field-desc" :style="{ paddingLeft: (depth * 16 + 36) + 'px' }">
      {{ field.description }}
    </div>

    <!-- Arguments -->
    <div v-if="hasArgs && isSelected" class="args-section">
      <div
        v-for="arg in field.args"
        :key="arg.name"
        class="arg-row"
        :style="{ paddingLeft: (depth * 16 + 36) + 'px' }"
      >
        <span class="arg-name">{{ arg.name }}</span>
        <span class="arg-type-label">{{ formatTypeRef(arg.type) }}</span>
        <input
          class="arg-input"
          :placeholder="arg.default_value || (isRequired(arg.type) ? 'required' : 'optional')"
          :value="getArgVal(arg.name)"
          @input="emit('arg-change', path, arg.name, ($event.target as HTMLInputElement).value)"
        />
      </div>
    </div>

    <!-- Recursive sub-fields -->
    <div v-if="hasChildren && isSelected && isExpanded && subFields.length > 0" class="sub-fields">
      <GraphQLFieldRow
        v-for="sub in subFields"
        :key="sub.name"
        :field="sub"
        :path="path + '.' + sub.name"
        :schema="schema"
        :selected-fields="selectedFields"
        :expanded-nodes="expandedNodes"
        :arg-values="argValues"
        :depth="depth + 1"
        @toggle="(p: string, f: GraphQLField) => emit('toggle', p, f)"
        @toggle-expand="(p: string) => emit('toggle-expand', p)"
        @arg-change="(p: string, a: string, v: string) => emit('arg-change', p, a, v)"
      />
    </div>
  </div>
</template>

<style scoped>
.field-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  border-radius: var(--radius-sm);
  cursor: default;
  min-height: 28px;
}

.field-row:hover {
  background: var(--bg-hover);
}

.expand-toggle {
  font-size: 10px;
  color: var(--text-muted);
  width: 12px;
  flex-shrink: 0;
  cursor: pointer;
  text-align: center;
}

.expand-placeholder {
  width: 12px;
  flex-shrink: 0;
}

.field-checkbox {
  accent-color: var(--accent);
  cursor: pointer;
  flex-shrink: 0;
}

.field-name {
  font-family: "SF Mono", "Fira Code", monospace;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
}

.field-type-label {
  font-family: "SF Mono", "Fira Code", monospace;
  font-size: 11px;
  color: var(--accent);
  opacity: 0.8;
}

.deprecated-badge {
  background: rgba(255, 152, 0, 0.15);
  color: var(--warning);
  font-size: 9px;
  padding: 1px 4px;
  border-radius: 3px;
}

.field-desc {
  font-size: 11px;
  color: var(--text-muted);
  font-style: italic;
  padding: 0 8px 2px;
}

.args-section {
  padding: 2px 0;
}

.arg-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 8px;
  font-size: 11px;
}

.arg-name {
  color: var(--text-secondary);
  font-family: "SF Mono", "Fira Code", monospace;
  flex-shrink: 0;
}

.arg-type-label {
  color: var(--text-muted);
  font-family: "SF Mono", "Fira Code", monospace;
  font-size: 10px;
  flex-shrink: 0;
}

.arg-input {
  flex: 1;
  max-width: 200px;
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 11px;
  font-family: "SF Mono", "Fira Code", monospace;
  padding: 2px 6px;
  outline: none;
}

.arg-input:focus {
  border-color: var(--accent);
}

.arg-input::placeholder {
  color: var(--text-muted);
  font-style: italic;
}

.sub-fields {
  border-left: 1px solid var(--border-subtle);
  margin-left: 20px;
}
</style>
