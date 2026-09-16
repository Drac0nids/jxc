<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { listStockCheckLogsApi, type StockCheckLogData } from '@/api/analytics'
import { ApiClientError } from '@/api/http'

const loading = ref(false)
const errorText = ref('')
const logs = ref<StockCheckLogData[]>([])
const total = ref(0)
const page = ref(1)
const pageSize = 20

const filter = reactive({ start_date: '', end_date: '' })

function fmtError(e: unknown, fb: string) {
  if (e instanceof ApiClientError) return `${e.message}（code=${e.code}）`
  return e instanceof Error ? e.message : fb
}

function statusLabel(s: string) {
  return { DRAFT: '草稿', COUNTING: '盘点中', CONFIRMED: '已确认' }[s] ?? s
}
function statusClass(s: string) {
  return { CONFIRMED: 'badge-ok', COUNTING: 'badge-warning', DRAFT: 'badge-muted' }[s] ?? 'badge-muted'
}

async function fetchLogs(resetPage = false) {
  if (resetPage) page.value = 1
  loading.value = true; errorText.value = ''
  try {
    const res = await listStockCheckLogsApi({
      page: page.value,
      page_size: pageSize,
      start_date: filter.start_date || undefined,
      end_date: filter.end_date || undefined,
    })
    logs.value = res.data.list
    total.value = res.data.total
  } catch (e) {
    errorText.value = fmtError(e, '查询盘点日志失败')
  } finally {
    loading.value = false
  }
}

function prevPage() { if (page.value > 1) { page.value--; void fetchLogs() } }
function nextPage() { if (page.value < Math.ceil(total.value / pageSize)) { page.value++; void fetchLogs() } }
function resetFilter() { filter.start_date = ''; filter.end_date = ''; void fetchLogs(true) }

onMounted(() => { void fetchLogs() })
</script>

<template>
  <section>
    <h2>盘点日志</h2>

    <div class="card-panel">
      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>开始日期</span>
          <input v-model="filter.start_date" type="date" />
        </label>
        <label class="form-label inline">
          <span>结束日期</span>
          <input v-model="filter.end_date" type="date" />
        </label>
      </div>
      <div class="form-actions">
        <button class="btn" :disabled="loading" @click="fetchLogs(true)">{{ loading ? '查询中...' : '查询' }}</button>
        <button class="btn btn-secondary" @click="resetFilter">重置</button>
      </div>
    </div>

    <p v-if="errorText" class="error-text">{{ errorText }}</p>
    <p class="table-summary">共 {{ total }} 条（第 {{ page }} 页）</p>

    <div class="table-wrapper">
      <table class="data-table">
        <thead>
          <tr>
            <th>ID</th>
            <th>业务单号</th>
            <th>状态</th>
            <th>商品数</th>
            <th>经手人</th>
            <th>备注</th>
            <th>确认时间</th>
            <th>创建时间</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="log in logs" :key="log.id">
            <td>{{ log.id }}</td>
            <td><small>{{ log.biz_no }}</small></td>
            <td><span class="badge" :class="statusClass(log.status)">{{ statusLabel(log.status) }}</span></td>
            <td>{{ log.item_count }}</td>
            <td>{{ log.operator_name }}</td>
            <td>{{ log.remark ?? '—' }}</td>
            <td><small>{{ log.confirmed_at ? log.confirmed_at.substring(0, 16).replace('T', ' ') : '—' }}</small></td>
            <td><small>{{ log.created_at.substring(0, 16).replace('T', ' ') }}</small></td>
          </tr>
          <tr v-if="!loading && logs.length === 0">
            <td colspan="8" class="empty-cell">暂无盘点记录</td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="pagination">
      <button class="btn btn-secondary" :disabled="page <= 1 || loading" @click="prevPage">上一页</button>
      <span class="page-info">第 {{ page }} 页 / 共 {{ Math.max(1, Math.ceil(total / pageSize)) }} 页</span>
      <button class="btn btn-secondary" :disabled="page >= Math.ceil(total / pageSize) || loading" @click="nextPage">下一页</button>
    </div>
  </section>
</template>
