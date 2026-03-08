<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'

import { exportSalesReportApi, getSalesReportApi } from '@/api/inventory'
import { ApiClientError } from '@/api/http'
import { useAuthStore } from '@/stores/auth'
import type {
  SalesReportData,
  SalesReportExportFormat,
  SalesReportItemData,
  SalesReportSummaryData,
} from '@/types/api'

const authStore = useAuthStore()

const loading = ref(false)
const exportingCsv = ref(false)
const exportingXlsx = ref(false)
const errorText = ref('')
const successText = ref('')

const query = reactive({
  start_date: '',
  end_date: '',
  page: 1,
  page_size: 20,
})

const reportData = ref<SalesReportData | null>(null)

const reportList = computed<SalesReportItemData[]>(() => reportData.value?.list ?? [])
const reportSummary = computed<SalesReportSummaryData | null>(() => reportData.value?.summary ?? null)
const total = computed(() => reportData.value?.total ?? 0)
const currentPage = computed(() => reportData.value?.page ?? query.page)
const pageSize = computed(() => reportData.value?.page_size ?? query.page_size)

const canExport = computed(() => {
  const role = authStore.session?.user.role
  return role === 'OWNER' || role === 'PURCHASER'
})

const canViewCostMetrics = computed(() => authStore.session?.user.role !== 'SALES')

const busy = computed(
  () => loading.value || exportingCsv.value || exportingXlsx.value,
)

function formatToday(): string {
  const now = new Date()
  const pad = (value: number): string => value.toString().padStart(2, '0')
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
}

function normalizeDateInput(value: string): string {
  return value.trim()
}

function validateDate(value: string, label: string): string | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) {
    return `${label}格式必须为 YYYY-MM-DD`
  }

  const date = new Date(`${value}T00:00:00`)
  if (Number.isNaN(date.getTime())) {
    return `${label}不是合法日期`
  }

  return null
}

function buildDateRangeValidationError(startDate: string, endDate: string): string | null {
  const hasStart = Boolean(startDate)
  const hasEnd = Boolean(endDate)

  if (hasStart !== hasEnd) {
    return 'start_date 与 end_date 需同时提供，或同时留空'
  }

  if (!hasStart && !hasEnd) {
    return null
  }

  const startError = validateDate(startDate, '开始日期')
  if (startError) {
    return startError
  }

  const endError = validateDate(endDate, '结束日期')
  if (endError) {
    return endError
  }

  if (startDate > endDate) {
    return 'start_date 不能晚于 end_date'
  }

  return null
}

function formatApiError(error: unknown, fallback: string): string {
  if (error instanceof ApiClientError) {
    return `${error.message}（code=${error.code}${error.requestId ? `, request_id=${error.requestId}` : ''}）`
  }
  return error instanceof Error ? error.message : fallback
}

async function fetchReport(options?: { keepPage?: boolean }): Promise<void> {
  if (busy.value) {
    return
  }

  errorText.value = ''
  successText.value = ''

  const startDate = normalizeDateInput(query.start_date)
  const endDate = normalizeDateInput(query.end_date)
  const dateRangeError = buildDateRangeValidationError(startDate, endDate)
  if (dateRangeError) {
    errorText.value = dateRangeError
    return
  }

  if (!options?.keepPage) {
    query.page = 1
  }

  loading.value = true
  try {
    const response = await getSalesReportApi({
      start_date: startDate || undefined,
      end_date: endDate || undefined,
      group_by: 'product',
      page: query.page,
      page_size: query.page_size,
    })

    reportData.value = response.data
    successText.value = `销售报表查询成功，共 ${response.data.total} 条`
  } catch (error) {
    errorText.value = formatApiError(error, '销售报表查询失败')
  } finally {
    loading.value = false
  }
}

function resetToToday(): void {
  const today = formatToday()
  query.start_date = today
  query.end_date = today
  query.page = 1
  reportData.value = null
  errorText.value = ''
  successText.value = ''
  void fetchReport()
}

function clearDateRange(): void {
  query.start_date = ''
  query.end_date = ''
  query.page = 1
  reportData.value = null
  errorText.value = ''
  successText.value = ''
  void fetchReport()
}

function prevPage(): void {
  if (busy.value || currentPage.value <= 1) {
    return
  }

  query.page = currentPage.value - 1
  void fetchReport({ keepPage: true })
}

function nextPage(): void {
  if (busy.value || currentPage.value * pageSize.value >= total.value) {
    return
  }

  query.page = currentPage.value + 1
  void fetchReport({ keepPage: true })
}

function triggerBrowserDownload(blob: Blob, filename: string): void {
  const objectUrl = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = objectUrl
  anchor.download = filename
  document.body.appendChild(anchor)
  anchor.click()
  anchor.remove()
  URL.revokeObjectURL(objectUrl)
}

async function exportReport(format: SalesReportExportFormat): Promise<void> {
  if (busy.value) {
    return
  }

  errorText.value = ''
  successText.value = ''

  if (!canExport.value) {
    errorText.value = '当前角色无导出权限，仅 OWNER/PURCHASER 可导出'
    return
  }

  const startDate = normalizeDateInput(query.start_date)
  const endDate = normalizeDateInput(query.end_date)
  const dateRangeError = buildDateRangeValidationError(startDate, endDate)
  if (dateRangeError) {
    errorText.value = dateRangeError
    return
  }

  if (format === 'csv') {
    exportingCsv.value = true
  } else {
    exportingXlsx.value = true
  }

  try {
    const result = await exportSalesReportApi({
      start_date: startDate || undefined,
      end_date: endDate || undefined,
      group_by: 'product',
      format,
    })
    triggerBrowserDownload(result.blob, result.filename)
    successText.value = `销售报表导出成功：${result.filename}`
  } catch (error) {
    errorText.value = formatApiError(error, '销售报表导出失败')
  } finally {
    exportingCsv.value = false
    exportingXlsx.value = false
  }
}

onMounted(() => {
  const today = formatToday()
  query.start_date = today
  query.end_date = today
  void fetchReport()
})
</script>

<template>
  <section>
    <h2>销售报表</h2>

    <p v-if="!canExport" class="warn-text">当前角色仅支持查询，导出权限限 OWNER/PURCHASER。</p>
    <p v-if="!canViewCostMetrics" class="warn-text">当前角色为 SALES，已隐藏成本与毛利字段。</p>

    <form class="form-inline" @submit.prevent="fetchReport()">
      <label class="form-label inline">
        <span>开始日期</span>
        <input v-model="query.start_date" :disabled="busy" placeholder="YYYY-MM-DD（留空默认当天）" />
      </label>

      <label class="form-label inline">
        <span>结束日期</span>
        <input v-model="query.end_date" :disabled="busy" placeholder="YYYY-MM-DD（留空默认当天）" />
      </label>

      <button class="btn" type="submit" :disabled="busy">
        {{ loading ? '查询中...' : '查询' }}
      </button>
      <button class="btn btn-secondary" type="button" :disabled="busy" @click="resetToToday">
        重置为今天
      </button>
      <button class="btn btn-secondary" type="button" :disabled="busy" @click="clearDateRange">
        清空日期
      </button>
    </form>

    <div class="form-actions" style="margin-bottom: 12px">
      <button class="btn" type="button" :disabled="busy || !canExport" @click="exportReport('csv')">
        {{ exportingCsv ? '导出 CSV 中...' : '导出 CSV' }}
      </button>
      <button class="btn" type="button" :disabled="busy || !canExport" @click="exportReport('xlsx')">
        {{ exportingXlsx ? '导出 XLSX 中...' : '导出 XLSX' }}
      </button>
    </div>

    <p v-if="errorText" class="error-text">{{ errorText }}</p>
    <p v-if="successText" class="success-text">{{ successText }}</p>

    <p class="table-summary">
      共 {{ total }} 条，当前第 {{ currentPage }} 页，分组方式：product
    </p>

    <div v-if="reportSummary" class="report-summary-grid">
      <article class="metric-card">
        <p class="metric-label">总销售额</p>
        <p class="metric-value">¥ {{ reportSummary.total_sales }}</p>
      </article>

      <article v-if="canViewCostMetrics" class="metric-card">
        <p class="metric-label">总成本</p>
        <p class="metric-value">¥ {{ reportSummary.total_cost }}</p>
      </article>

      <article v-if="canViewCostMetrics" class="metric-card">
        <p class="metric-label">总毛利</p>
        <p class="metric-value">¥ {{ reportSummary.total_gross_profit }}</p>
      </article>

      <article class="metric-card">
        <p class="metric-label">总销量</p>
        <p class="metric-value">{{ reportSummary.total_qty }}</p>
      </article>
    </div>

    <div class="table-wrapper">
      <table class="data-table">
        <thead>
          <tr>
            <th>商品ID</th>
            <th>商品名称</th>
            <th>净销量</th>
            <th>销售额</th>
            <th v-if="canViewCostMetrics">成本</th>
            <th v-if="canViewCostMetrics">毛利</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="item in reportList" :key="item.product_id">
            <td>{{ item.product_id }}</td>
            <td>{{ item.product_name }}</td>
            <td>{{ item.total_qty }}</td>
            <td>{{ item.total_sales }}</td>
            <td v-if="canViewCostMetrics">{{ item.total_cost }}</td>
            <td v-if="canViewCostMetrics">{{ item.gross_profit }}</td>
          </tr>
          <tr v-if="!loading && reportList.length === 0">
            <td :colspan="canViewCostMetrics ? 6 : 4" class="empty-cell">暂无数据</td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="pager">
      <button class="btn btn-secondary" :disabled="busy || currentPage <= 1" @click="prevPage">
        上一页
      </button>
      <button
        class="btn btn-secondary"
        :disabled="busy || currentPage * pageSize >= total"
        @click="nextPage"
      >
        下一页
      </button>
    </div>
  </section>
</template>
