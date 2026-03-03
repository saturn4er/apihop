<script setup lang="ts">
import { ref, onMounted } from "vue";
import BaseDialog from "./BaseDialog.vue";
import AuthEditor from "./AuthEditor.vue";
import { getCollection, updateCollection, type AuthConfig, type Collection } from "@/api/client";

const props = defineProps<{
  collectionId: string;
}>();

const emit = defineEmits<{
  close: [];
  saved: [collection: Collection];
}>();

const loading = ref(true);
const saving = ref(false);
const collectionName = ref("");
const collectionDescription = ref<string | undefined>(undefined);
const auth = ref<AuthConfig>({ type: "none" });

onMounted(async () => {
  try {
    const col = await getCollection(props.collectionId);
    collectionName.value = col.name;
    collectionDescription.value = col.description;
    auth.value = col.auth || { type: "none" };
  } finally {
    loading.value = false;
  }
});

async function save() {
  saving.value = true;
  try {
    const updated = await updateCollection(
      props.collectionId,
      collectionName.value,
      collectionDescription.value,
      auth.value,
    );
    emit("saved", updated);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <BaseDialog title="Collection Auth" width="480px" :show-footer="!loading" @close="emit('close')">
    <div v-if="loading" class="dialog-loading">Loading...</div>

    <template v-else>
      <div class="dialog-subtitle">{{ collectionName }}</div>
      <AuthEditor v-model="auth" :show-inherit-option="false" />
    </template>

    <template #footer>
      <button class="btn btn-secondary" @click="emit('close')">Cancel</button>
      <button class="btn btn-primary" :disabled="saving" @click="save">
        {{ saving ? "Saving..." : "Save" }}
      </button>
    </template>
  </BaseDialog>
</template>

<style scoped>
.dialog-loading {
  padding: 32px;
  text-align: center;
  color: var(--text-muted);
}

.dialog-subtitle {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.btn {
  padding: 7px 16px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  cursor: pointer;
  border: none;
  transition: all var(--transition);
}

.btn-secondary {
  background: var(--bg-tertiary);
  color: var(--text-secondary);
}

.btn-secondary:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.btn-primary {
  background: var(--accent);
  color: white;
}

.btn-primary:hover {
  background: var(--accent-hover);
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
