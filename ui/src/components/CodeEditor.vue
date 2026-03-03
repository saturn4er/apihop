<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, shallowRef } from "vue";
import { EditorState, type Extension } from "@codemirror/state";
import { EditorView, keymap, placeholder as cmPlaceholder, lineNumbers, Decoration, type DecorationSet, ViewPlugin, type ViewUpdate } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { json } from "@codemirror/lang-json";
import { xml } from "@codemirror/lang-xml";
import { html } from "@codemirror/lang-html";
import { javascript } from "@codemirror/lang-javascript";
import { bracketMatching, foldGutter, syntaxHighlighting, defaultHighlightStyle } from "@codemirror/language";
import { autocompletion, type CompletionContext, type CompletionResult } from "@codemirror/autocomplete";
import { oneDark } from "@codemirror/theme-one-dark";
import { RangeSetBuilder } from "@codemirror/state";

const props = withDefaults(
  defineProps<{
    modelValue?: string;
    language?: "json" | "xml" | "text" | "html" | "javascript";
    readonly?: boolean;
    placeholder?: string;
    variableNames?: string[];
  }>(),
  {
    modelValue: "",
    language: "text",
    readonly: false,
    placeholder: "",
    variableNames: () => [],
  }
);

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const container = ref<HTMLDivElement | null>(null);
const view = shallowRef<EditorView | null>(null);

const appTheme = EditorView.theme({
  "&": {
    fontSize: "13px",
    fontFamily: '"SF Mono", "Fira Code", "Cascadia Code", monospace',
    backgroundColor: "var(--bg-input)",
    color: "var(--text-primary)",
    borderRadius: "var(--radius-md)",
    border: "1px solid var(--border-color)",
  },
  "&.cm-focused": {
    outline: "none",
    borderColor: "var(--accent)",
    boxShadow: "0 0 0 2px var(--accent-muted)",
  },
  ".cm-gutters": {
    backgroundColor: "var(--bg-surface)",
    color: "var(--text-muted)",
    border: "none",
    borderRight: "1px solid var(--border-subtle)",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "var(--bg-hover)",
  },
  ".cm-activeLine": {
    backgroundColor: "rgba(255,255,255,0.03)",
  },
  ".cm-cursor": {
    borderLeftColor: "var(--accent)",
  },
  ".cm-selectionBackground, ::selection": {
    backgroundColor: "rgba(108, 99, 255, 0.2) !important",
  },
  ".cm-content": {
    padding: "8px 0",
    caretColor: "var(--accent)",
  },
  ".cm-scroller": {
    overflow: "auto",
  },
  ".cm-foldGutter .cm-gutterElement": {
    padding: "0 4px",
  },
  ".cm-placeholder": {
    color: "var(--text-muted)",
  },
  ".cm-variable-highlight": {
    backgroundColor: "rgba(108, 99, 255, 0.15)",
    borderRadius: "2px",
    color: "#b4a7ff",
  },
  ".cm-variable-highlight-dynamic": {
    backgroundColor: "rgba(226, 185, 61, 0.15)",
    borderRadius: "2px",
    color: "#e2b93d",
  },
});

const dynamicVariables = [
  { label: "{{$timestamp}}", detail: "Unix timestamp (seconds)" },
  { label: "{{$isoTimestamp}}", detail: "ISO 8601 timestamp" },
  { label: "{{$randomUUID}}", detail: "Random UUID v4" },
  { label: "{{$randomInt}}", detail: "Random integer 0-1000" },
  { label: "{{$randomEmail}}", detail: "Random email address" },
  { label: "{{$randomName}}", detail: "Random full name" },
];

function varCompletion(context: CompletionContext): CompletionResult | null {
  // Use explicit: false to allow activateOnTyping trigger, and check raw text
  const pos = context.pos;
  const line = context.state.doc.lineAt(pos);
  const textBefore = line.text.slice(0, pos - line.from);
  const rawMatch = textBefore.match(/(\{\{\$?\w*)$/);
  if (!rawMatch || rawMatch[1].length < 2) return null;
  const from = pos - rawMatch[1].length;
  const match = rawMatch[1];
  const envVarOptions = props.variableNames.map((name) => ({
    label: `{{${name}}}`,
    detail: "Environment variable",
    type: "variable",
  }));
  const dynamicOptions = dynamicVariables.map((v) => ({
    label: v.label,
    detail: v.detail,
    type: "variable",
  }));
  return {
    from,
    options: [...dynamicOptions, ...envVarOptions],
    filter: true,
  };
}

// Variable highlighting decoration
const varDeco = Decoration.mark({ class: "cm-variable-highlight" });
const dynamicVarDeco = Decoration.mark({ class: "cm-variable-highlight-dynamic" });

function buildDecorations(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const doc = view.state.doc.toString();
  const regex = /\{\{[^}]*\}\}/g;
  let m;
  while ((m = regex.exec(doc)) !== null) {
    const isDynamic = m[0].startsWith("{{$");
    builder.add(m.index, m.index + m[0].length, isDynamic ? dynamicVarDeco : varDeco);
  }
  return builder.finish();
}

const variableHighlightPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.decorations = buildDecorations(view);
    }
    update(update: ViewUpdate) {
      if (update.docChanged) {
        this.decorations = buildDecorations(update.view);
      }
    }
  },
  { decorations: (v) => v.decorations }
);

function getLanguageExtension() {
  switch (props.language) {
    case "json":
      return json();
    case "xml":
      return xml();
    case "html":
      return html();
    case "javascript":
      return javascript();
    default:
      return [];
  }
}

function createState(doc: string) {
  return EditorState.create({
    doc,
    extensions: [
      lineNumbers(),
      history(),
      bracketMatching(),
      foldGutter(),
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      keymap.of([...defaultKeymap, ...historyKeymap]),
      getLanguageExtension(),
      oneDark,
      appTheme,
      EditorState.readOnly.of(props.readonly),
      EditorView.editable.of(!props.readonly),
      ...(!props.readonly ? [autocompletion({ override: [varCompletion], activateOnTyping: true })] : []),
      variableHighlightPlugin,
      ...(props.placeholder ? [cmPlaceholder(props.placeholder)] : []),
      EditorView.updateListener.of((update) => {
        if (update.docChanged && !props.readonly) {
          emit("update:modelValue", update.state.doc.toString());
        }
      }),
    ],
  });
}

onMounted(() => {
  if (!container.value) return;
  view.value = new EditorView({
    state: createState(props.modelValue),
    parent: container.value,
  });
});

onUnmounted(() => {
  view.value?.destroy();
});

// Sync external value changes
let updating = false;
watch(
  () => props.modelValue,
  (newVal) => {
    if (updating) return;
    const v = view.value;
    if (!v) return;
    const current = v.state.doc.toString();
    if (newVal !== current) {
      updating = true;
      v.dispatch({
        changes: { from: 0, to: v.state.doc.length, insert: newVal },
      });
      updating = false;
    }
  }
);

// Recreate state when language or readonly changes
watch(
  () => [props.language, props.readonly],
  () => {
    const v = view.value;
    if (!v) return;
    const doc = v.state.doc.toString();
    v.setState(createState(doc));
  }
);
</script>

<template>
  <div ref="container" class="code-editor"></div>
</template>

<style scoped>
.code-editor {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.code-editor :deep(.cm-editor) {
  height: 100%;
}
</style>
