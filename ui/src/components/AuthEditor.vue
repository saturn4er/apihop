<script setup lang="ts">
import { computed } from "vue";
import type { AuthConfig, ApiKeyLocation } from "@/api/client";
import type { FrontendAuthConfig } from "@/api/types";

const props = defineProps<{
  modelValue: AuthConfig;
  inheritedAuth?: AuthConfig;
  showInheritOption?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: AuthConfig];
}>();

const authType = computed(() => props.modelValue.type);

const authTypeLabels: Record<string, string> = {
  none: "None",
  inherit: "Inherit from Collection",
  basic: "Basic Auth",
  bearer: "Bearer Token",
  api_key: "API Key",
  oauth2_client_credentials: "OAuth2 Client Credentials",
};

function setType(type: string) {
  switch (type) {
    case "inherit":
      emit("update:modelValue", { type: "inherit" } as FrontendAuthConfig);
      break;
    case "basic":
      emit("update:modelValue", { type: "basic", username: "", password: "" });
      break;
    case "bearer":
      emit("update:modelValue", { type: "bearer", token: "" });
      break;
    case "api_key":
      emit("update:modelValue", { type: "api_key", key: "", value: "", add_to: "header" });
      break;
    case "oauth2_client_credentials":
      emit("update:modelValue", { type: "oauth2_client_credentials", token_url: "", client_id: "", client_secret: "", scope: "" });
      break;
    default:
      emit("update:modelValue", { type: "none" });
  }
}

function update(field: string, value: string) {
  emit("update:modelValue", { ...props.modelValue, [field]: value } as AuthConfig);
}

function updateAddTo(value: string) {
  if (props.modelValue.type === "api_key") {
    emit("update:modelValue", { ...props.modelValue, add_to: value as ApiKeyLocation });
  }
}

const inheritedLabel = computed(() => {
  if (!props.inheritedAuth || props.inheritedAuth.type === "none") return null;
  return authTypeLabels[props.inheritedAuth.type] || props.inheritedAuth.type;
});
</script>

<template>
  <div class="auth-editor">
    <div class="auth-type-row">
      <label class="auth-label">Type</label>
      <select class="auth-select" :value="authType" @change="setType(($event.target as HTMLSelectElement).value)">
        <option v-if="showInheritOption !== false" value="inherit">Inherit from Collection</option>
        <option value="none">No Auth</option>
        <option value="basic">Basic Auth</option>
        <option value="bearer">Bearer Token</option>
        <option value="api_key">API Key</option>
        <option value="oauth2_client_credentials">OAuth2 Client Credentials</option>
      </select>
    </div>

    <div v-if="authType === 'inherit' && inheritedLabel" class="inherited-indicator">
      Will use collection auth: {{ inheritedLabel }}
    </div>
    <div v-else-if="authType === 'inherit' && !inheritedLabel" class="inherited-indicator inherited-none">
      Collection has no auth configured
    </div>

    <!-- Basic Auth -->
    <template v-if="modelValue.type === 'basic'">
      <div class="auth-field">
        <label class="auth-label">Username</label>
        <input
          type="text"
          class="auth-input"
          :value="modelValue.username"
          placeholder="Username"
          @input="update('username', ($event.target as HTMLInputElement).value)"
        />
      </div>
      <div class="auth-field">
        <label class="auth-label">Password</label>
        <input
          type="password"
          class="auth-input"
          :value="modelValue.password"
          placeholder="Password"
          @input="update('password', ($event.target as HTMLInputElement).value)"
        />
      </div>
    </template>

    <!-- Bearer Token -->
    <template v-if="modelValue.type === 'bearer'">
      <div class="auth-field">
        <label class="auth-label">Token</label>
        <input
          type="password"
          class="auth-input"
          :value="modelValue.token"
          placeholder="Bearer token"
          @input="update('token', ($event.target as HTMLInputElement).value)"
        />
      </div>
    </template>

    <!-- API Key -->
    <template v-if="modelValue.type === 'api_key'">
      <div class="auth-field">
        <label class="auth-label">Key</label>
        <input
          type="text"
          class="auth-input"
          :value="modelValue.key"
          placeholder="Header or param name"
          @input="update('key', ($event.target as HTMLInputElement).value)"
        />
      </div>
      <div class="auth-field">
        <label class="auth-label">Value</label>
        <input
          type="password"
          class="auth-input"
          :value="modelValue.value"
          placeholder="API key value"
          @input="update('value', ($event.target as HTMLInputElement).value)"
        />
      </div>
      <div class="auth-field">
        <label class="auth-label">Add to</label>
        <select class="auth-select" :value="modelValue.add_to" @change="updateAddTo(($event.target as HTMLSelectElement).value)">
          <option value="header">Header</option>
          <option value="query_param">Query Param</option>
        </select>
      </div>
    </template>

    <!-- OAuth2 Client Credentials -->
    <template v-if="modelValue.type === 'oauth2_client_credentials'">
      <div class="auth-field">
        <label class="auth-label">Token URL</label>
        <input
          type="text"
          class="auth-input"
          :value="modelValue.token_url"
          placeholder="https://auth.example.com/oauth/token"
          @input="update('token_url', ($event.target as HTMLInputElement).value)"
        />
      </div>
      <div class="auth-field">
        <label class="auth-label">Client ID</label>
        <input
          type="text"
          class="auth-input"
          :value="modelValue.client_id"
          placeholder="Client ID"
          @input="update('client_id', ($event.target as HTMLInputElement).value)"
        />
      </div>
      <div class="auth-field">
        <label class="auth-label">Client Secret</label>
        <input
          type="password"
          class="auth-input"
          :value="modelValue.client_secret"
          placeholder="Client secret"
          @input="update('client_secret', ($event.target as HTMLInputElement).value)"
        />
      </div>
      <div class="auth-field">
        <label class="auth-label">Scope</label>
        <input
          type="text"
          class="auth-input"
          :value="modelValue.scope"
          placeholder="Optional scope"
          @input="update('scope', ($event.target as HTMLInputElement).value)"
        />
      </div>
    </template>
  </div>
</template>

<style scoped>
.auth-editor {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 8px 0;
}

.auth-type-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.auth-field {
  display: flex;
  align-items: center;
  gap: 12px;
}

.auth-label {
  width: 100px;
  flex-shrink: 0;
  font-size: 12px;
  color: var(--text-secondary);
}

.auth-input,
.auth-select {
  flex: 1;
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  padding: 7px 10px;
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color var(--transition);
}

.auth-input:focus,
.auth-select:focus {
  border-color: var(--accent);
}

.auth-select {
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' fill='%238888A0'%3E%3Cpath d='M2 4l4 4 4-4'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  padding-right: 28px;
}

.auth-select option {
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.inherited-indicator {
  font-size: 12px;
  color: var(--text-secondary);
  background: var(--accent-muted);
  border-radius: var(--radius-sm);
  padding: 6px 10px;
}

.inherited-indicator.inherited-none {
  background: rgba(255, 152, 0, 0.08);
  color: var(--text-muted);
}
</style>
