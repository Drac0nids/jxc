<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'

import { getDashboardApi } from '@/api/inventory'
import { ApiClientError } from '@/api/http'
import { useAuthStore } from '@/stores/auth'
import type { DashboardData } from '@/types/api'

function formatToday(): string {
  const now = new Date()
  const pad = (value: number): string => value.toString().padStart(2, '0')
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
}

const loading = ref(false)
const errorText = ref('')
const dashboard = ref<DashboardData | null>(null)
const authStore = useAuthStore()

const canViewGrossProfit = computed(() => authStore.session?.user.role !== 'SALES')

const query = reactive({
  date: formatToday(),
})

function validateDate(raw: string): string | null {
  const value = raw.trim()
  if (!value) {
    return null
  }

  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) {
    return '日期格式必须为 YYYY-MM-DD'
  }

  const date = new Date(`${value}T00:00:00`)
  if (Number.isNaN(date.getTime())) {
    return '日期不合法，请重新输入'
  }

  return null
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
    const response = await getDashboardApi({
      date: query.date.trim() || undefined,
    })
    dashboard.value = response.data
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

onMounted(() => {
  void fetchDashboard()
})
</script>

<template>
  <section>
    <h2>经营看板</h2>
    <p v-if="!canViewGrossProfit" class="warn-text">当前角色为 SALES，已隐藏毛利指标。</p>

    <form class="form-inline" @submit.prevent="fetchDashboard">
      <label class="form-label inline">
        <span>统计日期</span>
        <input v-model="query.date" placeholder="YYYY-MM-DD（留空默认当天）" />
      </label>

      <button class="btn" type="submit" :disabled="loading">{{ loading ? '查询中...' : '查询' }}</button>
      <button class="btn btn-secondary" type="button" :disabled="loading" @click="resetToToday">
        重置为今天
      </button>
    </form>

    <p v-if="errorText" class="error-text">{{ errorText }}</p>

    <div v-if="dashboard" class="dashboard-grid">
      <article class="metric-card">
        <p class="metric-label">销售额</p>
        <p class="metric-value">¥ {{ dashboard.total_sales }}</p>
      </article>

      <article v-if="canViewGrossProfit" class="metric-card">
        <p class="metric-label">毛利润</p>
        <p class="metric-value">¥ {{ dashboard.total_gross_profit }}</p>
      </article>

      <article class="metric-card">
        <p class="metric-label">订单数</p>
        <p class="metric-value">{{ dashboard.total_orders }}</p>
      </article>

      <article class="metric-card">
        <p class="metric-label">低库存预警数</p>
        <p class="metric-value">{{ dashboard.low_stock_count }}</p>
      </article>

      <article class="metric-card metric-card-wide">
        <p class="metric-label">热销商品</p>
        <p class="metric-value">{{ dashboard.top_selling_item || '暂无' }}</p>
      </article>
    </div>
  </section>
</template>
