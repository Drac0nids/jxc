import type { UserSession } from '@/types/app'

const SESSION_STORAGE_KEY = 'jxc.client.session'
const SCAN_PREFERENCES_KEY = 'jxc.client.scan_preferences'

type ScanPreferencePage = 'inbound' | 'outbound'

interface ScopedScanPreferences {
  inbound?: string
  outbound?: string
}

type ScanPreferencesMap = Record<string, ScopedScanPreferences>

export function resolveScanPreferenceScope(session: UserSession | null): string | null {
  if (!session) {
    return null
  }

  const tenantId = session.tenantId?.trim()
  const userId = session.user?.id?.trim()
  if (!tenantId || !userId) {
    return null
  }

  return `${tenantId}:${userId}`
}

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

function readScanPreferencesMap(): ScanPreferencesMap {
  if (typeof window === 'undefined') {
    return {}
  }

  try {
    const raw = window.localStorage.getItem(SCAN_PREFERENCES_KEY)
    if (!raw) {
      return {}
    }

    const parsed = JSON.parse(raw) as unknown
    if (!parsed || typeof parsed !== 'object') {
      return {}
    }

    return parsed as ScanPreferencesMap
  } catch {
    return {}
  }
}

function writeScanPreferencesMap(map: ScanPreferencesMap): void {
  if (typeof window === 'undefined') {
    return
  }

  window.localStorage.setItem(SCAN_PREFERENCES_KEY, JSON.stringify(map))
}

export function readStoredScanMode(scope: string, page: ScanPreferencePage): string | null {
  if (!scope.trim()) {
    return null
  }

  const map = readScanPreferencesMap()
  return map[scope]?.[page] ?? null
}

export function writeStoredScanMode(scope: string, page: ScanPreferencePage, mode: string): void {
  if (!scope.trim() || !mode.trim()) {
    return
  }

  const map = readScanPreferencesMap()
  const scoped = map[scope] ?? {}
  scoped[page] = mode
  map[scope] = scoped
  writeScanPreferencesMap(map)
}

export function clearStoredScanModes(scope?: string): void {
  if (typeof window === 'undefined') {
    return
  }

  if (!scope || !scope.trim()) {
    window.localStorage.removeItem(SCAN_PREFERENCES_KEY)
    return
  }

  const map = readScanPreferencesMap()
  if (!map[scope]) {
    return
  }

  delete map[scope]
  writeScanPreferencesMap(map)
}
