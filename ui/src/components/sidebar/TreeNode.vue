<script setup lang="ts">
import { ref, nextTick } from "vue";
import { METHOD_COLORS } from "@/utils/http";
import type { TreeItem } from "@/api/types";

export type NodeType = "collection" | "folder" | "request";

const props = defineProps<{
  type: NodeType;
  label: string;
  depth: number;
  expanded?: boolean;
  active?: boolean;
  method?: string;
  requestType?: string;
  /** Children for recursive rendering */
  children?: TreeItem[];
  /** Active request ID for highlighting */
  activeRequestId?: string;
}>();

const emit = defineEmits<{
  toggle: [];
  click: [];
  contextmenu: [event: MouseEvent];
  rename: [name: string];
  "rename-cancel": [];
  /** Bubbled events from recursive children */
  "child-toggle": [item: TreeItem];
  "child-click": [item: TreeItem];
  "child-contextmenu": [event: MouseEvent, item: TreeItem];
  "child-rename": [item: TreeItem, name: string];
  "child-rename-cancel": [];
}>();

const renaming = ref(false);
const renameValue = ref("");
const renameInput = ref<HTMLInputElement | null>(null);

const methodColors = METHOD_COLORS;

function startRename() {
  renaming.value = true;
  renameValue.value = props.label;
  nextTick(() => {
    renameInput.value?.focus();
    renameInput.value?.select();
  });
}

function commitRename() {
  const val = renameValue.value.trim();
  renaming.value = false;
  if (val && val !== props.label) {
    emit("rename", val);
  } else {
    emit("rename-cancel");
  }
}

function cancelRename() {
  renaming.value = false;
  emit("rename-cancel");
}

defineExpose({ startRename });
</script>

<template>
  <div
    class="tree-node"
    :class="{ active }"
    :style="{ paddingLeft: depth * 16 + 8 + 'px' }"
    role="treeitem"
    :aria-expanded="type !== 'request' ? expanded : undefined"
    :aria-selected="active"
    @click="emit('click')"
    @contextmenu.prevent="emit('contextmenu', $event)"
  >
    <button
      v-if="type !== 'request'"
      class="expand-btn"
      @click.stop="emit('toggle')"
      :aria-label="expanded ? 'Collapse' : 'Expand'"
    >
      {{ expanded ? "\u25BE" : "\u25B8" }}
    </button>
    <span v-else class="expand-spacer" />

    <span
      v-if="type === 'request' && requestType === 'websocket'"
      class="ws-badge"
    >WS</span>
    <span
      v-else-if="type === 'request' && requestType === 'graphql'"
      class="gql-badge"
    >GQL</span>
    <span
      v-else-if="type === 'request' && method"
      class="method-badge"
      :style="{ color: methodColors[method] || 'var(--text-secondary)' }"
    >
      {{ method.slice(0, 3) }}
    </span>
    <span v-else-if="type === 'collection'" class="node-icon">&#128193;</span>
    <span v-else class="node-icon">&#128194;</span>

    <input
      v-if="renaming"
      ref="renameInput"
      class="rename-input"
      v-model="renameValue"
      @keydown.enter="commitRename"
      @keydown.escape="cancelRename"
      @blur="commitRename"
      @click.stop
    />
    <span v-else class="node-label" :title="label">{{ label }}</span>
  </div>

  <!-- Recursive children rendering -->
  <template v-if="expanded && children && children.length > 0">
    <div role="group">
      <TreeNode
        v-for="child in children"
        :key="child.id"
        :type="child.type"
        :label="child.label"
        :depth="depth + 1"
        :expanded="false"
        :active="child.type === 'request' && child.id === activeRequestId"
        :method="child.method"
        :request-type="child.requestType"
        @toggle="emit('child-toggle', child)"
        @click="emit('child-click', child)"
        @contextmenu="emit('child-contextmenu', $event, child)"
        @rename="emit('child-rename', child, $event)"
        @rename-cancel="emit('child-rename-cancel')"
        @child-toggle="emit('child-toggle', $event)"
        @child-click="emit('child-click', $event)"
        @child-contextmenu="(ev: MouseEvent, item: TreeItem) => emit('child-contextmenu', ev, item)"
        @child-rename="(item: TreeItem, name: string) => emit('child-rename', item, name)"
        @child-rename-cancel="emit('child-rename-cancel')"
      />
    </div>
  </template>
</template>

<style scoped>
.tree-node {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 8px;
  cursor: pointer;
  border-radius: var(--radius-sm);
  min-height: 28px;
  user-select: none;
  transition: background var(--transition);
}

.tree-node:hover {
  background: var(--bg-hover);
}

.tree-node.active {
  background: var(--accent-muted);
}

.expand-btn {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 10px;
  cursor: pointer;
  padding: 0;
  width: 14px;
  flex-shrink: 0;
  text-align: center;
  transition: transform var(--transition);
}

.expand-spacer {
  width: 14px;
  flex-shrink: 0;
}

.method-badge {
  font-size: 10px;
  font-weight: 700;
  flex-shrink: 0;
  width: 28px;
  text-align: center;
}

.ws-badge {
  font-size: 9px;
  font-weight: 700;
  background: var(--accent);
  color: #fff;
  padding: 1px 4px;
  border-radius: 3px;
  flex-shrink: 0;
}

.gql-badge {
  font-size: 9px;
  font-weight: 700;
  background: #e535ab;
  color: #fff;
  padding: 1px 4px;
  border-radius: 3px;
  flex-shrink: 0;
}

.node-icon {
  font-size: 12px;
  flex-shrink: 0;
  width: 18px;
  text-align: center;
}

.node-label {
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.rename-input {
  flex: 1;
  background: var(--bg-input);
  border: 1px solid var(--accent);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 13px;
  padding: 1px 4px;
  outline: none;
}
</style>
