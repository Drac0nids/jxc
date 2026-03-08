<script setup lang="ts">
import { reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { loginApi, registerApi } from '@/api/auth'
import { ApiClientError } from '@/api/http'
import { useAuthStore } from '@/stores/auth'

type AuthMode = 'login' | 'register'

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()

const mode = ref<AuthMode>('login')
const loading = ref(false)
const errorText = ref('')

const loginForm = reactive({
  username: 'admin',
  password: 'admin123',
})

const registerForm = reactive({
  tenantName: '',
  username: '',
  name: '',
  password: '',
})

function switchMode(nextMode: AuthMode): void {
  if (loading.value || mode.value === nextMode) {
    return
  }

  mode.value = nextMode
  errorText.value = ''
}

function persistSession(payload: {
  access_token: string
  refresh_token: string
  expires_in: number
  tenant_id: string
  user_info: {
    id: string
    name: string
    role: string
  }
}): void {
  authStore.setSession({
    accessToken: payload.access_token,
    refreshToken: payload.refresh_token,
    expiresIn: payload.expires_in,
    tenantId: payload.tenant_id,
    user: {
      id: payload.user_info.id,
      name: payload.user_info.name,
      role: payload.user_info.role,
    },
  })
}

async function submitLogin(): Promise<void> {
  if (!loginForm.username.trim() || !loginForm.password) {
    errorText.value = '请输入用户名和密码'
    return
  }

  const response = await loginApi({
    username: loginForm.username.trim(),
    password: loginForm.password,
  })

  persistSession(response.data)
}

async function submitRegister(): Promise<void> {
  if (!registerForm.username.trim() || !registerForm.name.trim() || !registerForm.password) {
    errorText.value = '请输入用户名、姓名和密码'
    return
  }

  const tenantName = registerForm.tenantName.trim()
  const response = await registerApi({
    tenant_name: tenantName || undefined,
    username: registerForm.username.trim(),
    name: registerForm.name.trim(),
    password: registerForm.password,
  })

  persistSession(response.data)
}

async function submit(): Promise<void> {
  if (loading.value) {
    return
  }

  errorText.value = ''

  loading.value = true
  try {
    if (mode.value === 'login') {
      await submitLogin()
    } else {
      await submitRegister()
    }

    if (errorText.value) {
      return
    }

    const redirect = typeof route.query.redirect === 'string' ? route.query.redirect : '/products'
    await router.replace(redirect)
  } catch (error) {
    if (error instanceof ApiClientError) {
      errorText.value = `${error.message}（code=${error.code}${error.requestId ? `, request_id=${error.requestId}` : ''}）`
    } else {
      errorText.value = error instanceof Error ? error.message : '登录失败'
    }
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="login-page">
    <div class="login-card">
      <h2>{{ mode === 'login' ? '登录系统' : '租户注册' }}</h2>
      <div class="auth-switch">
        <button
          class="btn btn-secondary auth-switch-btn"
          :class="{ 'is-active': mode === 'login' }"
          type="button"
          :disabled="loading"
          @click="switchMode('login')"
        >
          登录
        </button>
        <button
          class="btn btn-secondary auth-switch-btn"
          :class="{ 'is-active': mode === 'register' }"
          type="button"
          :disabled="loading"
          @click="switchMode('register')"
        >
          注册
        </button>
      </div>

      <p v-if="mode === 'login'" class="form-tip">默认演示账号：admin / admin123</p>
      <p v-else class="form-tip">注册成功后将自动登录并创建 OWNER 账号</p>

      <form class="form-grid" @submit.prevent="submit">
        <template v-if="mode === 'login'">
          <label class="form-label">
            <span>用户名</span>
            <input v-model="loginForm.username" type="text" autocomplete="username" />
          </label>

          <label class="form-label">
            <span>密码</span>
            <input v-model="loginForm.password" type="password" autocomplete="current-password" />
          </label>
        </template>

        <template v-else>
          <label class="form-label">
            <span>租户名称（可选）</span>
            <input
              v-model="registerForm.tenantName"
              type="text"
              autocomplete="organization"
              placeholder="例如：华东便利店"
            />
          </label>

          <label class="form-label">
            <span>Owner 登录用户名</span>
            <input v-model="registerForm.username" type="text" autocomplete="username" />
          </label>

          <label class="form-label">
            <span>Owner 姓名</span>
            <input v-model="registerForm.name" type="text" autocomplete="name" />
          </label>

          <label class="form-label">
            <span>密码</span>
            <input v-model="registerForm.password" type="password" autocomplete="new-password" />
          </label>
        </template>

        <button class="btn" :disabled="loading" type="submit">
          {{ loading ? (mode === 'login' ? '登录中...' : '注册中...') : mode === 'login' ? '登录' : '注册并登录' }}
        </button>

        <p v-if="errorText" class="error-text">{{ errorText }}</p>
      </form>
    </div>
  </div>
</template>

<style scoped>
.auth-switch {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin-bottom: 14px;
}

.auth-switch-btn {
  width: 100%;
}

.auth-switch-btn.is-active {
  border-color: #2563eb;
  color: #2563eb;
  background: #eff6ff;
}
</style>
