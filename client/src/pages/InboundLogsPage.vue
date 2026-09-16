<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { listInboundLogsApi, type InboundLogData } from '@/api/catalog'
import { ApiClientError } from '@/api/http'

const loading = ref(false)
const errorText = ref('')
const logs = ref<InboundLogData[]>([])
const total = ref(0)
const page = ref(1)
const pageSize = 20

const filter = reactive({
  keyword: '',
  start_date: '',
  end_date: '',
})

function fmtError(e: unknown, fb: string) {
  if (e instanceof ApiClientError) return `${e.message}（code=${e.code}）`
  return e instanceof Error ? e.message : fb
}

async function fetchLogs(resetPage = false) {
  if (resetPage) page.value = 1
  loading.value = true; errorText.value = ''
  try {
    const res = await listInboundLogsApi({
      page: page.value,
      page_size: pageSize,
      keyword: filter.keyword.trim() || undefined,
      start_date: filter.start_date || undefined,
      end_date: filter.end_date || undefined,
    })
    logs.value = res.data.list
    total.value = res.data.total
  } catch (e) {
    errorText.value = fmtError(e, '查询入库日志失败')
  } finally {
    loading.value = false
  }
}

function prevPage() { if (page.value > 1) { page.value--; void fetchLogs() } }
function nextPage() {
  const maxPage = Math.ceil(total.value / pageSize)
  if (page.value < maxPage) { page.value++; void fetchLogs() }
}

function resetFilter() {
  filter.keyword = ''
  filter.start_date = ''
  filter.end_date = ''
  void fetchLogs(true)
}

onMounted(() => { void fetchLogs() })
</script>

<template>
  <section>
    <h2>入库记录</h2>

    <!-- 搜索栏 -->
    <div class="card-panel">
      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>关键词</span>
          <input v-model="filter.keyword" placeholder="商品名/SKU/经手人" style="width:180px" />
        </label>
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
        <button class="btn" :disabled="loading" @click="fetchLogs(true)">
          {{ loading ? '查询中...' : '查询' }}
        </button>
        <button class="btn btn-secondary" :disabled="loading" @click="resetFilter">重置</button>
      </div>
    </div>

    <p v-if="errorText" class="error-text">{{ errorText }}</p>
    <p class="table-summary">共 {{ total }} 条入库记录（第 {{ page }} 页）</p>

    <div class="table-wrapper">
      <table class="data-table">
        <thead>
          <tr>
            <th>ID</th>
            <th>商品名称</th>
            <th>SKU</th>
            <th>入库数量</th>
            <th>单价</th>
            <th>合计金额</th>
            <th>经手人</th>
            <th>备注</th>
            <th>时间</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="log in logs" :key="log.id">
            <td>{{ log.id }}</td>
            <td>{{ log.product_name }}</td>
            <td><small>{{ log.product_sku }}</small></td>
            <td>{{ log.quantity }}</td>
            <td>¥{{ log.unit_cost }}</td>
            <td>¥{{ log.total_cost }}</td>
            <td>{{ log.operator_name }}</td>
            <td>{{ log.notes ?? '—' }}</td>
            <td><small>{{ log.created_at.substring(0, 16).replace('T', ' ') }}</small></td>
          </tr>
          <tr v-if="!loading && logs.length === 0">
            <td colspan="9" class="empty-cell">暂无入库记录</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 分页 -->
    <div class="pagination">
      <button class="btn btn-secondary" :disabled="page <= 1 || loading" @click="prevPage">上一页</button>
      <span class="page-info">第 {{ page }} 页 / 共 {{ Math.max(1, Math.ceil(total / pageSize)) }} 页</span>
      <button class="btn btn-secondary" :disabled="page >= Math.ceil(total / pageSize) || loading" @click="nextPage">下一页</button>
    </div>
  </section>
</template>
