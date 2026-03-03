<script setup lang="ts">
import { ref, onMounted } from "vue";
import {
  listEnvironments,
  type Environment,
} from "@/api/client";

const props = defineProps<{
  activeEnvironmentId?: string;
}>();

const emit = defineEmits<{
  "update:activeEnvironmentId": [id: string | undefined];
  "open-manage": [];
}>();

const environments = ref<Environment[]>([]);
const loading = ref(false);

onMounted(async () => {
  await refresh();
});

async function refresh() {
  loading.value = true;
  try {
    environments.value = await listEnvironments();
  } finally {
    loading.value = false;
  }
}

function onSelect(e: Event) {
  const val = (e.target as HTMLSelectElement).value;
  emit("update:activeEnvironmentId", val || undefined);
}

defineExpose({ refresh });
</script>

<template>
  <div class="env-selector">
    <select
      class="env-select"
      :value="activeEnvironmentId || ''"
      :disabled="loading"
      @change="onSelect"
    >
      <option value="">{{ loading ? "Loading..." : "No Environment" }}</option>
      <option v-for="env in environments" :key="env.id" :value="env.id">
        {{ env.name }}
      </option>
    </select>
    <button class="manage-btn" @click="emit('open-manage')" title="Manage Environments">
      &#9881;
    </button>
  </div>
</template>

<style scoped>
.env-selector {
  display: flex;
  gap: 4px;
  align-items: center;
}

.env-select {
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 12px;
  padding: 4px 24px 4px 8px;
  outline: none;
  max-width: 160px;
  appearance: none;
  -webkit-appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='5'%3E%3Cpath d='M0 0l4 5 4-5z' fill='%238888A0'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  transition: border-color var(--transition);
}

.env-select:focus {
  border-color: var(--accent);
}

.env-select option {
  background: var(--bg-secondary);
}

.manage-btn {
  background: none;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 14px;
  padding: 2px 6px;
  cursor: pointer;
  line-height: 1;
  transition: all var(--transition);
}

.manage-btn:hover {
  color: var(--accent);
  border-color: var(--accent);
}
</style>
