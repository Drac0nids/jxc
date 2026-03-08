<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'

import { listAuditLogsApi } from '@/api/inventory'
import { ApiClientError } from '@/api/http'
import { useAuthStore } from '@/stores/auth'
import type { AuditLogData } from '@/types/api'

const authStore = useAuthStore()

const loading = ref(false)
const errorText = ref('')

const query = reactive({
  page: 1,
  page_size: 20,
  action: '',
  target_type: '',
  operator_id: '',
  request_id: '',
  start_date: '',
  end_date: '',
})

const logs = ref<AuditLogData[]>([])
const total = ref(0)

const isOwner = computed(() => authStore.session?.user.role === 'OWNER')

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
  if (!isOwner.value) {
    errorText.value = '当前角色无审计日志查询权限，仅限 OWNER。'
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
    const operatorId = query.operator_id.trim()
    const requestId = query.request_id.trim()
    const startDate = query.start_date.trim()
    const endDate = query.end_date.trim()

    const response = await listAuditLogsApi({
      page: query.page,
      page_size: query.page_size,
      action: query.action.trim() || undefined,
      target_type: query.target_type.trim() || undefined,
      operator_id: operatorId || undefined,
      request_id: requestId || undefined,
      start_date: startDate || undefined,
      end_date: endDate || undefined,
    })
    logs.value = response.data.list
    total.value = response.data.total
  } catch (error) {
    if (error instanceof ApiClientError) {
      errorText.value = `${error.message}（code=${error.code}${error.requestId ? `, request_id=${error.requestId}` : ''}）`
    } else {
      errorText.value = error instanceof Error ? error.message : '审计日志查询失败'
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
  query.action = ''
  query.target_type = ''
  query.operator_id = ''
  query.request_id = ''
  query.start_date = ''
  query.end_date = ''
  query.page = 1
  void fetchLogs()
}

onMounted(() => {
  if (isOwner.value) {
    const today = formatToday()
    query.start_date = today
    query.end_date = today
    void fetchLogs()
  }
})
</script>

<template>
  <section>
    <h2>审计日志</h2>

    <p v-if="!isOwner" class="warn-text">当前角色无权限查看审计日志，仅 OWNER 可访问。</p>

    <div v-else>
      <form class="card-panel form-grid" @submit.prevent="fetchLogs()">
        <div class="form-inline form-inline-compact">
          <label class="form-label inline">
            <span>动作类型</span>
            <input v-model="query.action" placeholder="例如：STOCK_CHECK_CONFIRM" />
          </label>
          <label class="form-label inline">
            <span>目标类型</span>
            <input v-model="query.target_type" placeholder="例如：product" />
          </label>
          <label class="form-label inline">
            <span>Request ID</span>
            <input v-model="query.request_id" placeholder="精确匹配" />
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
          <button class="btn" type="submit" :disabled="loading">查询审计</button>
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
              <th>动作</th>
              <th>目标</th>
              <th>目标ID</th>
              <th>操作人ID</th>
              <th>Request ID</th>
            </tr>
          </thead>
          <tbody>
            <template v-for="log in logs" :key="log.id">
              <tr>
                <td>{{ log.id }}</td>
                <td>{{ log.created_at }}</td>
                <td><strong>{{ log.action }}</strong></td>
                <td>{{ log.target_type }}</td>
                <td>{{ log.target_id }}</td>
                <td><small>{{ log.operator_id }}</small></td>
                <td><small>{{ log.request_id }}</small></td>
              </tr>
              <!-- Optional: detail row for data diffs -->
            </template>
            <tr v-if="!loading && logs.length === 0">
              <td colspan="7" class="empty-cell">暂无审计记录</td>
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
