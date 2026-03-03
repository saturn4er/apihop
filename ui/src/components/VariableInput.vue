<script setup lang="ts">
import { computed, ref, nextTick } from "vue";

const props = withDefaults(
  defineProps<{
    modelValue: string;
    placeholder?: string;
    variableNames?: string[];
  }>(),
  {
    placeholder: "",
    variableNames: () => [],
  }
);

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const inputEl = ref<HTMLInputElement | null>(null);
const showDropdown = ref(false);
const dropdownPos = ref({ top: 0, left: 0 });
const selectedIndex = ref(0);

const dynamicVariables = [
  { name: "{{$timestamp}}", description: "Unix timestamp", isDynamic: true },
  { name: "{{$isoTimestamp}}", description: "ISO 8601 timestamp", isDynamic: true },
  { name: "{{$randomUUID}}", description: "Random UUID v4", isDynamic: true },
  { name: "{{$randomInt}}", description: "Random integer", isDynamic: true },
  { name: "{{$randomEmail}}", description: "Random email", isDynamic: true },
  { name: "{{$randomName}}", description: "Random name", isDynamic: true },
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
  if (!showDropdown.value) return [];
  const input = inputEl.value;
  if (!input) return allVariables.value;
  const pos = input.selectionStart ?? 0;
  const textBefore = props.modelValue.slice(0, pos);
  const match = textBefore.match(/\{\{(\$?\w*)$/);
  if (!match) return allVariables.value;
  const filter = match[1].toLowerCase();
  if (!filter) return allVariables.value;
  return allVariables.value.filter((v) =>
    v.name.toLowerCase().includes(filter)
  );
});

function onInput(e: Event) {
  const val = (e.target as HTMLInputElement).value;
  emit("update:modelValue", val);
  nextTick(() => checkTrigger());
}

function checkTrigger() {
  const input = inputEl.value;
  if (!input) return;
  const pos = input.selectionStart ?? 0;
  const textBefore = props.modelValue.slice(0, pos);
  if (/\{\{\$?\w*$/.test(textBefore)) {
    const rect = input.getBoundingClientRect();
    const charWidth = 7.8;
    dropdownPos.value = {
      top: rect.bottom + 2,
      left: Math.min(rect.left + pos * charWidth, window.innerWidth - 300),
    };
    showDropdown.value = true;
    selectedIndex.value = 0;
  } else {
    showDropdown.value = false;
  }
}

function insertVariable(varName: string) {
  const input = inputEl.value;
  if (!input) return;
  const pos = input.selectionStart ?? 0;
  const textBefore = props.modelValue.slice(0, pos);
  const match = textBefore.match(/\{\{\$?\w*$/);
  if (!match) return;
  const start = pos - match[0].length;
  const newVal = props.modelValue.slice(0, start) + varName + props.modelValue.slice(pos);
  emit("update:modelValue", newVal);
  showDropdown.value = false;
  nextTick(() => {
    const newPos = start + varName.length;
    input.setSelectionRange(newPos, newPos);
    input.focus();
  });
}

function onKeydown(e: KeyboardEvent) {
  if (!showDropdown.value || filteredVars.value.length === 0) return;
  if (e.key === "ArrowDown") {
    e.preventDefault();
    selectedIndex.value = (selectedIndex.value + 1) % filteredVars.value.length;
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    selectedIndex.value = (selectedIndex.value - 1 + filteredVars.value.length) % filteredVars.value.length;
  } else if (e.key === "Enter" || e.key === "Tab") {
    e.preventDefault();
    insertVariable(filteredVars.value[selectedIndex.value].name);
  } else if (e.key === "Escape") {
    showDropdown.value = false;
  }
}

function onBlur() {
  if (showDropdown.value) {
    setTimeout(() => { showDropdown.value = false; }, 150);
  }
}

function focus() {
  inputEl.value?.focus();
}

defineExpose({ focus });
</script>

<template>
  <input
    ref="inputEl"
    type="text"
    :placeholder="placeholder"
    :value="modelValue"
    @input="onInput"
    @keydown="onKeydown"
    @blur="onBlur"
  />
  <Teleport to="body">
    <div
      v-if="showDropdown && filteredVars.length > 0"
      class="var-dropdown"
      :style="{ top: dropdownPos.top + 'px', left: dropdownPos.left + 'px' }"
    >
      <div
        v-for="(v, i) in filteredVars"
        :key="v.name"
        class="var-option"
        :class="{ selected: i === selectedIndex }"
        @mousedown.prevent="insertVariable(v.name)"
      >
        <span class="var-name" :class="{ 'var-env': !v.isDynamic }">{{ v.name }}</span>
        <span class="var-desc">{{ v.description }}</span>
      </div>
    </div>
  </Teleport>
</template>
