<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, reactive, ref, watch } from 'vue'
import { useRouter } from 'vue-router'

import { getDashboardApi } from '@/api/inventory'
import { ApiClientError } from '@/api/http'
import { useAuthStore } from '@/stores/auth'
import type { DashboardData } from '@/types/api'

function formatToday(): string {
  const now = new Date()
  const pad = (value: number): string => value.toString().padStart(2, '0')
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
}

function parseDateValue(raw: string): { normalized: string } | { error: string } {
  const value = raw.trim()
  if (!value) {
    return { normalized: '' }
  }

  const match = value.match(/^(\d{4})-(\d{2})-(\d{2})$/)
  if (!match) {
    return { error: '日期格式必须为 YYYY-MM-DD' }
  }

  const year = Number(match[1])
  const month = Number(match[2])
  const day = Number(match[3])
  const date = new Date(year, month - 1, day)

  if (
    Number.isNaN(date.getTime()) ||
    date.getFullYear() !== year ||
    date.getMonth() !== month - 1 ||
    date.getDate() !== day
  ) {
    return { error: '日期不合法，请重新选择' }
  }

  return { normalized: value }
}

const loading = ref(false)
const errorText = ref('')
const lastUpdatedText = ref('')
const dashboard = ref<DashboardData | null>(null)
const animated = reactive({
  sales: 0,
  grossProfit: 0,
  orders: 0,
  lowStock: 0,
})
const authStore = useAuthStore()
const router = useRouter()
const runningFrames = new Set<number>()

const canViewGrossProfit = computed(() => authStore.session?.user.role !== 'SALES')
const canDrilldownOrders = computed(() => {
  const role = authStore.session?.user.role
  return role === 'OWNER' || role === 'SALES'
})

const query = reactive({
  date: formatToday(),
})

function toNumber(value: string | number): number {
  const parsed = typeof value === 'number' ? value : Number(value)
  return Number.isFinite(parsed) ? parsed : 0
}

function formatMoney(value: number): string {
  return value.toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
}

function animateTo(key: keyof typeof animated, target: number, duration = 700): void {
  const start = animated[key]
  const delta = target - start
  const startTime = performance.now()

  const step = (now: number): void => {
    const progress = Math.min((now - startTime) / duration, 1)
    const eased = 1 - Math.pow(1 - progress, 3)
    animated[key] = start + delta * eased

    if (progress < 1) {
      const frame = requestAnimationFrame(step)
      runningFrames.add(frame)
      return
    }

    animated[key] = target
  }

  const frame = requestAnimationFrame(step)
  runningFrames.add(frame)
}

const sparklineValues = computed(() => {
  const base = toNumber(dashboard.value?.total_sales ?? 0)
  if (base <= 0) {
    return [8, 10, 12, 9, 11, 13, 15]
  }

  return Array.from({ length: 7 }, (_, index) => {
    const wave = Math.sin(index * 1.2) * 0.08
    const trend = 0.72 + index * 0.06
    return base * (trend + wave)
  })
})

const sparklinePath = computed(() => {
  const values = sparklineValues.value
  const max = Math.max(...values)
  const min = Math.min(...values)
  const range = Math.max(max - min, 1)
  const width = 220
  const height = 34

  return values
    .map((value, index) => {
      const x = (index / (values.length - 1)) * width
      const y = height - ((value - min) / range) * (height - 4) - 2
      return `${index === 0 ? 'M' : 'L'}${x.toFixed(2)},${y.toFixed(2)}`
    })
    .join(' ')
})

const sparklineAreaPath = computed(() => `${sparklinePath.value} L220,34 L0,34 Z`)

function validateDate(raw: string): string | null {
  const result = parseDateValue(raw)
  return 'error' in result ? result.error : null
}

async function fetchDashboard(): Promise<void> {
  errorText.value = ''

  const dateError = validateDate(query.date)
  if (dateError) {
    errorText.value = dateError
    return
  }

  loading.value = true
  try {
    const parsedDate = parseDateValue(query.date)
    const date = 'error' in parsedDate ? undefined : parsedDate.normalized || undefined

    const response = await getDashboardApi({
      date,
    })
    dashboard.value = response.data
    lastUpdatedText.value = new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
  } catch (error) {
    if (error instanceof ApiClientError) {
      errorText.value = `${error.message}（code=${error.code}${error.requestId ? `, request_id=${error.requestId}` : ''}）`
    } else {
      errorText.value = error instanceof Error ? error.message : '经营看板查询失败'
    }
  } finally {
    loading.value = false
  }
}

function resetToToday(): void {
  query.date = formatToday()
  void fetchDashboard()
}

async function goToOrdersDrilldown(): Promise<void> {
  if (!dashboard.value) {
    return
  }

  if (!canDrilldownOrders.value) {
    errorText.value = '当前角色无销售单下钻权限，仅 OWNER / SALES 可查看'
    return
  }

  const drilldownPageSize = Math.min(Math.max(dashboard.value.total_orders, 1), 100)

  try {
    await router.push({
      name: 'sales-orders',
      query: {
        start_date: dashboard.value.date,
        end_date: dashboard.value.date,
        page: '1',
        page_size: String(drilldownPageSize),
      },
    })
  } catch (error) {
    errorText.value = error instanceof Error ? `订单下钻跳转失败：${error.message}` : '订单下钻跳转失败'
  }
}

onMounted(() => {
  void fetchDashboard()
})

watch(dashboard, (next) => {
  if (!next) {
    return
  }

  animateTo('sales', toNumber(next.total_sales))
  animateTo('grossProfit', toNumber(next.total_gross_profit))
  animateTo('orders', next.total_orders)
  animateTo('lowStock', next.low_stock_count)
})

onBeforeUnmount(() => {
  runningFrames.forEach((frame) => cancelAnimationFrame(frame))
  runningFrames.clear()
})
</script>

<template>
  <section>
    <div class="card-panel-header" style="margin-bottom: 20px">
      <h2>经营看板</h2>
      <div v-if="lastUpdatedText" class="live-indicator">
        <span class="live-dot" />
        <span>数据已同步于 {{ lastUpdatedText }}</span>
      </div>
    </div>
    <p v-if="!canViewGrossProfit" class="warn-text">当前角色为 SALES，已隐藏毛利指标。</p>

    <form class="form-inline" @submit.prevent="fetchDashboard">
      <label class="form-label inline">
        <span>统计日期</span>
        <input v-model="query.date" type="date" />
      </label>

      <button class="btn" type="submit" :disabled="loading">{{ loading ? '查询中...' : '查询' }}</button>
      <button class="btn btn-secondary" type="button" :disabled="loading" @click="resetToToday">
        重置为今天
      </button>
    </form>

    <p v-if="errorText" class="error-text">{{ errorText }}</p>

    <div v-if="loading && !dashboard" class="page-skeleton">
      <div class="skeleton-item h-24" />
      <div class="skeleton-item h-44" />
      <div class="skeleton-item h-44" />
    </div>

    <div v-if="dashboard" class="dashboard-grid">
      <article class="metric-card">
        <p class="metric-label">销售额</p>
        <p class="metric-value metric-value-primary mono-number">¥ {{ formatMoney(animated.sales) }}</p>
        <div class="metric-trend" aria-hidden="true">
          <svg viewBox="0 0 220 34" preserveAspectRatio="none">
            <path class="sparkline-area" :d="sparklineAreaPath" />
            <path class="sparkline-line" :d="sparklinePath" />
          </svg>
        </div>
      </article>

      <article v-if="canViewGrossProfit" class="metric-card">
        <p class="metric-label">毛利润</p>
        <p class="metric-value mono-number">¥ {{ formatMoney(animated.grossProfit) }}</p>
      </article>

      <article
        class="metric-card"
        :class="{ 'metric-card-clickable': canDrilldownOrders }"
        @click="goToOrdersDrilldown"
      >
        <p class="metric-label">订单数</p>
        <button class="metric-value metric-value-button" type="button" :disabled="loading" @click.stop="goToOrdersDrilldown">
          {{ Math.round(animated.orders) }}
        </button>
        <p class="metric-hint">
          {{ canDrilldownOrders ? '点击查看订单下钻' : '仅 OWNER / SALES 可查看订单下钻' }}
        </p>
      </article>

      <article class="metric-card">
        <p class="metric-label">低库存预警数</p>
        <p class="metric-value mono-number">{{ Math.round(animated.lowStock) }}</p>
      </article>

      <article class="metric-card metric-card-wide">
        <p class="metric-label">热销商品</p>
        <p class="metric-value">{{ dashboard.top_selling_item || '暂无' }}</p>
      </article>
    </div>
  </section>
</template>

<style scoped>
.metric-value-button {
  border: 0;
  background: transparent;
  color: inherit;
  padding: 0;
  text-align: left;
}

.metric-card-clickable {
  cursor: pointer;
}

.metric-card-clickable .metric-value-button {
  color: var(--brand-700);
  cursor: pointer;
}

.metric-card-clickable .metric-value-button:disabled {
  color: inherit;
  text-decoration: none;
  cursor: not-allowed;
}

.metric-hint {
  margin-top: 6px;
  color: var(--text-muted);
  font-size: 12px;
}
</style>
