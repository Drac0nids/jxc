import { createRouter, createWebHashHistory } from 'vue-router'

import { useAuthStore } from '@/stores/auth'

const routes = [
  {
    path: '/login',
    name: 'login',
    component: () => import('@/pages/LoginPage.vue'),
  },
  {
    path: '/',
    component: () => import('@/layouts/MainLayout.vue'),
    meta: { requiresAuth: true },
    children: [
      {
        path: '',
        redirect: '/dashboard',
      },
      {
        path: 'dashboard',
        name: 'dashboard',
        meta: { roles: ['OWNER', 'PURCHASER', 'SALES'] },
        component: () => import('@/pages/DashboardPage.vue'),
      },
      {
        path: 'users',
        name: 'users',
        meta: { roles: ['OWNER'] },
        component: () => import('@/pages/UsersPage.vue'),
      },
      {
        path: 'products',
        name: 'products',
        meta: { roles: ['OWNER', 'PURCHASER', 'SALES'] },
        component: () => import('@/pages/ProductsPage.vue'),
      },
      {
        path: 'inbound',
        name: 'inbound',
        meta: { roles: ['OWNER', 'PURCHASER'] },
        component: () => import('@/pages/InboundPage.vue'),
      },
      {
        path: 'purchase-orders',
        name: 'purchase-orders',
        meta: { roles: ['OWNER', 'PURCHASER'] },
        component: () => import('@/pages/PurchaseOrdersPage.vue'),
      },
      {
        path: 'stock-checks',
        name: 'stock-checks',
        meta: { roles: ['OWNER', 'PURCHASER'] },
        component: () => import('@/pages/StockChecksPage.vue'),
      },
      {
        path: 'sales-orders',
        name: 'sales-orders',
        meta: { roles: ['OWNER', 'SALES'] },
        component: () => import('@/pages/SalesOrdersPage.vue'),
      },
      {
        path: 'outbound',
        name: 'outbound',
        meta: { roles: ['OWNER', 'SALES'] },
        component: () => import('@/pages/OutboundPage.vue'),
      },
      {
        path: 'low-stock',
        name: 'low-stock',
        meta: { roles: ['OWNER', 'PURCHASER'] },
        component: () => import('@/pages/LowStockPage.vue'),
      },
      {
        path: 'sales-report',
        name: 'sales-report',
        meta: { roles: ['OWNER', 'PURCHASER', 'SALES'] },
        component: () => import('@/pages/SalesReportPage.vue'),
      },
      {
        path: 'audit-logs',
        name: 'audit-logs',
        meta: { roles: ['OWNER'] },
        component: () => import('@/pages/AuditLogsPage.vue'),
      },
      {
        path: 'stock-logs',
        name: 'stock-logs',
        meta: { roles: ['OWNER', 'PURCHASER'] },
        component: () => import('@/pages/StockLogsPage.vue'),
      },
    ],
  },
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

router.beforeEach((to) => {
  const authStore = useAuthStore()

  if (to.meta.requiresAuth && !authStore.isLoggedIn) {
    return {
      path: '/login',
      query: {
        redirect: to.fullPath,
      },
    }
  }

  if (to.path === '/login' && authStore.isLoggedIn) {
    return '/dashboard'
  }

  const requiredRoles = to.meta.roles as string[] | undefined
  if (requiredRoles?.length) {
    const role = authStore.session?.user.role
    if (!role || !requiredRoles.includes(role)) {
      return '/dashboard'
    }
  }

  return true
})
