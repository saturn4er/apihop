<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from "vue";
import type { MenuItem } from "@/api/types";

const props = defineProps<{
  x: number;
  y: number;
  items: MenuItem[];
}>();

const emit = defineEmits<{
  select: [action: string];
  close: [];
}>();

const menuRef = ref<HTMLElement | null>(null);
const focusedIndex = ref(-1);

const actionItems = props.items.filter((item) => !item.separator);

function focusItem(index: number) {
  focusedIndex.value = index;
  const items = menuRef.value?.querySelectorAll('[role="menuitem"]');
  if (items && items[index]) {
    (items[index] as HTMLElement).focus();
  }
}

function onClickOutside() {
  emit("close");
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    emit("close");
  } else if (e.key === "ArrowDown") {
    e.preventDefault();
    const next = focusedIndex.value < actionItems.length - 1 ? focusedIndex.value + 1 : 0;
    focusItem(next);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    const prev = focusedIndex.value > 0 ? focusedIndex.value - 1 : actionItems.length - 1;
    focusItem(prev);
  } else if (e.key === "Enter" || e.key === " ") {
    e.preventDefault();
    if (focusedIndex.value >= 0 && focusedIndex.value < actionItems.length) {
      emit("select", actionItems[focusedIndex.value].action);
    }
  }
}

onMounted(() => {
  setTimeout(() => {
    document.addEventListener("click", onClickOutside);
  }, 0);
  document.addEventListener("keydown", onKeydown);
  nextTick(() => {
    if (actionItems.length > 0) {
      focusItem(0);
    }
  });
});

onUnmounted(() => {
  document.removeEventListener("click", onClickOutside);
  document.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div
    ref="menuRef"
    class="context-menu"
    role="menu"
    :style="{ left: x + 'px', top: y + 'px' }"
  >
    <template v-for="(item, i) in items" :key="i">
      <div v-if="item.separator" class="separator" role="separator" />
      <button
        v-else
        class="menu-item"
        :class="{ danger: item.danger }"
        role="menuitem"
        tabindex="-1"
        @click.stop="emit('select', item.action)"
      >
        {{ item.label }}
      </button>
    </template>
  </div>
</template>

<style scoped>
.context-menu {
  position: fixed;
  z-index: 1000;
  background: var(--bg-surface);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  padding: 4px;
  min-width: 160px;
  box-shadow: var(--shadow-lg);
  backdrop-filter: blur(8px);
}

.menu-item {
  display: block;
  width: 100%;
  background: none;
  border: none;
  padding: 7px 12px;
  color: var(--text-primary);
  font-size: 13px;
  text-align: left;
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: background var(--transition);
}

.menu-item:hover,
.menu-item:focus {
  background: var(--bg-hover);
  outline: none;
}

.menu-item.danger:hover,
.menu-item.danger:focus {
  background: rgba(244, 67, 54, 0.12);
  color: var(--error);
}

.menu-item.danger {
  color: var(--error);
}

.separator {
  height: 1px;
  background: var(--border-subtle);
  margin: 4px 0;
}
</style>
