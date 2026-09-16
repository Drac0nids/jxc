<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { getTrendApi, type TrendDayData } from '@/api/analytics'
import { ApiClientError } from '@/api/http'

const loading = ref(false)
const errorText = ref('')
const days = ref<TrendDayData[]>([])
const mode = ref<'sales' | 'profit'>('sales')

function today() {
  return new Date().toISOString().substring(0, 10)
}

function startOf7Days() {
  const d = new Date()
  d.setDate(d.getDate() - 6)
  return d.toISOString().substring(0, 10)
}

function fmtError(e: unknown, fb: string) {
  if (e instanceof ApiClientError) return `${e.message}（code=${e.code}）`
  return e instanceof Error ? e.message : fb
}

async function fetchTrend() {
  loading.value = true; errorText.value = ''
  try {
    const res = await getTrendApi({ start_date: startOf7Days(), end_date: today() })
    days.value = res.data.days
  } catch (e) {
    errorText.value = fmtError(e, '查询趋势失败')
  } finally {
    loading.value = false
  }
}

// ── SVG chart helpers ─────────────────────────────────────────────────────────

const CHART_W = 560
const CHART_H = 180
const PAD = { top: 16, right: 16, bottom: 32, left: 56 }

const innerW = CHART_W - PAD.left - PAD.right
const innerH = CHART_H - PAD.top - PAD.bottom

function vals(key: 'total_sales' | 'total_gross_profit') {
  return days.value.map(d => parseFloat(d[key]) || 0)
}

function makeChart(key: 'total_sales' | 'total_gross_profit') {
  const data = vals(key)
  if (!data.length) return null
  const max = Math.max(...data, 1)
  const n = data.length

  const points = data.map((v, i) => ({
    x: PAD.left + (i / Math.max(n - 1, 1)) * innerW,
    y: PAD.top + innerH - (v / max) * innerH,
    v, label: days.value[i]?.date.substring(5) ?? '',
  }))

  const first = points[0]
  const last = points[points.length - 1]
  if (!first || !last) return null

  const polyline = points.map(p => `${p.x},${p.y}`).join(' ')
  const area = [
    `${first.x},${PAD.top + innerH}`,
    ...points.map(p => `${p.x},${p.y}`),
    `${last.x},${PAD.top + innerH}`,
  ].join(' ')

  // Y axis labels
  const yTicks = [0, 0.25, 0.5, 0.75, 1].map(t => ({
    y: PAD.top + innerH - t * innerH,
    label: fmtNum(max * t),
  }))

  return { points, polyline, area, yTicks, max }
}

function fmtNum(v: number) {
  if (v >= 10000) return `${(v / 10000).toFixed(1)}w`
  if (v >= 1000) return `${(v / 1000).toFixed(1)}k`
  return v.toFixed(0)
}

const chart = computed(() =>
  mode.value === 'sales'
    ? makeChart('total_sales')
    : makeChart('total_gross_profit')
)
const chartColor = computed(() => mode.value === 'sales' ? '#10b981' : '#6366f1')
const chartColorSoft = computed(() => mode.value === 'sales' ? 'rgba(16,185,129,0.12)' : 'rgba(99,102,241,0.12)')

// Summary metrics
const totalSales = computed(() => vals('total_sales').reduce((a, b) => a + b, 0))
const totalProfit = computed(() => vals('total_gross_profit').reduce((a, b) => a + b, 0))
const totalOrders = computed(() => days.value.reduce((a, d) => a + d.total_orders, 0))
const profitRate = computed(() => totalSales.value > 0 ? (totalProfit.value / totalSales.value * 100).toFixed(1) : '0.0')

onMounted(() => { void fetchTrend() })
</script>

<template>
  <section>
    <h2>经营趋势（近7天）</h2>

    <!-- 汇总卡片 -->
    <div class="stats-row" v-if="!loading && days.length">
      <div class="stat-card" style="background:#e7faf2;color:#059669;border-color:#a7f3d0">
        <div class="stat-num">¥{{ fmtNum(totalSales) }}</div>
        <div class="stat-label">累计销售额</div>
      </div>
      <div class="stat-card" style="background:#ede9fe;color:#4f46e5;border-color:#c4b5fd">
        <div class="stat-num">¥{{ fmtNum(totalProfit) }}</div>
        <div class="stat-label">累计毛利</div>
      </div>
      <div class="stat-card" style="background:#eff6ff;color:#2563eb;border-color:#bfdbfe">
        <div class="stat-num">{{ totalOrders }}</div>
        <div class="stat-label">总订单数</div>
      </div>
      <div class="stat-card" style="background:#fefce8;color:#ca8a04;border-color:#fde68a">
        <div class="stat-num">{{ profitRate }}%</div>
        <div class="stat-label">综合毛利率</div>
      </div>
    </div>

    <!-- 切换 + 刷新 -->
    <div class="form-inline form-inline-compact" style="margin-bottom:14px">
      <div class="scan-mode-tabs">
        <button class="scan-mode-tab" :class="{ 'is-active': mode === 'sales' }" @click="mode = 'sales'">销售额趋势</button>
        <button class="scan-mode-tab" :class="{ 'is-active': mode === 'profit' }" @click="mode = 'profit'">毛利趋势</button>
      </div>
      <button class="btn btn-secondary" :disabled="loading" @click="fetchTrend">{{ loading ? '加载中...' : '刷新' }}</button>
    </div>

    <p v-if="errorText" class="error-text">{{ errorText }}</p>

    <!-- SVG 折线图 -->
    <div class="card-panel" v-if="chart && days.length">
      <svg :width="CHART_W" :height="CHART_H" style="overflow:visible;max-width:100%">
        <!-- 网格 -->
        <line v-for="t in chart.yTicks" :key="t.y"
          :x1="PAD.left" :y1="t.y" :x2="PAD.left + innerW" :y2="t.y"
          stroke="#e7ebf4" stroke-width="1" />
        <!-- Y轴刻度 -->
        <text v-for="t in chart.yTicks" :key="'l' + t.y"
          :x="PAD.left - 6" :y="t.y + 4"
          text-anchor="end" font-size="10" fill="#9aa5ba">{{ t.label }}</text>
        <!-- 面积 -->
        <polygon :points="chart.area" :fill="chartColorSoft" />
        <!-- 折线 -->
        <polyline :points="chart.polyline" :stroke="chartColor" stroke-width="2.5" fill="none" stroke-linejoin="round" stroke-linecap="round" />
        <!-- 数据点 -->
        <g v-for="p in chart.points" :key="p.x">
          <circle :cx="p.x" :cy="p.y" r="4" :fill="chartColor" stroke="white" stroke-width="2" />
          <!-- Tooltip label -->
          <text :x="p.x" :y="p.y - 10" text-anchor="middle" font-size="10" :fill="chartColor" font-weight="600">
            {{ fmtNum(p.v) }}
          </text>
        </g>
        <!-- X轴标签 -->
        <text v-for="p in chart.points" :key="'x' + p.x"
          :x="p.x" :y="PAD.top + innerH + 20"
          text-anchor="middle" font-size="10" fill="#9aa5ba">{{ p.label }}</text>
      </svg>
    </div>

    <!-- 明细表 -->
    <div class="table-wrapper" style="margin-top:16px" v-if="days.length">
      <table class="data-table">
        <thead>
          <tr>
            <th>日期</th>
            <th>销售额</th>
            <th>毛利</th>
            <th>毛利率</th>
            <th>订单数</th>
            <th>热销商品</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="d in days" :key="d.date">
            <td>{{ d.date }}</td>
            <td>¥{{ parseFloat(d.total_sales).toFixed(2) }}</td>
            <td>¥{{ parseFloat(d.total_gross_profit).toFixed(2) }}</td>
            <td>
              <span :style="{ color: parseFloat(d.total_sales) > 0 ? '#059669' : '#9aa5ba' }">
                {{ parseFloat(d.total_sales) > 0
                  ? (parseFloat(d.total_gross_profit) / parseFloat(d.total_sales) * 100).toFixed(1) + '%'
                  : '—' }}
              </span>
            </td>
            <td>{{ d.total_orders }}</td>
            <td>{{ d.top_selling_item || '—' }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <p v-if="!loading && !days.length && !errorText" class="empty-cell" style="margin-top:24px">暂无趋势数据</p>
  </section>
</template>
