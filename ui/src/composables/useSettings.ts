import { reactive, watch, computed, ref } from "vue";

export interface AppSettings {
  // General
  // NOTE: requestTimeout, followRedirects, maxRedirects are currently frontend-only.
  // They are NOT yet sent to the backend with SendRequestPayload.
  // TODO: add these to SendRequestPayload when the backend supports them.
  requestTimeout: number; // ms, default 30000
  followRedirects: boolean;
  maxRedirects: number;

  // History
  historyRetentionDays: number;
  maxHistoryEntries: number;

  // Appearance
  theme: "dark" | "light" | "system";
  fontSize: number; // px, default 13
}

const STORAGE_KEY = "apihop-settings";

const defaults: AppSettings = {
  requestTimeout: 30000,
  followRedirects: true,
  maxRedirects: 10,
  historyRetentionDays: 30,
  maxHistoryEntries: 1000,
  theme: "dark",
  fontSize: 13,
};

function load(): AppSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      return { ...defaults, ...JSON.parse(raw) };
    }
  } catch {
    // ignore
  }
  return { ...defaults };
}

const settings = reactive<AppSettings>(load());

watch(settings, (val) => {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(val));
}, { deep: true });

// Track system preference reactively
const systemPrefersDark = ref(
  window.matchMedia("(prefers-color-scheme: dark)").matches
);

const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
mediaQuery.addEventListener("change", (e) => {
  systemPrefersDark.value = e.matches;
});

// Resolved theme accounting for "system" preference
const resolvedTheme = computed(() => {
  if (settings.theme !== "system") return settings.theme;
  return systemPrefersDark.value ? "dark" : "light";
});

export function useSettings() {
  return { settings, resolvedTheme, defaults };
}
