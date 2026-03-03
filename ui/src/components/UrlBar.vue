<script setup lang="ts">
import { computed, ref, nextTick } from "vue";
import { METHOD_COLORS } from "@/utils/http";

const props = withDefaults(
  defineProps<{
    method: string;
    url: string;
    loading: boolean;
    variableNames?: string[];
  }>(),
  { variableNames: () => [] }
);

const emit = defineEmits<{
  "update:method": [value: string];
  "update:url": [value: string];
  send: [];
}>();

const methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] as const;

const methodColor = computed(() => {
  return METHOD_COLORS[props.method] || "var(--text-primary)";
});

const urlInput = ref<HTMLInputElement | null>(null);
const showVarDropdown = ref(false);
const varDropdownPos = ref({ top: 0, left: 0 });
const selectedVarIndex = ref(0);

const dynamicVariables = [
  { name: "{{$timestamp}}", description: "Unix timestamp (seconds)", isDynamic: true },
  { name: "{{$isoTimestamp}}", description: "ISO 8601 timestamp", isDynamic: true },
  { name: "{{$randomUUID}}", description: "Random UUID v4", isDynamic: true },
  { name: "{{$randomInt}}", description: "Random integer 0-1000", isDynamic: true },
  { name: "{{$randomEmail}}", description: "Random email address", isDynamic: true },
  { name: "{{$randomName}}", description: "Random full name", isDynamic: true },
];

const allVariables = computed(() => {
  const envVars = props.variableNames.map((name) => ({
    name: `{{${name}}}`,
    description: "Environment variable",
    isDynamic: false,
  }));
  return [...dynamicVariables, ...envVars];
});

const filteredVars = computed(() => {
  if (!showVarDropdown.value) return [];
  const input = urlInput.value;
  if (!input) return allVariables.value;
  const pos = input.selectionStart ?? 0;
  const textBefore = props.url.slice(0, pos);
  const match = textBefore.match(/\{\{(\$?\w*)$/);
  if (!match) return allVariables.value;
  const filter = match[1].toLowerCase();
  if (!filter) return allVariables.value;
  return allVariables.value.filter((v) =>
    v.name.toLowerCase().includes(filter)
  );
});

function focus() {
  urlInput.value?.focus();
}

function onInput(e: Event) {
  const val = (e.target as HTMLInputElement).value;
  emit("update:url", val);
  nextTick(() => checkForVariableTrigger());
}

function checkForVariableTrigger() {
  const input = urlInput.value;
  if (!input) return;
  const pos = input.selectionStart ?? 0;
  const textBefore = props.url.slice(0, pos);
  if (/\{\{\$?\w*$/.test(textBefore)) {
    // Position dropdown relative to viewport for Teleport
    const rect = input.getBoundingClientRect();
    const charWidth = 8.4; // approx monospace char width at 14px
    varDropdownPos.value = {
      top: rect.bottom + 2,
      left: rect.left + pos * charWidth,
    };
    showVarDropdown.value = true;
    selectedVarIndex.value = 0;
  } else {
    showVarDropdown.value = false;
  }
}

function insertVariable(varName: string) {
  const input = urlInput.value;
  if (!input) return;
  const pos = input.selectionStart ?? 0;
  const textBefore = props.url.slice(0, pos);
  const match = textBefore.match(/\{\{\$?\w*$/);
  if (!match) return;
  const start = pos - match[0].length;
  const newUrl = props.url.slice(0, start) + varName + props.url.slice(pos);
  emit("update:url", newUrl);
  showVarDropdown.value = false;
  nextTick(() => {
    const newPos = start + varName.length;
    input.setSelectionRange(newPos, newPos);
    input.focus();
  });
}

function onKeydown(e: KeyboardEvent) {
  if (showVarDropdown.value && filteredVars.value.length > 0) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      selectedVarIndex.value = (selectedVarIndex.value + 1) % filteredVars.value.length;
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      selectedVarIndex.value = (selectedVarIndex.value - 1 + filteredVars.value.length) % filteredVars.value.length;
      return;
    }
    if (e.key === "Enter" || e.key === "Tab") {
      e.preventDefault();
      insertVariable(filteredVars.value[selectedVarIndex.value].name);
      return;
    }
    if (e.key === "Escape") {
      showVarDropdown.value = false;
      return;
    }
  }
  if (e.key === "Enter") {
    emit("send");
  }
}

function onBlur() {
  if (showVarDropdown.value) {
    setTimeout(() => { showVarDropdown.value = false; }, 150);
  }
}

defineExpose({ focus });
</script>

<template>
  <div class="url-bar">
    <select
      class="method-select"
      :value="method"
      :style="{ color: methodColor }"
      @change="emit('update:method', ($event.target as HTMLSelectElement).value)"
    >
      <option v-for="m in methods" :key="m" :value="m">{{ m }}</option>
    </select>
    <div class="url-input-wrapper">
      <input
        ref="urlInput"
        class="url-input"
        type="text"
        placeholder="Enter URL..."
        :value="url"
        @input="onInput"
        @keydown="onKeydown"
        @blur="onBlur"
      />
    </div>
    <Teleport to="body">
      <div
        v-if="showVarDropdown && filteredVars.length > 0"
        class="var-dropdown"
        :style="{ top: varDropdownPos.top + 'px', left: varDropdownPos.left + 'px' }"
      >
        <div
          v-for="(v, i) in filteredVars"
          :key="v.name"
          class="var-option"
          :class="{ selected: i === selectedVarIndex }"
          @mousedown.prevent="insertVariable(v.name)"
        >
          <span class="var-name" :class="{ 'var-env': !v.isDynamic }">{{ v.name }}</span>
          <span class="var-desc">{{ v.description }}</span>
        </div>
      </div>
    </Teleport>
    <button class="send-btn" :disabled="loading" @click="emit('send')">
      <span v-if="loading" class="spinner"></span>
      <span v-else>Send</span>
    </button>
  </div>
</template>

<style scoped>
.url-bar {
  display: flex;
  gap: 0;
  border: 1.5px solid var(--border-color);
  border-radius: var(--radius-md);
  overflow: hidden;
  background: var(--bg-input);
  transition: border-color var(--transition);
}

.url-bar:focus-within {
  border-color: var(--accent);
}

.method-select {
  background: var(--bg-secondary);
  border: none;
  border-right: 1px solid var(--border-color);
  padding: 10px 12px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  outline: none;
  min-width: 100px;
  appearance: none;
  -webkit-appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6'%3E%3Cpath d='M0 0l5 6 5-6z' fill='%238888A0'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 10px center;
  padding-right: 28px;
  border-radius: var(--radius-sm) 0 0 var(--radius-sm);
}

.method-select option {
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.url-input-wrapper {
  flex: 1;
  position: relative;
}

.url-input {
  width: 100%;
  background: transparent;
  border: none;
  padding: 10px 14px;
  color: var(--text-primary);
  font-size: 14px;
  outline: none;
}

.url-input::placeholder {
  color: var(--text-muted);
}


.send-btn {
  background: var(--accent);
  border: none;
  color: #fff;
  padding: 10px 24px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition);
  min-width: 80px;
  border-radius: 0 var(--radius-md) var(--radius-md) 0;
}

.send-btn:hover:not(:disabled) {
  background: var(--accent-hover);
  box-shadow: var(--shadow-sm);
}

.send-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.spinner {
  display: inline-block;
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>

<style>
.var-dropdown {
  position: fixed;
  z-index: 10000;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-lg, 0 8px 24px rgba(0, 0, 0, 0.3));
  min-width: 280px;
  max-height: 200px;
  overflow-y: auto;
  padding: 4px;
}

.var-option {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background var(--transition);
}

.var-option:hover,
.var-option.selected {
  background: var(--bg-hover);
}

.var-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--accent);
  font-family: "SF Mono", "Fira Code", monospace;
  white-space: nowrap;
}

.var-name.var-env {
  color: var(--warning, #e2b93d);
}

.var-desc {
  font-size: 11px;
  color: var(--text-muted);
  white-space: nowrap;
}
</style>
