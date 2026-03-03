<script setup lang="ts">
import { ref, computed } from "vue";
import type { GraphQLSchema, GraphQLType, GraphQLTypeRef } from "@/api/types";

const props = defineProps<{
  schema: GraphQLSchema;
}>();

const emit = defineEmits<{
  insert: [text: string];
}>();

const searchQuery = ref("");
const expandedTypes = ref<Set<string>>(new Set());

function formatTypeRef(t: GraphQLTypeRef): string {
  if (t.kind === "NON_NULL") {
    return t.of_type ? formatTypeRef(t.of_type) + "!" : "!";
  }
  if (t.kind === "LIST") {
    return t.of_type ? "[" + formatTypeRef(t.of_type) + "]" : "[]";
  }
  return t.name || "Unknown";
}

// Filter to user-defined types (exclude introspection types)
const userTypes = computed(() => {
  return props.schema.types.filter(
    (t) => t.name && !t.name.startsWith("__")
  );
});

const filteredTypes = computed(() => {
  if (!searchQuery.value) return userTypes.value;
  const q = searchQuery.value.toLowerCase();
  return userTypes.value.filter((t) => {
    if (t.name?.toLowerCase().includes(q)) return true;
    if (t.description?.toLowerCase().includes(q)) return true;
    if (t.fields?.some((f) => f.name.toLowerCase().includes(q))) return true;
    return false;
  });
});

const rootTypes = computed(() => {
  const names = new Set<string>();
  if (props.schema.query_type) names.add(props.schema.query_type);
  if (props.schema.mutation_type) names.add(props.schema.mutation_type);
  if (props.schema.subscription_type) names.add(props.schema.subscription_type);
  return names;
});

function toggleType(name: string) {
  if (expandedTypes.value.has(name)) {
    expandedTypes.value.delete(name);
  } else {
    expandedTypes.value.add(name);
  }
}

function kindLabel(kind: string): string {
  const labels: Record<string, string> = {
    OBJECT: "type",
    INPUT_OBJECT: "input",
    ENUM: "enum",
    SCALAR: "scalar",
    INTERFACE: "interface",
    UNION: "union",
  };
  return labels[kind] || kind.toLowerCase();
}

function kindClass(kind: string): string {
  return "kind-" + kind.toLowerCase().replace("_", "-");
}

function buildQueryForType(type: GraphQLType): string {
  if (!type.fields) return "";
  const fields = type.fields
    .filter((f) => !f.name.startsWith("__"))
    .slice(0, 10)
    .map((f) => "  " + f.name)
    .join("\n");
  const prefix = rootTypes.value.has(type.name || "")
    ? type.name === props.schema.mutation_type
      ? "mutation"
      : type.name === props.schema.subscription_type
        ? "subscription"
        : "query"
    : "query";
  return `${prefix} {\n${fields}\n}`;
}

function insertType(type: GraphQLType) {
  const queryStr = buildQueryForType(type);
  if (queryStr) emit("insert", queryStr);
}
</script>

<template>
  <div class="schema-explorer">
    <div class="search-bar">
      <input v-model="searchQuery" placeholder="Search types..." class="search-input" />
    </div>
    <div class="type-list">
      <div v-for="type in filteredTypes" :key="type.name" class="type-item">
        <div class="type-header" @click="toggleType(type.name || '')">
          <span class="expand-icon">{{ expandedTypes.has(type.name || '') ? "\u25BE" : "\u25B8" }}</span>
          <span class="kind-badge" :class="kindClass(type.kind)">{{ kindLabel(type.kind) }}</span>
          <span class="type-name" :class="{ 'root-type': rootTypes.has(type.name || '') }">
            {{ type.name }}
          </span>
          <button
            v-if="type.fields && rootTypes.has(type.name || '')"
            class="insert-btn"
            title="Insert query"
            @click.stop="insertType(type)"
          >
            Insert
          </button>
        </div>
        <div v-if="type.description && expandedTypes.has(type.name || '')" class="type-description">
          {{ type.description }}
        </div>
        <div v-if="expandedTypes.has(type.name || '')" class="type-fields">
          <!-- Object/Interface fields -->
          <template v-if="type.fields">
            <div v-for="field in type.fields" :key="field.name" class="field-item">
              <span class="field-name">{{ field.name }}</span>
              <span v-if="field.args.length > 0" class="field-args">
                ({{ field.args.map((a) => a.name + ": " + formatTypeRef(a.type)).join(", ") }})
              </span>
              <span class="field-type">: {{ formatTypeRef(field.type) }}</span>
              <span v-if="field.is_deprecated" class="deprecated">deprecated</span>
              <div v-if="field.description" class="field-description">{{ field.description }}</div>
            </div>
          </template>
          <!-- Input fields -->
          <template v-if="type.input_fields">
            <div v-for="field in type.input_fields" :key="field.name" class="field-item">
              <span class="field-name">{{ field.name }}</span>
              <span class="field-type">: {{ formatTypeRef(field.type) }}</span>
              <div v-if="field.description" class="field-description">{{ field.description }}</div>
            </div>
          </template>
          <!-- Enum values -->
          <template v-if="type.enum_values">
            <div v-for="ev in type.enum_values" :key="ev.name" class="field-item">
              <span class="enum-value">{{ ev.name }}</span>
              <span v-if="ev.is_deprecated" class="deprecated">deprecated</span>
              <div v-if="ev.description" class="field-description">{{ ev.description }}</div>
            </div>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.schema-explorer {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.search-bar {
  padding: 8px 0;
  flex-shrink: 0;
}

.search-input {
  width: 100%;
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-size: 12px;
  padding: 6px 10px;
  outline: none;
}

.search-input:focus {
  border-color: var(--accent);
}

.type-list {
  flex: 1;
  overflow: auto;
}

.type-item {
  border-bottom: 1px solid var(--border-subtle);
}

.type-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 4px;
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: background var(--transition);
}

.type-header:hover {
  background: var(--bg-hover);
}

.expand-icon {
  font-size: 10px;
  color: var(--text-muted);
  width: 12px;
  flex-shrink: 0;
}

.kind-badge {
  font-size: 9px;
  font-weight: 700;
  padding: 1px 5px;
  border-radius: 4px;
  text-transform: uppercase;
  flex-shrink: 0;
}

.kind-object { background: rgba(108, 99, 255, 0.15); color: var(--accent); }
.kind-input-object { background: rgba(255, 152, 0, 0.15); color: var(--warning); }
.kind-enum { background: rgba(76, 175, 80, 0.15); color: var(--success); }
.kind-scalar { background: rgba(158, 158, 158, 0.15); color: var(--text-secondary); }
.kind-interface { background: rgba(33, 150, 243, 0.15); color: var(--info); }
.kind-union { background: rgba(156, 39, 176, 0.15); color: #ce93d8; }

.type-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  font-family: "SF Mono", "Fira Code", monospace;
}

.root-type {
  color: var(--accent);
}

.insert-btn {
  margin-left: auto;
  background: none;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 10px;
  padding: 2px 8px;
  cursor: pointer;
  transition: all var(--transition);
}

.insert-btn:hover {
  color: var(--accent);
  border-color: var(--accent);
}

.type-description {
  padding: 2px 4px 4px 24px;
  font-size: 11px;
  color: var(--text-muted);
  font-style: italic;
}

.type-fields {
  padding: 0 4px 8px 24px;
}

.field-item {
  padding: 3px 6px;
  font-size: 12px;
  border-radius: var(--radius-sm);
  transition: background var(--transition);
}

.field-item:hover {
  background: var(--bg-hover);
}

.field-name {
  color: var(--text-primary);
  font-weight: 600;
  font-family: "SF Mono", "Fira Code", monospace;
}

.field-args {
  color: var(--text-muted);
  font-family: "SF Mono", "Fira Code", monospace;
  font-size: 11px;
}

.field-type {
  color: var(--accent);
  font-family: "SF Mono", "Fira Code", monospace;
}

.enum-value {
  color: var(--success);
  font-family: "SF Mono", "Fira Code", monospace;
  font-weight: 600;
}

.deprecated {
  background: rgba(255, 152, 0, 0.15);
  color: var(--warning);
  font-size: 9px;
  padding: 1px 4px;
  border-radius: 3px;
  margin-left: 4px;
}

.field-description {
  font-size: 11px;
  color: var(--text-muted);
  padding: 2px 0 2px 16px;
}
</style>
