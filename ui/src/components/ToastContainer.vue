<script setup lang="ts">
import { useToast } from "@/composables/useToast";

const { toasts, removeToast } = useToast();
</script>

<template>
  <div class="toast-container">
    <TransitionGroup name="toast">
      <div
        v-for="toast in toasts"
        :key="toast.id"
        class="toast"
        :class="`toast-${toast.type}`"
      >
        <span class="toast-icon">
          <template v-if="toast.type === 'success'">&#10003;</template>
          <template v-else-if="toast.type === 'error'">&#10007;</template>
          <template v-else-if="toast.type === 'warning'">&#9888;</template>
          <template v-else>&#8505;</template>
        </span>
        <span class="toast-message">{{ toast.message }}</span>
        <button class="toast-close" @click="removeToast(toast.id)">&times;</button>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toast-container {
  position: fixed;
  bottom: 20px;
  right: 20px;
  z-index: 9999;
  display: flex;
  flex-direction: column-reverse;
  gap: 8px;
  pointer-events: none;
}

.toast {
  pointer-events: auto;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  box-shadow: var(--shadow-md);
  font-size: 13px;
  color: var(--text-primary);
  min-width: 220px;
  max-width: 380px;
}

.toast-success {
  border-left: 3px solid var(--success);
}

.toast-error {
  border-left: 3px solid var(--error);
}

.toast-warning {
  border-left: 3px solid var(--warning);
}

.toast-info {
  border-left: 3px solid var(--info);
}

.toast-icon {
  flex-shrink: 0;
  font-size: 14px;
}

.toast-success .toast-icon { color: var(--success); }
.toast-error .toast-icon { color: var(--error); }
.toast-warning .toast-icon { color: var(--warning); }
.toast-info .toast-icon { color: var(--info); }

.toast-message {
  flex: 1;
}

.toast-close {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 16px;
  cursor: pointer;
  padding: 0 2px;
  line-height: 1;
  flex-shrink: 0;
}

.toast-close:hover {
  color: var(--text-primary);
}

.toast-enter-active {
  transition: all 0.3s ease;
}

.toast-leave-active {
  transition: all 0.2s ease;
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(40px);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(40px);
}
</style>
