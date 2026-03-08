<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'

import { listStockLogsApi } from '@/api/inventory'
import { ApiClientError } from '@/api/http'
import { useAuthStore } from '@/stores/auth'
import type { StockLogData } from '@/types/api'

const authStore = useAuthStore()

const loading = ref(false)
const errorText = ref('')

const query = reactive({
  page: 1,
  page_size: 20,
  biz_type: '',
  biz_no: '',
  product_id: '',
  operator_id: '',
  start_date: '',
  end_date: '',
})

const logs = ref<StockLogData[]>([])
const total = ref(0)

const canView = computed(() => {
  const role = authStore.session?.user.role
  return role === 'OWNER' || role === 'PURCHASER'
})

function validateUuid(value: string, label: string): string | null {
  const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i
  if (!uuidRegex.test(value)) {
    return `${label}格式错误，应为 UUID`
  }
  return null
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

function validateQuery(): string | null {
  const productId = query.product_id.trim()
  if (productId) {
    const value = Number(productId)
    if (!Number.isInteger(value) || value <= 0) {
      return '商品ID必须为正整数'
    }
  }

  const operatorId = query.operator_id.trim()
  if (operatorId) {
    const uuidError = validateUuid(operatorId, '操作人ID')
    if (uuidError) {
      return uuidError
    }
  }

  const startDate = query.start_date.trim()
  const endDate = query.end_date.trim()
  const hasStart = Boolean(startDate)
  const hasEnd = Boolean(endDate)
  if (hasStart !== hasEnd) {
    return '开始日期与结束日期需同时提供'
  }

  if (hasStart && hasEnd) {
    const startError = validateDate(startDate, '开始日期')
    if (startError) {
      return startError
    }

    const endError = validateDate(endDate, '结束日期')
    if (endError) {
      return endError
    }

    if (startDate > endDate) {
      return '开始日期不能晚于结束日期'
    }
  }

  return null
}

function formatToday(): string {
  const now = new Date()
  const pad = (value: number): string => value.toString().padStart(2, '0')
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
}

async function fetchLogs(options?: { keepPage?: boolean }): Promise<void> {
  if (!canView.value) {
    errorText.value = '当前角色无库存流水查询权限，仅限 OWNER/PURCHASER。'
    return
  }

  const validationError = validateQuery()
  if (validationError) {
    errorText.value = validationError
    return
  }

  if (!options?.keepPage) {
    query.page = 1
  }

  loading.value = true
  errorText.value = ''

  try {
    const productId = query.product_id.trim()
    const operatorId = query.operator_id.trim()
    const startDate = query.start_date.trim()
    const endDate = query.end_date.trim()

    const response = await listStockLogsApi({
      page: query.page,
      page_size: query.page_size,
      biz_type: query.biz_type.trim() || undefined,
      biz_no: query.biz_no.trim() || undefined,
      product_id: productId ? Number(productId) : undefined,
      operator_id: operatorId || undefined,
      start_date: startDate || undefined,
      end_date: endDate || undefined,
    })
    logs.value = response.data.list
    total.value = response.data.total
  } catch (error) {
    if (error instanceof ApiClientError) {
      errorText.value = `${error.message}（code=${error.code}${error.requestId ? `, request_id=${error.requestId}` : ''}）`
    } else {
      errorText.value = error instanceof Error ? error.message : '库存流水查询失败'
    }
  } finally {
    loading.value = false
  }
}

function prevPage(): void {
  if (query.page <= 1) {
    return
  }
  query.page -= 1
  void fetchLogs({ keepPage: true })
}

function nextPage(): void {
  if (query.page * query.page_size >= total.value) {
    return
  }
  query.page += 1
  void fetchLogs({ keepPage: true })
}

function resetFilters(): void {
  query.biz_type = ''
  query.biz_no = ''
  query.product_id = ''
  query.operator_id = ''
  query.start_date = ''
  query.end_date = ''
  query.page = 1
  void fetchLogs()
}

onMounted(() => {
  if (canView.value) {
    const today = formatToday()
    query.start_date = today
    query.end_date = today
    void fetchLogs()
  }
})
</script>

<template>
  <section>
    <h2>库存流水</h2>

    <p v-if="!canView" class="warn-text">当前角色无权限查看库存流水，仅 OWNER/PURCHASER 可访问。</p>

    <div v-else>
      <form class="card-panel form-grid" @submit.prevent="fetchLogs()">
        <div class="form-inline form-inline-compact">
          <label class="form-label inline">
            <span>业务类型</span>
            <input v-model="query.biz_type" placeholder="例如：IN_PURCHASE" />
          </label>
          <label class="form-label inline">
            <span>业务单号</span>
            <input v-model="query.biz_no" placeholder="模糊匹配" />
          </label>
          <label class="form-label inline">
            <span>商品ID</span>
            <input v-model="query.product_id" placeholder="数字ID" />
          </label>
        </div>

        <div class="form-inline form-inline-compact">
          <label class="form-label inline">
            <span>开始日期</span>
            <input v-model="query.start_date" placeholder="YYYY-MM-DD" />
          </label>
          <label class="form-label inline">
            <span>结束日期</span>
            <input v-model="query.end_date" placeholder="YYYY-MM-DD" />
          </label>
          <label class="form-label inline">
            <span>操作人ID</span>
            <input v-model="query.operator_id" placeholder="UUID" />
          </label>
        </div>

        <div class="form-actions">
          <button class="btn" type="submit" :disabled="loading">查询流水</button>
          <button class="btn btn-secondary" type="button" @click="resetFilters">重置条件</button>
        </div>
      </form>

      <p v-if="errorText" class="error-text">{{ errorText }}</p>

      <p class="table-summary">共 {{ total }} 条记录，当前第 {{ query.page }} 页</p>

      <div class="table-wrapper">
        <table class="data-table">
          <thead>
            <tr>
              <th>ID</th>
              <th>时间</th>
              <th>商品ID</th>
              <th>业务类型</th>
              <th>业务单号</th>
              <th>数量变化</th>
              <th>结存库存</th>
              <th>结存成本</th>
              <th>操作人</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="log in logs" :key="log.id">
              <td>{{ log.id }}</td>
              <td>{{ log.created_at }}</td>
              <td>{{ log.product_id }}</td>
              <td><strong>{{ log.biz_type }}</strong></td>
              <td>{{ log.biz_no }}</td>
              <td :class="log.delta_qty > 0 ? 'success-text' : log.delta_qty < 0 ? 'warn-text' : ''">
                {{ log.delta_qty > 0 ? '+' : '' }}{{ log.delta_qty }}
              </td>
              <td>{{ log.snapshot_stock }}</td>
              <td>¥ {{ log.snapshot_cost }}</td>
              <td><small>{{ log.operator_id }}</small></td>
            </tr>
            <tr v-if="!loading && logs.length === 0">
              <td colspan="9" class="empty-cell">暂无流水记录</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="pager">
        <button class="btn btn-secondary" :disabled="loading || query.page <= 1" @click="prevPage">
          上一页
        </button>
        <button
          class="btn btn-secondary"
          :disabled="loading || query.page * query.page_size >= total"
          @click="nextPage"
        >
          下一页
        </button>
      </div>
    </div>
  </section>
</template>
