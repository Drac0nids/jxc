import type { UserSession } from '@/types/app'

const SESSION_STORAGE_KEY = 'jxc.client.session'

export function readStoredSession(): UserSession | null {
  if (typeof window === 'undefined') {
    return null
  }

  try {
    const raw = window.localStorage.getItem(SESSION_STORAGE_KEY)
    if (!raw) {
      return null
    }
    return JSON.parse(raw) as UserSession
  } catch {
    return null
  }
}

export function writeStoredSession(session: UserSession | null): void {
  if (typeof window === 'undefined') {
    return
  }

  if (!session) {
    window.localStorage.removeItem(SESSION_STORAGE_KEY)
    return
  }

  window.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(session))
}
