<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'

import { listLowStockAlertsApi } from '@/api/inventory'
import { ApiClientError } from '@/api/http'
import type { LowStockAlertData } from '@/types/api'

const loading = ref(false)
const errorText = ref('')

const query = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  only_active: true,
})

const alerts = ref<LowStockAlertData[]>([])
const total = ref(0)
const activeCount = ref(0)

async function fetchAlerts(): Promise<void> {
  loading.value = true
  errorText.value = ''

  try {
    const response = await listLowStockAlertsApi({
      page: query.page,
      page_size: query.page_size,
      keyword: query.keyword.trim() || undefined,
      only_active: query.only_active,
    })
    alerts.value = response.data.list
    total.value = response.data.total
    activeCount.value = response.data.active_low_stock_count
  } catch (error) {
    if (error instanceof ApiClientError) {
      errorText.value = `${error.message}（code=${error.code}${error.requestId ? `, request_id=${error.requestId}` : ''}）`
    } else {
      errorText.value = error instanceof Error ? error.message : '低库存查询失败'
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
  void fetchAlerts()
}

function nextPage(): void {
  if (query.page * query.page_size >= total.value) {
    return
  }
  query.page += 1
  void fetchAlerts()
}

onMounted(() => {
  void fetchAlerts()
})
</script>

<template>
  <section>
    <h2>低库存预警</h2>

    <form class="form-inline" @submit.prevent="fetchAlerts">
      <label class="form-label inline">
        <span>关键字</span>
        <input v-model="query.keyword" placeholder="名称 / SKU / 条码" />
      </label>

      <label class="checkbox-inline">
        <input v-model="query.only_active" type="checkbox" />
        <span>仅显示预警项</span>
      </label>

      <button class="btn" type="submit" :disabled="loading">查询</button>
    </form>

    <p v-if="errorText" class="error-text">{{ errorText }}</p>

    <p class="table-summary">
      共 {{ total }} 条，预警中 {{ activeCount }} 条，当前第 {{ query.page }} 页
    </p>

    <div class="table-wrapper">
      <table class="data-table">
        <thead>
          <tr>
            <th>商品ID</th>
            <th>SKU</th>
            <th>条码</th>
            <th>名称</th>
            <th>当前库存</th>
            <th>阈值</th>
            <th>缺口</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="item in alerts" :key="item.product_id">
            <td>{{ item.product_id }}</td>
            <td>{{ item.sku }}</td>
            <td>{{ item.barcode }}</td>
            <td>{{ item.name }}</td>
            <td>{{ item.current_stock }}</td>
            <td>{{ item.min_stock_limit }}</td>
            <td :class="{ 'warn-text': item.shortage_qty > 0 }">{{ item.shortage_qty }}</td>
          </tr>
          <tr v-if="!loading && alerts.length === 0">
            <td colspan="7" class="empty-cell">暂无数据</td>
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
  </section>
</template>
