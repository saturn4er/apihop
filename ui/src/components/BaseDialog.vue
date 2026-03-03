<script setup lang="ts">
import { onMounted, onUnmounted, useId } from "vue";

const props = withDefaults(
  defineProps<{
    title: string;
    width?: string;
    showFooter?: boolean;
  }>(),
  {
    width: "480px",
    showFooter: false,
  }
);

const emit = defineEmits<{
  close: [];
}>();

const titleId = useId();

function onOverlayClick(e: MouseEvent) {
  if ((e.target as HTMLElement).classList.contains("base-dialog-overlay")) {
    emit("close");
  }
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    emit("close");
  }
}

onMounted(() => {
  document.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  document.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <Teleport to="body">
    <div
      class="base-dialog-overlay"
      role="dialog"
      aria-modal="true"
      :aria-labelledby="titleId"
      @click="onOverlayClick"
    >
      <div class="base-dialog" :style="{ width }">
        <div class="base-dialog-header">
          <h3 :id="titleId" class="base-dialog-title">{{ title }}</h3>
          <button class="base-dialog-close" @click="emit('close')">&times;</button>
        </div>
        <div class="base-dialog-body">
          <slot />
        </div>
        <div v-if="showFooter || $slots.footer" class="base-dialog-footer">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.base-dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.base-dialog {
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  max-width: 90vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.base-dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border-subtle);
}

.base-dialog-title {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
}

.base-dialog-close {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 20px;
  cursor: pointer;
  padding: 0 4px;
  line-height: 1;
  border-radius: var(--radius-sm);
  transition: all var(--transition);
}

.base-dialog-close:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.base-dialog-body {
  padding: 16px 20px;
  overflow: auto;
  flex: 1;
}

.base-dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 20px;
  border-top: 1px solid var(--border-subtle);
}
</style>
