import type {
  LoginRequest,
  LoginResponseData,
  LogoutResponseData,
  RegisterRequest,
  RegisterResponseData,
  RefreshTokenRequest,
  RefreshTokenResponseData,
} from '@/types/api'
import { requestApi } from '@/api/http'

export async function registerApi(payload: RegisterRequest) {
  return requestApi<RegisterResponseData>({
    method: 'post',
    url: '/auth/register',
    data: payload,
  })
}

export async function loginApi(payload: LoginRequest) {
  return requestApi<LoginResponseData>({
    method: 'post',
    url: '/auth/login',
    data: payload,
  })
}

export async function refreshTokenApi(payload: RefreshTokenRequest) {
  return requestApi<RefreshTokenResponseData>({
    method: 'post',
    url: '/auth/refresh',
    data: payload,
  })
}

export async function logoutApi() {
  return requestApi<LogoutResponseData>({
    method: 'post',
    url: '/auth/logout',
  })
}
