import axios, {
  AxiosHeaders,
  type AxiosError,
  type AxiosRequestConfig,
  type AxiosResponse,
  type InternalAxiosRequestConfig,
} from 'axios'
import { invoke } from '@tauri-apps/api/core'

import type { ApiErrorPayload, ApiResponse, RefreshTokenResponseData } from '@/types/api'
import { useAuthStore } from '@/stores/auth'
import { pinia } from '@/stores/pinia'

const DEFAULT_API_BASE_URL = 'http://1.14.45.242:8080/api/v1'

// ── Tauri / local-mode detection ─────────────────────────────────────────────

/** True when running inside a Tauri window (local offline mode) */
function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function looksLikeTauriRuntime(): boolean {
  if (typeof window === 'undefined') {
    return false
  }

  if (isTauri()) {
    return true
  }

  const host = window.location.hostname
  const protocol = window.location.protocol
  return protocol === 'tauri:' || host === 'tauri.localhost' || host.endsWith('.localhost')
}

let _resolvedBaseUrl: string | null = null
let _resolvePromise: Promise<string> | null = null

/**
 * Resolve the API base URL once.
 * - Tauri mode : ask the Rust side for the dynamic port
 * - Web mode   : use VITE_API_BASE_URL env var (falls back to hard-coded remote)
 */
async function resolveApiBaseUrl(): Promise<string> {
  if (_resolvedBaseUrl) return _resolvedBaseUrl
  if (_resolvePromise) return _resolvePromise

  const fallback = (import.meta.env.VITE_API_BASE_URL ?? DEFAULT_API_BASE_URL).replace(/\/+$/, '')
  const preferLocal = looksLikeTauriRuntime()

  _resolvePromise = (async (): Promise<string> => {
    if (preferLocal) {
      try {
        const port = (await invoke('get_server_port')) as number
        const url = `http://127.0.0.1:${port}/api/v1`
        _resolvedBaseUrl = url
        return url
      } catch (e) {
        console.error('[http] failed to get server port from Tauri', e)
        // Keep it retryable in Tauri runtime; avoid permanently pinning to remote.
        _resolvePromise = null
        return fallback
      }
    }

    _resolvedBaseUrl = fallback
    return fallback
  })()

  return _resolvePromise
}

// Bootstrap the URL resolution immediately so it's ready before the first request
resolveApiBaseUrl()

const API_BASE_URL_SYNC = (import.meta.env.VITE_API_BASE_URL ?? DEFAULT_API_BASE_URL).replace(/\/+$/, '')

const X_REQUEST_ID_HEADER = 'x-request-id'
const X_IDEMPOTENCY_KEY_HEADER = 'x-idempotency-key'
const AUTH_REFRESH_RETRY_HEADER = 'x-auth-refresh-retry'

type RequestMethod = 'get' | 'post' | 'put' | 'patch' | 'delete'

interface RetryableRequestConfig extends InternalAxiosRequestConfig {
  _authRefreshed?: boolean
}

let refreshingTokenPromise: Promise<boolean> | null = null

export class ApiClientError extends Error {
  code: number
  requestId?: string
  data?: unknown

  constructor(payload: ApiErrorPayload) {
    super(payload.message)
    this.name = 'ApiClientError'
    this.code = payload.code
    this.requestId = payload.request_id
    this.data = payload.data
  }
}

function generateRequestId(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return `req_${crypto.randomUUID().replace(/-/g, '')}`
  }

  return `req_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`
}

function generateIdempotencyKey(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID()
  }

  return `${Date.now()}-${Math.random().toString(16).slice(2, 10)}`
}

function isWriteMethod(method?: string): boolean {
  if (!method) {
    return false
  }

  const normalized = method.toLowerCase() as RequestMethod
  return normalized === 'post' || normalized === 'put' || normalized === 'patch' || normalized === 'delete'
}

function enrichRequestHeaders(config: InternalAxiosRequestConfig): InternalAxiosRequestConfig {
  const authStore = useAuthStore(pinia)
  const token = authStore.accessToken

  const headers = AxiosHeaders.from(config.headers ?? {})

  if (token) {
    headers.set('Authorization', `Bearer ${token}`)
  }

  if (!headers.has(X_REQUEST_ID_HEADER)) {
    headers.set(X_REQUEST_ID_HEADER, generateRequestId())
  }

  if (isWriteMethod(config.method) && !headers.has(X_IDEMPOTENCY_KEY_HEADER)) {
    headers.set(X_IDEMPOTENCY_KEY_HEADER, generateIdempotencyKey())
  }

  headers.set('X-Client-Type', 'windows')
  config.headers = headers

  return config
}

async function refreshAccessToken(): Promise<boolean> {
  const authStore = useAuthStore(pinia)
  const session = authStore.session
  if (!session?.refreshToken) {
    return false
  }

  try {
    const response = await axios.request<ApiResponse<RefreshTokenResponseData>>({
      method: 'post',
      baseURL: await resolveApiBaseUrl(),
      url: '/auth/refresh',
      data: {
        refresh_token: session.refreshToken,
      },
      headers: {
        [X_REQUEST_ID_HEADER]: generateRequestId(),
        'X-Client-Type': 'windows',
      },
    })

    const payload = response.data.data

    authStore.setSession({
      accessToken: payload.access_token,
      refreshToken: payload.refresh_token ?? session.refreshToken,
      expiresIn: payload.expires_in,
      tenantId: payload.tenant_id,
      user: {
        id: payload.user_info.id,
        name: payload.user_info.name,
        role: payload.user_info.role,
      },
    })
    return true
  } catch {
    return false
  }
}

async function ensureRefreshedToken(): Promise<boolean> {
  if (!refreshingTokenPromise) {
    refreshingTokenPromise = refreshAccessToken().finally(() => {
      refreshingTokenPromise = null
    })
  }

  return refreshingTokenPromise
}

function redirectToLoginPage(): void {
  if (typeof window === 'undefined') {
    return
  }

  if (window.location.hash !== '#/login') {
    window.location.hash = '#/login'
  }
}

async function handleUnauthorizedWithRefresh(
  error: AxiosError<ApiResponse<unknown>>,
): Promise<AxiosResponse<ApiResponse<unknown>> | null> {
  const originalConfig = error.config as RetryableRequestConfig | undefined
  if (!originalConfig) {
    return null
  }

  const isRefreshRequest = originalConfig.url?.includes('/auth/refresh')
  if (isRefreshRequest) {
    return null
  }

  const headers = AxiosHeaders.from(originalConfig.headers ?? {})
  const alreadyRetried =
    originalConfig._authRefreshed === true || headers.has(AUTH_REFRESH_RETRY_HEADER)
  if (alreadyRetried) {
    return null
  }

  const refreshed = await ensureRefreshedToken()
  if (!refreshed) {
    return null
  }

  originalConfig._authRefreshed = true
  headers.set(AUTH_REFRESH_RETRY_HEADER, '1')
  originalConfig.headers = headers

  return http.request<ApiResponse<unknown>>(originalConfig)
}

function normalizeApiError(error: unknown): ApiClientError {
  if (axios.isAxiosError(error)) {
    const axiosError = error as AxiosError<ApiResponse<unknown>>
    const payload = axiosError.response?.data

    if (payload && typeof payload.code === 'number') {
      return new ApiClientError({
        code: payload.code,
        message: payload.message,
        data: payload.data,
        request_id: payload.request_id,
      })
    }

    return new ApiClientError({
      code: 5000,
      message: axiosError.message || '网络请求失败',
      request_id: axiosError.response?.headers?.[X_REQUEST_ID_HEADER] as string | undefined,
    })
  }

  return new ApiClientError({
    code: 5000,
    message: error instanceof Error ? error.message : '未知异常',
  })
}

// Axios instance — starts with a best-effort sync URL; updated dynamically once port resolves
export const http = axios.create({
  baseURL: API_BASE_URL_SYNC,
  timeout: 15_000,
})

// Once the port is known, patch the baseURL so all subsequent requests go to it
resolveApiBaseUrl().then((url) => {
  http.defaults.baseURL = url
})

http.interceptors.request.use(async (config) => {
  config.baseURL = await resolveApiBaseUrl()
  return enrichRequestHeaders(config)
})

http.interceptors.response.use(
  (response) => response,
  async (error: AxiosError<ApiResponse<unknown>>) => {
    if (!axios.isAxiosError(error)) {
      return Promise.reject(error)
    }

    const payload = error.response?.data
    if (payload?.code !== 4010) {
      return Promise.reject(error)
    }

    const replayedResponse = await handleUnauthorizedWithRefresh(error)
    if (replayedResponse) {
      return replayedResponse
    }

    const authStore = useAuthStore(pinia)
    authStore.clearSession()
    redirectToLoginPage()
    return Promise.reject(error)
  },
)

export async function requestApi<T>(config: AxiosRequestConfig): Promise<ApiResponse<T>> {
  try {
    const response = await http.request<ApiResponse<T>>(config)
    return response.data
  } catch (error) {
    throw normalizeApiError(error)
  }
}
