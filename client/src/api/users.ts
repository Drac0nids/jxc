import { requestApi } from '@/api/http'
import type {
  CreateUserRequest,
  ListUsersResponseData,
  ResetUserPasswordRequest,
  ResetUserPasswordResponseData,
  UpdateUserRoleRequest,
  UserData,
} from '@/types/api'

export async function listUsersApi() {
  return requestApi<ListUsersResponseData>({
    method: 'get',
    url: '/users',
  })
}

export async function createUserApi(payload: CreateUserRequest) {
  return requestApi<UserData>({
    method: 'post',
    url: '/users',
    data: payload,
  })
}

export async function updateUserRoleApi(id: string, payload: UpdateUserRoleRequest) {
  return requestApi<UserData>({
    method: 'patch',
    url: `/users/${id}/role`,
    data: payload,
  })
}

export async function resetUserPasswordApi(id: string, payload: ResetUserPasswordRequest) {
  return requestApi<ResetUserPasswordResponseData>({
    method: 'post',
    url: `/users/${id}/reset-password`,
    data: payload,
  })
}
