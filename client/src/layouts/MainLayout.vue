<script setup lang="ts">
import { computed, ref } from 'vue'
import { RouterLink, RouterView, useRoute, useRouter } from 'vue-router'

import { logoutApi } from '@/api/auth'
import { ApiClientError } from '@/api/http'
import { useAuthStore } from '@/stores/auth'

const authStore = useAuthStore()
const route = useRoute()
const router = useRouter()

interface NavItem {
  to: string
  label: string
  roles?: string[]
}

const navItems: NavItem[] = [
  { to: '/dashboard', label: '经营看板' },
  { to: '/users', label: '员工管理', roles: ['OWNER'] },
  { to: '/products', label: '商品列表' },
  { to: '/inbound', label: '采购入库', roles: ['OWNER', 'PURCHASER'] },
  { to: '/purchase-orders', label: '采购单状态流', roles: ['OWNER', 'PURCHASER'] },
  { to: '/stock-checks', label: '库存盘点状态流', roles: ['OWNER', 'PURCHASER'] },
  { to: '/sales-orders', label: '销售单状态流', roles: ['OWNER', 'SALES'] },
  { to: '/outbound', label: '销售出库', roles: ['OWNER', 'SALES'] },
  { to: '/sales-report', label: '销售报表' },
  { to: '/low-stock', label: '低库存预警', roles: ['OWNER', 'PURCHASER'] },
  { to: '/stock-logs', label: '库存流水', roles: ['OWNER', 'PURCHASER'] },
  { to: '/audit-logs', label: '审计日志', roles: ['OWNER'] },
]

const userLabel = computed(() => authStore.session?.user.name ?? '-')
const roleLabel = computed(() => authStore.session?.user.role ?? '-')
const isLoggingOut = ref(false)

const visibleNavItems = computed(() => {
  const role = authStore.session?.user.role
  return navItems.filter((item) => {
    if (!item.roles || item.roles.length === 0) {
      return true
    }
    if (!role) {
      return false
    }
    return item.roles.includes(role)
  })
})

function isActive(path: string): boolean {
  return route.path.startsWith(path)
}

async function logout(): Promise<void> {
  if (isLoggingOut.value) {
    return
  }

  isLoggingOut.value = true
  try {
    await logoutApi()
  } catch (error) {
    if (error instanceof ApiClientError && error.code !== 4010) {
      // 忽略退出接口失败，继续本地清理会话，避免用户无法退出
      console.warn(
        `logout failed: code=${error.code}${error.requestId ? ` request_id=${error.requestId}` : ''}`,
      )
    }
  } finally {
    authStore.clearSession()
    await router.replace('/login')
    isLoggingOut.value = false
  }
}
</script>

<template>
  <div class="layout-shell">
    <aside class="layout-sidebar">
      <h1 class="layout-title">极速云进销存</h1>
      <p class="layout-subtitle">Windows 客户端 MVP</p>

      <nav class="layout-nav">
        <RouterLink
          v-for="item in visibleNavItems"
          :key="item.to"
          :to="item.to"
          class="layout-nav-link"
          :class="{ 'is-active': isActive(item.to) }"
        >
          {{ item.label }}
        </RouterLink>
      </nav>
    </aside>

    <main class="layout-main">
      <header class="layout-header">
        <div>
          <p class="layout-user">当前用户：{{ userLabel }}（{{ roleLabel }}）</p>
        </div>
        <button class="btn btn-secondary" type="button" :disabled="isLoggingOut" @click="logout">
          {{ isLoggingOut ? '退出中...' : '退出登录' }}
        </button>
      </header>

      <section class="layout-content">
        <RouterView />
      </section>
    </main>
  </div>
</template>
