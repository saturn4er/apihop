import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { AuthUser } from '@/api/types';

export const useAuthStore = defineStore('auth', () => {
  const user = ref<AuthUser | null>(null);
  const isAuthenticated = computed(() => !!user.value);

  async function login(email: string, password: string) {
    const { loginUser } = await import('@/api/client');
    const tokens = await loginUser(email, password);
    user.value = tokens.user;
  }

  async function register(email: string, password: string, displayName?: string) {
    const { registerUser } = await import('@/api/client');
    const tokens = await registerUser(email, password, displayName);
    user.value = tokens.user;
  }

  async function logout() {
    const { logoutUser } = await import('@/api/client');
    await logoutUser();
    user.value = null;
  }

  async function loadUser() {
    const token = localStorage.getItem('apihop_access_token');
    if (!token) return;
    try {
      const { getMe } = await import('@/api/client');
      user.value = await getMe();
    } catch {
      // Token invalid/expired
      localStorage.removeItem('apihop_access_token');
      localStorage.removeItem('apihop_refresh_token');
      user.value = null;
    }
  }

  return { user, isAuthenticated, login, register, logout, loadUser };
});
