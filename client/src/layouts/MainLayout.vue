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
  iconClass: string
  roles?: string[]
}

interface NavGroup {
  title: string
  items: NavItem[]
}

const navGroups: NavGroup[] = [
  {
    title: '经营分析',
    items: [
      { to: '/dashboard', label: '经营看板', iconClass: 'dashboard' },
      { to: '/trend', label: '趋势分析', iconClass: 'trend' },
      { to: '/sales-report', label: '销售报表', iconClass: 'report' },
    ]
  },
  {
    title: '资料管理',
    items: [
      { to: '/products', label: '商品列表', iconClass: 'products' },
      { to: '/categories', label: '分类管理', iconClass: 'categories', roles: ['OWNER', 'PURCHASER'] },
      { to: '/suppliers', label: '供应商', iconClass: 'suppliers', roles: ['OWNER', 'PURCHASER'] },
    ]
  },
  {
    title: '库存业务',
    items: [
      { to: '/inbound', label: '采购入库', iconClass: 'inbound', roles: ['OWNER', 'PURCHASER'] },
      { to: '/purchase-orders', label: '采购单库', iconClass: 'purchase', roles: ['OWNER', 'PURCHASER'] },
      { to: '/stock-checks', label: '库存盘点', iconClass: 'checks', roles: ['OWNER', 'PURCHASER'] },
      { to: '/batches', label: '批次跟踪', iconClass: 'batches', roles: ['OWNER', 'PURCHASER'] },
      { to: '/serials', label: '序列号追踪', iconClass: 'serials' },
    ]
  },
  {
    title: '销售业务',
    items: [
      { to: '/outbound', label: '扫码出库', iconClass: 'outbound', roles: ['OWNER', 'SALES'] },
      { to: '/sales-orders', label: '销售单库', iconClass: 'sales', roles: ['OWNER', 'SALES'] },
    ]
  },
  {
    title: '预警与审计',
    items: [
      { to: '/expiring-batches', label: '过期预警', iconClass: 'expiry', roles: ['OWNER', 'PURCHASER'] },
      { to: '/low-stock', label: '低库存预警', iconClass: 'lowstock', roles: ['OWNER', 'PURCHASER'] },
      { to: '/inbound-logs', label: '入库日志', iconClass: 'logs', roles: ['OWNER', 'PURCHASER'] },
      { to: '/stock-check-logs', label: '盘点日志', iconClass: 'checklogs', roles: ['OWNER', 'PURCHASER'] },
      { to: '/stock-logs', label: '库存流水', iconClass: 'logs', roles: ['OWNER', 'PURCHASER'] },
      { to: '/audit-logs', label: '审计日志', iconClass: 'audit', roles: ['OWNER'] },
      { to: '/users', label: '员工管理', iconClass: 'users', roles: ['OWNER'] },
    ]
  }
]

const userLabel = computed(() => authStore.session?.user.name ?? '-')
const roleLabel = computed(() => authStore.session?.user.role ?? '-')
const isLoggingOut = ref(false)

const visibleNavGroups = computed(() => {
  const role = authStore.session?.user.role
  return navGroups.map(group => ({
    ...group,
    items: group.items.filter(item => {
      if (!item.roles || item.roles.length === 0) return true
      if (!role) return false
      return item.roles.includes(role)
    })
  })).filter(group => group.items.length > 0)
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
        <div v-for="group in visibleNavGroups" :key="group.title" class="nav-group">
          <p class="nav-group-title">{{ group.title }}</p>
          <RouterLink
            v-for="item in group.items"
            :key="item.to"
            :to="item.to"
            class="layout-nav-link"
            :class="{ 'is-active': isActive(item.to) }"
          >
            <span class="nav-icon" :class="`nav-icon-${item.iconClass}`" aria-hidden="true" />
            <span class="nav-label">{{ item.label }}</span>
          </RouterLink>
        </div>
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
