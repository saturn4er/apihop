<script setup lang="ts">
import { ref, computed, watch } from "vue";
import GraphQLFieldRow from "./GraphQLFieldRow.vue";
import type { GraphQLSchema, GraphQLType, GraphQLField, GraphQLTypeRef } from "@/api/types";

const props = defineProps<{
  schema: GraphQLSchema;
  modelValue: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const selectedFields = ref<Set<string>>(new Set());
const expandedNodes = ref<Set<string>>(new Set());
const argValues = ref<Map<string, string>>(new Map());

const activeRoot = ref<"query" | "mutation" | "subscription">("query");

const rootTypeName = computed(() => {
  if (activeRoot.value === "mutation") return props.schema.mutation_type;
  if (activeRoot.value === "subscription") return props.schema.subscription_type;
  return props.schema.query_type;
});

const rootType = computed(() => {
  const name = rootTypeName.value;
  if (!name) return null;
  return props.schema.types.find((t) => t.name === name) ?? null;
});

const rootFields = computed(() =>
  rootType.value?.fields?.filter((f) => !f.name.startsWith("__")) ?? []
);

const availableRoots = computed(() => {
  const roots: { key: "query" | "mutation" | "subscription"; label: string }[] = [];
  if (props.schema.query_type) roots.push({ key: "query", label: "Query" });
  if (props.schema.mutation_type) roots.push({ key: "mutation", label: "Mutation" });
  if (props.schema.subscription_type) roots.push({ key: "subscription", label: "Subscription" });
  return roots;
});

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

function toggleField(path: string, field: GraphQLField) {
  if (selectedFields.value.has(path)) {
    // Deselect this field and all children
    const toRemove: string[] = [];
    for (const key of selectedFields.value) {
      if (key === path || key.startsWith(path + ".")) {
        toRemove.push(key);
      }
    }
    toRemove.forEach((k) => selectedFields.value.delete(k));
    for (const key of argValues.value.keys()) {
      if (key.startsWith(path + ":") || key.startsWith(path + ".")) {
        argValues.value.delete(key);
      }
    }
  } else {
    selectedFields.value.add(path);
    // If it's an object type, auto-expand and select first scalar
    if (isObjectType(field.type)) {
      expandedNodes.value.add(path);
      const subType = getFieldType(field.type);
      if (subType?.fields) {
        const firstScalar = subType.fields.find(
          (f) => !f.name.startsWith("__") && !isObjectType(f.type)
        );
        if (firstScalar) {
          selectedFields.value.add(path + "." + firstScalar.name);
        }
      }
    }
  }
  generateQuery();
}

function toggleExpand(path: string) {
  if (expandedNodes.value.has(path)) {
    expandedNodes.value.delete(path);
  } else {
    expandedNodes.value.add(path);
  }
}

function onArgChange(path: string, argName: string, value: string) {
  const key = path + ":" + argName;
  if (value) {
    argValues.value.set(key, value);
  } else {
    argValues.value.delete(key);
  }
  generateQuery();
}

function generateQuery() {
  if (!rootType.value || !rootTypeName.value) {
    emit("update:modelValue", "");
    return;
  }

  const prefix =
    activeRoot.value === "mutation"
      ? "mutation"
      : activeRoot.value === "subscription"
        ? "subscription"
        : "query";

  const body = buildSelectionSet(rootTypeName.value, rootType.value);
  if (!body.trim()) {
    emit("update:modelValue", "{\n  \n}");
    return;
  }

  emit("update:modelValue", `${prefix} {\n${body}}`);
}

function buildSelectionSet(parentPath: string, type: GraphQLType, indent: number = 1): string {
  if (!type.fields) return "";

  const lines: string[] = [];
  const pad = "  ".repeat(indent);

  for (const field of type.fields) {
    if (field.name.startsWith("__")) continue;
    const fieldPath = parentPath + "." + field.name;
    if (!selectedFields.value.has(fieldPath)) continue;

    let argsStr = "";
    if (field.args.length > 0) {
      const argParts: string[] = [];
      for (const arg of field.args) {
        const val = argValues.value.get(fieldPath + ":" + arg.name) || "";
        if (val) {
          argParts.push(`${arg.name}: ${formatArgValue(val, arg.type)}`);
        }
      }
      if (argParts.length > 0) {
        argsStr = `(${argParts.join(", ")})`;
      }
    }

    const subType = getFieldType(field.type);
    if (subType && isObjectType(field.type)) {
      const subBody = buildSelectionSet(fieldPath, subType, indent + 1);
      if (subBody.trim()) {
        lines.push(`${pad}${field.name}${argsStr} {\n${subBody}${pad}}`);
      } else {
        lines.push(`${pad}${field.name}${argsStr} {\n${pad}  __typename\n${pad}}`);
      }
    } else {
      lines.push(`${pad}${field.name}${argsStr}`);
    }
  }

  return lines.length > 0 ? lines.join("\n") + "\n" : "";
}

function formatArgValue(value: string, typeRef: GraphQLTypeRef): string {
  const baseName = resolveTypeName(typeRef);
  if (value.startsWith("{") || value.startsWith("[")) return value;
  if (/^-?\d+(\.\d+)?$/.test(value)) return value;
  if (value === "true" || value === "false") return value;
  if (value === "null") return value;
  const resolvedType = props.schema.types.find((t) => t.name === baseName);
  if (resolvedType?.kind === "ENUM") return value;
  return `"${value.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}

function clearAll() {
  selectedFields.value.clear();
  expandedNodes.value.clear();
  argValues.value.clear();
  generateQuery();
}

watch(activeRoot, () => {
  generateQuery();
});
</script>

<template>
  <div class="query-builder">
    <div class="builder-header">
      <div v-if="availableRoots.length > 1" class="root-tabs">
        <button
          v-for="root in availableRoots"
          :key="root.key"
          class="root-tab"
          :class="{ active: activeRoot === root.key }"
          @click="activeRoot = root.key"
        >
          {{ root.label }}
        </button>
      </div>
      <button v-if="selectedFields.size > 0" class="clear-btn" @click="clearAll">Clear</button>
    </div>

    <div v-if="!rootType" class="empty-state">
      No {{ activeRoot }} type found in schema
    </div>

    <div v-else class="field-tree">
      <GraphQLFieldRow
        v-for="field in rootFields"
        :key="field.name"
        :field="field"
        :path="(rootTypeName || '') + '.' + field.name"
        :schema="schema"
        :selected-fields="selectedFields"
        :expanded-nodes="expandedNodes"
        :arg-values="argValues"
        :depth="0"
        @toggle="toggleField"
        @toggle-expand="toggleExpand"
        @arg-change="onArgChange"
      />
    </div>
  </div>
</template>

<style scoped>
.query-builder {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.builder-header {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
}

.root-tabs {
  display: flex;
  gap: 0;
}

.root-tab {
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  padding: 6px 16px;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition);
}

.root-tab:hover {
  color: var(--text-primary);
}

.root-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.clear-btn {
  margin-left: auto;
  background: none;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 11px;
  padding: 3px 10px;
  cursor: pointer;
  transition: all var(--transition);
}

.clear-btn:hover {
  color: var(--error);
  border-color: var(--error);
}

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 80px;
  color: var(--text-muted);
  font-size: 13px;
}

.field-tree {
  flex: 1;
  overflow: auto;
  padding: 8px 0;
}
</style>
