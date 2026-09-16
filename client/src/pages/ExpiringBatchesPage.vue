<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { listExpiringBatchesApi } from '@/api/catalog'
import { ApiClientError } from '@/api/http'
import type { ExpiringBatchData } from '@/types/api'

const loading = ref(false)
const errorText = ref('')
const items = ref<ExpiringBatchData[]>([])
const withinDays = ref(30)

function fmtError(e: unknown, fb: string) {
  if (e instanceof ApiClientError) return `${e.message}（code=${e.code}）`
  return e instanceof Error ? e.message : fb
}

function expiryClass(level: string | null) {
  return { EXPIRED: 'text-danger', CRITICAL: 'text-danger', WARNING: 'text-warning', NOTICE: 'text-notice' }[level ?? ''] ?? ''
}

function expiryLabel(level: string | null) {
  return { EXPIRED: '已过期', CRITICAL: '紧急（<7天）', WARNING: '警告（<15天）', NOTICE: '注意（<30天）' }[level ?? ''] ?? '正常'
}

async function fetchExpiring() {
  loading.value = true; errorText.value = ''
  try {
    const res = await listExpiringBatchesApi(withinDays.value)
    items.value = res.data
  } catch (e) {
    errorText.value = fmtError(e, '查询失败')
  } finally {
    loading.value = false
  }
}

const expiredCount = () => items.value.filter(i => i.batch.expiry_level === 'EXPIRED').length
const criticalCount = () => items.value.filter(i => i.batch.expiry_level === 'CRITICAL').length
const warningCount = () => items.value.filter(i => ['WARNING', 'NOTICE'].includes(i.batch.expiry_level ?? '')).length

onMounted(() => { void fetchExpiring() })
</script>

<template>
  <section>
    <h2>即将过期批次</h2>

    <!-- 统计卡片 -->
    <div class="stats-row" v-if="!loading && items.length">
      <div class="stat-card stat-danger">
        <div class="stat-num">{{ expiredCount() }}</div>
        <div class="stat-label">已过期</div>
      </div>
      <div class="stat-card stat-warning">
        <div class="stat-num">{{ criticalCount() }}</div>
        <div class="stat-label">7天内到期</div>
      </div>
      <div class="stat-card stat-notice">
        <div class="stat-num">{{ warningCount() }}</div>
        <div class="stat-label">30天内到期</div>
      </div>
    </div>

    <!-- 筛选 -->
    <div class="form-inline form-inline-compact" style="margin: 12px 0;">
      <label class="form-label inline">
        <span>预警天数</span>
        <select v-model="withinDays" style="width:120px">
          <option :value="7">7天</option>
          <option :value="15">15天</option>
          <option :value="30">30天</option>
          <option :value="60">60天</option>
          <option :value="90">90天</option>
        </select>
      </label>
      <button class="btn btn-secondary" :disabled="loading" @click="fetchExpiring">
        {{ loading ? '加载中...' : '刷新' }}
      </button>
    </div>

    <p v-if="errorText" class="error-text">{{ errorText }}</p>
    <p class="table-summary">共 {{ items.length }} 个批次需关注</p>

    <div class="table-wrapper">
      <table class="data-table">
        <thead>
          <tr>
            <th>商品名称</th>
            <th>SKU</th>
            <th>批次号</th>
            <th>供应商</th>
            <th>生产日期</th>
            <th>过期日期</th>
            <th>剩余天数</th>
            <th>预警级别</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="item in items" :key="item.batch.id">
            <td>{{ item.product_name }}</td>
            <td><small>{{ item.product_sku }}</small></td>
            <td>{{ item.batch.lot_number }}</td>
            <td>{{ item.batch.supplier ?? '—' }}</td>
            <td><small>{{ item.batch.produced_at ? item.batch.produced_at.substring(0, 10) : '—' }}</small></td>
            <td>
              <span :class="expiryClass(item.batch.expiry_level)">
                {{ item.batch.expires_at ? item.batch.expires_at.substring(0, 10) : '—' }}
              </span>
            </td>
            <td>
              <span :class="expiryClass(item.batch.expiry_level)" style="font-weight:600">
                {{ item.batch.days_until_expiry !== null ? item.batch.days_until_expiry + ' 天' : '—' }}
              </span>
            </td>
            <td>
              <span class="badge" :class="{
                'badge-danger': item.batch.expiry_level === 'EXPIRED' || item.batch.expiry_level === 'CRITICAL',
                'badge-warning': item.batch.expiry_level === 'WARNING',
                'badge-notice': item.batch.expiry_level === 'NOTICE',
              }">{{ expiryLabel(item.batch.expiry_level) }}</span>
            </td>
          </tr>
          <tr v-if="!loading && items.length === 0">
            <td colspan="8" class="empty-cell">✅ 近期无即将过期批次</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
