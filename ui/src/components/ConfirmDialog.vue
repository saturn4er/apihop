<script setup lang="ts">
import BaseDialog from "./BaseDialog.vue";

defineProps<{
  title?: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
}>();

const emit = defineEmits<{
  confirm: [];
  cancel: [];
}>();
</script>

<template>
  <BaseDialog :title="title || 'Confirm'" width="400px" show-footer @close="emit('cancel')">
    <p class="confirm-message">{{ message }}</p>
    <template #footer>
      <button class="cancel-btn" @click="emit('cancel')">{{ cancelLabel || "Cancel" }}</button>
      <button
        class="confirm-btn"
        :class="{ danger }"
        @click="emit('confirm')"
      >
        {{ confirmLabel || "Confirm" }}
      </button>
    </template>
  </BaseDialog>
</template>

<style scoped>
.confirm-message {
  margin: 0;
  font-size: 14px;
  color: var(--text-primary);
  line-height: 1.5;
}

.cancel-btn {
  background: none;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  padding: 7px 18px;
  font-size: 13px;
  cursor: pointer;
  transition: all var(--transition);
}

.cancel-btn:hover {
  color: var(--text-primary);
  border-color: var(--text-secondary);
}

.confirm-btn {
  background: var(--accent);
  border: none;
  border-radius: var(--radius-sm);
  color: #fff;
  padding: 7px 18px;
  font-size: 13px;
  cursor: pointer;
  transition: all var(--transition);
}

.confirm-btn:hover {
  background: var(--accent-hover);
}

.confirm-btn.danger {
  background: var(--error);
}

.confirm-btn.danger:hover {
  background: #e53935;
}
</style>
