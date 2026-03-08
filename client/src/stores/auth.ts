import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type { UserSession } from '@/types/app'
import { readStoredSession, writeStoredSession } from '@/utils/sessionStorage'

export const useAuthStore = defineStore('auth', () => {
  const session = ref<UserSession | null>(readStoredSession())

  const isLoggedIn = computed(() => Boolean(session.value?.accessToken))

  const accessToken = computed(() => session.value?.accessToken ?? null)

  const refreshToken = computed(() => session.value?.refreshToken ?? null)

  function setSession(nextSession: UserSession): void {
    session.value = nextSession
    writeStoredSession(nextSession)
  }

  function clearSession(): void {
    session.value = null
    writeStoredSession(null)
  }

  return {
    session,
    isLoggedIn,
    accessToken,
    refreshToken,
    setSession,
    clearSession,
  }
})
