<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'

import {
  createUserApi,
  listUsersApi,
  resetUserPasswordApi,
  updateUserRoleApi,
} from '@/api/users'
import { ApiClientError } from '@/api/http'
import { useAuthStore } from '@/stores/auth'
import type { UserData } from '@/types/api'

const authStore = useAuthStore()

const roles = ['OWNER', 'PURCHASER', 'SALES'] as const

const loading = ref(false)
const createLoading = ref(false)
const updateRoleLoadingId = ref<string | null>(null)
const resetPwdLoadingId = ref<string | null>(null)

const errorText = ref('')
const successText = ref('')

const users = ref<UserData[]>([])
const total = ref(0)

const form = reactive({
  username: '',
  name: '',
  role: 'SALES',
  password: '',
})

const resetPasswordDraft = reactive<Record<string, string>>({})

const isOwner = computed(() => authStore.session?.user.role === 'OWNER')

function formatApiError(error: unknown, fallback: string): string {
  if (error instanceof ApiClientError) {
    return `${error.message}（code=${error.code}${error.requestId ? `, request_id=${error.requestId}` : ''}）`
  }

  return error instanceof Error ? error.message : fallback
}

async function fetchUsers(): Promise<void> {
  if (!isOwner.value) {
    errorText.value = '当前角色无员工管理权限，仅 OWNER 可访问。'
    return
  }

  loading.value = true
  errorText.value = ''

  try {
    const response = await listUsersApi()
    users.value = response.data.list
    total.value = response.data.total

    users.value.forEach((user) => {
      if (!Object.prototype.hasOwnProperty.call(resetPasswordDraft, user.id)) {
        resetPasswordDraft[user.id] = ''
      }
    })
  } catch (error) {
    errorText.value = formatApiError(error, '员工列表查询失败')
  } finally {
    loading.value = false
  }
}

function resetCreateForm(): void {
  form.username = ''
  form.name = ''
  form.role = 'SALES'
  form.password = ''
}

function validateCreateForm(): string | null {
  if (!form.username.trim()) {
    return '用户名不能为空'
  }

  if (!form.name.trim()) {
    return '员工姓名不能为空'
  }

  if (!roles.includes(form.role as (typeof roles)[number])) {
    return '角色不合法'
  }

  if (form.password.trim().length < 6) {
    return '初始密码至少 6 位'
  }

  return null
}

async function submitCreateUser(): Promise<void> {
  if (!isOwner.value || createLoading.value) {
    return
  }

  successText.value = ''
  errorText.value = ''

  const validationError = validateCreateForm()
  if (validationError) {
    errorText.value = validationError
    return
  }

  createLoading.value = true
  try {
    const response = await createUserApi({
      username: form.username.trim(),
      name: form.name.trim(),
      role: form.role,
      password: form.password.trim(),
    })

    successText.value = `新增员工成功：${response.data.name}（${response.data.username}）`
    resetCreateForm()
    await fetchUsers()
  } catch (error) {
    errorText.value = formatApiError(error, '新增员工失败')
  } finally {
    createLoading.value = false
  }
}

async function updateRole(user: UserData, nextRole: string): Promise<void> {
  if (!isOwner.value || updateRoleLoadingId.value) {
    return
  }

  if (user.role === nextRole) {
    return
  }

  successText.value = ''
  errorText.value = ''
  updateRoleLoadingId.value = user.id

  try {
    const response = await updateUserRoleApi(user.id, {
      role: nextRole,
    })
    const target = users.value.find((item) => item.id === user.id)
    if (target) {
      target.role = response.data.role
    }
    successText.value = `角色更新成功：${response.data.name} -> ${response.data.role}`
  } catch (error) {
    errorText.value = formatApiError(error, '更新角色失败')
  } finally {
    updateRoleLoadingId.value = null
  }
}

async function resetPassword(user: UserData): Promise<void> {
  if (!isOwner.value || resetPwdLoadingId.value) {
    return
  }

  const nextPassword = (resetPasswordDraft[user.id] ?? '').trim()
  if (nextPassword.length < 6) {
    errorText.value = `员工 ${user.username} 的新密码至少 6 位`
    return
  }

  successText.value = ''
  errorText.value = ''
  resetPwdLoadingId.value = user.id
  try {
    await resetUserPasswordApi(user.id, {
      new_password: nextPassword,
    })
    resetPasswordDraft[user.id] = ''
    successText.value = `重置密码成功：${user.name}（${user.username}）`
  } catch (error) {
    errorText.value = formatApiError(error, '重置密码失败')
  } finally {
    resetPwdLoadingId.value = null
  }
}

onMounted(() => {
  if (isOwner.value) {
    void fetchUsers()
  }
})
</script>

<template>
  <section>
    <h2>员工管理</h2>

    <p v-if="!isOwner" class="warn-text">当前角色无权限访问员工管理，仅 OWNER 可访问。</p>

    <div v-else>
      <div class="card-panel form-grid">
        <h3>新增员工</h3>

        <div class="form-inline form-inline-compact">
          <label class="form-label inline">
            <span>用户名 *</span>
            <input v-model="form.username" :disabled="createLoading" placeholder="例如：sales01" />
          </label>

          <label class="form-label inline">
            <span>姓名 *</span>
            <input v-model="form.name" :disabled="createLoading" placeholder="例如：王五" />
          </label>

          <label class="form-label inline">
            <span>角色 *</span>
            <select v-model="form.role" :disabled="createLoading">
              <option v-for="role in roles" :key="role" :value="role">{{ role }}</option>
            </select>
          </label>

          <label class="form-label inline">
            <span>初始密码 *</span>
            <input
              v-model="form.password"
              :disabled="createLoading"
              type="password"
              placeholder="至少 6 位"
            />
          </label>
        </div>

        <div class="form-actions">
          <button class="btn" type="button" :disabled="createLoading" @click="submitCreateUser">
            {{ createLoading ? '创建中...' : '创建员工' }}
          </button>
          <button class="btn btn-secondary" type="button" :disabled="createLoading" @click="resetCreateForm">
            重置
          </button>
          <button class="btn btn-secondary" type="button" :disabled="loading" @click="fetchUsers">
            刷新列表
          </button>
        </div>
      </div>

      <p v-if="errorText" class="error-text">{{ errorText }}</p>
      <p v-if="successText" class="success-text">{{ successText }}</p>

      <p class="table-summary">共 {{ total }} 名员工</p>

      <div class="table-wrapper">
        <table class="data-table">
          <thead>
            <tr>
              <th>ID</th>
              <th>用户名</th>
              <th>姓名</th>
              <th>角色</th>
              <th>修改角色</th>
              <th>重置密码</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="user in users" :key="user.id">
              <td>
                <small>{{ user.id }}</small>
              </td>
              <td>{{ user.username }}</td>
              <td>{{ user.name }}</td>
              <td>{{ user.role }}</td>
              <td>
                <div class="form-inline form-inline-compact">
                  <select
                    :value="user.role"
                    :disabled="updateRoleLoadingId !== null"
                    @change="updateRole(user, ($event.target as HTMLSelectElement).value)"
                  >
                    <option v-for="role in roles" :key="role" :value="role">{{ role }}</option>
                  </select>
                </div>
              </td>
              <td>
                <div class="form-inline form-inline-compact">
                  <input
                    v-model="resetPasswordDraft[user.id]"
                    type="password"
                    placeholder="新密码（>=6位）"
                    :disabled="resetPwdLoadingId !== null"
                  />
                  <button
                    class="btn btn-secondary"
                    type="button"
                    :disabled="resetPwdLoadingId !== null"
                    @click="resetPassword(user)"
                  >
                    {{ resetPwdLoadingId === user.id ? '重置中...' : '重置' }}
                  </button>
                </div>
              </td>
            </tr>
            <tr v-if="!loading && users.length === 0">
              <td colspan="6" class="empty-cell">暂无员工数据</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </section>
</template>
