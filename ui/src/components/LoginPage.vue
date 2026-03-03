<script setup lang="ts">
import { ref, computed } from 'vue';
import { useAuthStore } from '@/stores/authStore';
import { useAppStore } from '@/stores/appStore';

const authStore = useAuthStore();
const appStore = useAppStore();

const isRegister = ref(false);
const email = ref('');
const password = ref('');
const displayName = ref('');
const error = ref('');
const loading = ref(false);

const registrationEnabled = computed(() => {
  if (appStore.mode.kind === 'web-organization') {
    return appStore.mode.serverInfo.registration_enabled;
  }
  return false;
});

const serverName = computed(() => {
  if (appStore.mode.kind === 'web-organization') {
    return appStore.mode.serverInfo.name;
  }
  return 'apihop';
});

async function submit() {
  error.value = '';
  loading.value = true;
  try {
    if (isRegister.value) {
      await authStore.register(email.value, password.value, displayName.value || undefined);
    } else {
      await authStore.login(email.value, password.value);
    }
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : 'An error occurred';
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="login-page">
    <div class="login-card">
      <h1 class="login-title">{{ serverName }}</h1>
      <p class="login-subtitle">{{ isRegister ? 'Create an account' : 'Sign in to continue' }}</p>

      <form @submit.prevent="submit" class="login-form">
        <div v-if="isRegister" class="form-field">
          <label>Display Name</label>
          <input v-model="displayName" type="text" placeholder="Your name (optional)" />
        </div>
        <div class="form-field">
          <label>Email</label>
          <input v-model="email" type="email" placeholder="email@example.com" required />
        </div>
        <div class="form-field">
          <label>Password</label>
          <input v-model="password" type="password" placeholder="Password" required minlength="6" />
        </div>

        <p v-if="error" class="error-message">{{ error }}</p>

        <button type="submit" class="submit-btn" :disabled="loading">
          {{ loading ? 'Please wait...' : (isRegister ? 'Create Account' : 'Sign In') }}
        </button>
      </form>

      <p v-if="registrationEnabled" class="toggle-text">
        <template v-if="isRegister">
          Already have an account? <a @click.prevent="isRegister = false" href="#">Sign in</a>
        </template>
        <template v-else>
          Need an account? <a @click.prevent="isRegister = true" href="#">Register</a>
        </template>
      </p>
    </div>
  </div>
</template>

<style scoped>
.login-page {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100vh;
  background: var(--bg-primary);
}

.login-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-lg, 8px);
  padding: 32px;
  width: 360px;
  max-width: 90vw;
}

.login-title {
  font-size: 20px;
  font-weight: 700;
  margin: 0 0 4px 0;
  text-align: center;
}

.login-subtitle {
  color: var(--text-secondary);
  font-size: 13px;
  margin: 0 0 24px 0;
  text-align: center;
}

.login-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.form-field label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
}

.form-field input {
  background: var(--bg-primary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm, 4px);
  padding: 8px 12px;
  font-size: 13px;
  color: var(--text-primary);
  outline: none;
}

.form-field input:focus {
  border-color: var(--accent);
}

.error-message {
  color: var(--color-error, #e74c3c);
  font-size: 12px;
  margin: 0;
}

.submit-btn {
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: var(--radius-sm, 4px);
  padding: 10px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}

.submit-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.toggle-text {
  text-align: center;
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 16px;
}

.toggle-text a {
  color: var(--accent);
  cursor: pointer;
  text-decoration: none;
}
</style>
