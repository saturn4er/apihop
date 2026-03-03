<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import BaseDialog from "../BaseDialog.vue";
import {
  listCollections,
  listFolders,
  createCollection,
  type Collection,
  type Folder,
} from "@/api/client";

const props = defineProps<{
  initialName?: string;
  initialCollectionId?: string;
  initialFolderId?: string;
}>();

const emit = defineEmits<{
  save: [data: { name: string; collectionId: string; folderId?: string }];
  cancel: [];
}>();

const name = ref(props.initialName || "");
const collections = ref<Collection[]>([]);
const folders = ref<Folder[]>([]);
const selectedCollectionId = ref(props.initialCollectionId || "");
const selectedFolderId = ref(props.initialFolderId || "");

const creatingCollection = ref(false);
const newCollectionName = ref("");

const nameInput = ref<HTMLInputElement | null>(null);

onMounted(async () => {
  collections.value = await listCollections();
  if (props.initialCollectionId) {
    selectedCollectionId.value = props.initialCollectionId;
  } else if (collections.value.length > 0) {
    selectedCollectionId.value = collections.value[0].id;
  }
  nameInput.value?.focus();
});

watch(selectedCollectionId, async (id) => {
  if (!id) {
    folders.value = [];
    selectedFolderId.value = "";
    return;
  }
  folders.value = await listFolders(id);
  if (!folders.value.find((f) => f.id === selectedFolderId.value)) {
    selectedFolderId.value = "";
  }
});

async function onCreateCollection() {
  const cname = newCollectionName.value.trim();
  if (!cname) return;
  const col = await createCollection(cname);
  collections.value.push(col);
  selectedCollectionId.value = col.id;
  creatingCollection.value = false;
  newCollectionName.value = "";
}

function onSave() {
  if (!name.value.trim() || !selectedCollectionId.value) return;
  emit("save", {
    name: name.value.trim(),
    collectionId: selectedCollectionId.value,
    folderId: selectedFolderId.value || undefined,
  });
}
</script>

<template>
  <BaseDialog title="Save Request" width="380px" @close="emit('cancel')">
    <label class="field-label">Name</label>
    <input
      ref="nameInput"
      v-model="name"
      class="field-input"
      placeholder="Request name..."
      @keydown.enter="onSave"
    />

    <label class="field-label">Collection</label>
    <div class="collection-row">
      <select v-model="selectedCollectionId" class="field-select">
        <option value="" disabled>Select collection...</option>
        <option v-for="c in collections" :key="c.id" :value="c.id">{{ c.name }}</option>
      </select>
      <button class="icon-btn" @click="creatingCollection = !creatingCollection" title="New collection">
        +
      </button>
    </div>

    <div v-if="creatingCollection" class="new-collection-row">
      <input
        v-model="newCollectionName"
        class="field-input"
        placeholder="New collection name..."
        @keydown.enter="onCreateCollection"
      />
      <button class="small-btn" @click="onCreateCollection">Create</button>
    </div>

    <label v-if="folders.length > 0" class="field-label">Folder (optional)</label>
    <select v-if="folders.length > 0" v-model="selectedFolderId" class="field-select">
      <option value="">Root (no folder)</option>
      <option v-for="f in folders" :key="f.id" :value="f.id">{{ f.name }}</option>
    </select>

    <div class="dialog-actions">
      <button class="btn-cancel" @click="emit('cancel')">Cancel</button>
      <button
        class="btn-save"
        :disabled="!name.trim() || !selectedCollectionId"
        @click="onSave"
      >
        Save
      </button>
    </div>
  </BaseDialog>
</template>

<style scoped>
.field-label {
  display: block;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 4px;
  margin-top: 12px;
}

.field-input {
  width: 100%;
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 13px;
  padding: 7px 10px;
  outline: none;
  transition: border-color var(--transition), box-shadow var(--transition);
}

.field-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-muted);
}

.field-select {
  width: 100%;
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 13px;
  padding: 7px 10px;
  outline: none;
  transition: border-color var(--transition);
}

.field-select:focus {
  border-color: var(--accent);
}

.field-select option {
  background: var(--bg-secondary);
}

.collection-row {
  display: flex;
  gap: 6px;
}

.collection-row .field-select {
  flex: 1;
}

.icon-btn {
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 16px;
  width: 34px;
  cursor: pointer;
  transition: all var(--transition);
}

.icon-btn:hover {
  border-color: var(--accent);
  background: var(--accent-muted);
}

.new-collection-row {
  display: flex;
  gap: 6px;
  margin-top: 6px;
}

.new-collection-row .field-input {
  flex: 1;
}

.small-btn {
  background: var(--accent);
  border: none;
  border-radius: var(--radius-sm);
  color: #fff;
  font-size: 12px;
  padding: 0 12px;
  cursor: pointer;
  transition: background var(--transition);
}

.small-btn:hover {
  background: var(--accent-hover);
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 20px;
}

.btn-cancel {
  background: transparent;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  padding: 7px 16px;
  font-size: 13px;
  cursor: pointer;
  transition: all var(--transition);
}

.btn-cancel:hover {
  color: var(--text-primary);
  border-color: var(--text-secondary);
}

.btn-save {
  background: var(--accent);
  border: none;
  border-radius: var(--radius-sm);
  color: #fff;
  padding: 7px 20px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: background var(--transition);
}

.btn-save:hover:not(:disabled) {
  background: var(--accent-hover);
}

.btn-save:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
