<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import {
  getSerialHistoryApi,
  serialInboundApi,
  serialOutboundApi,
  type SerialNumber,
} from '@/api/analytics'
import { listProductsApi } from '@/api/products'
import { ApiClientError } from '@/api/http'
import type { ProductData } from '@/types/api'

// ── State ─────────────────────────────────────────────────────────────────────
const tab = ref<'inbound' | 'outbound' | 'query'>('inbound')
const loading = ref(false)
const saving = ref(false)
const errorText = ref('')
const successText = ref('')
const products = ref<ProductData[]>([])

// Inbound
const inForm = reactive({
  product_id: '',
  batch_id: '',
  unit_cost: '',
  snText: '',   // 多行SN，每行一个
})
const inResult = ref<{ biz_no: string; count: number } | null>(null)

// Outbound
const outForm = reactive({ sell_price: '', snText: '' })
const outResult = ref<{ biz_no: string; count: number; list: SerialNumber[] } | null>(null)

// Query / History
const qFilter = reactive({
  sn: '', product_id: '', status: '',
  start_date: '', end_date: '',
  page: 1,
})
const pageSize = 20
const historyList = ref<SerialNumber[]>([])
const historyTotal = ref(0)

function fmtError(e: unknown, fb: string) {
  if (e instanceof ApiClientError) return `${e.message}（code=${e.code}）`
  return e instanceof Error ? e.message : fb
}

function statusLabel(s: string) {
  return { IN_STOCK: '在库', SOLD: '已售', RETURNED: '已退' }[s] ?? s
}
function statusClass(s: string) {
  return { IN_STOCK: 'badge-ok', SOLD: 'badge-muted', RETURNED: 'badge-warning' }[s] ?? 'badge-muted'
}

function parseSns(text: string): string[] {
  return text.split(/[\n,\s]+/).map(s => s.trim()).filter(Boolean)
}

// ── Fetch Products ─────────────────────────────────────────────────────────────
async function fetchProducts() {
  try {
    const res = await listProductsApi({ page_size: 500 })
    products.value = res.data.list
  } catch (_) { /* ignore */ }
}

// ── Inbound ───────────────────────────────────────────────────────────────────
async function submitInbound() {
  const sns = parseSns(inForm.snText)
  if (!inForm.product_id) { errorText.value = '请选择商品'; return }
  if (!sns.length) { errorText.value = '请输入至少一个 SN 码'; return }
  saving.value = true; errorText.value = ''; successText.value = ''; inResult.value = null
  try {
    const res = await serialInboundApi({
      product_id: Number(inForm.product_id),
      batch_id: inForm.batch_id || undefined,
      unit_cost: inForm.unit_cost || undefined,
      sns,
    })
    inResult.value = res.data
    successText.value = `入库成功：单号 ${res.data.biz_no}，共 ${res.data.count} 个 SN`
    inForm.snText = ''
  } catch (e) {
    errorText.value = fmtError(e, '序列号入库失败')
  } finally {
    saving.value = false
  }
}

// ── Outbound ──────────────────────────────────────────────────────────────────
async function submitOutbound() {
  const sns = parseSns(outForm.snText)
  if (!sns.length) { errorText.value = '请输入至少一个 SN 码'; return }
  saving.value = true; errorText.value = ''; successText.value = ''; outResult.value = null
  try {
    const res = await serialOutboundApi({
      sell_price: outForm.sell_price || undefined,
      sns,
    })
    outResult.value = res.data
    successText.value = `出库成功：单号 ${res.data.biz_no}，共 ${res.data.count} 个 SN`
    outForm.snText = ''
  } catch (e) {
    errorText.value = fmtError(e, '序列号出库失败')
  } finally {
    saving.value = false
  }
}

// ── Query ─────────────────────────────────────────────────────────────────────
async function fetchHistory(resetPage = false) {
  if (resetPage) qFilter.page = 1
  loading.value = true; errorText.value = ''
  try {
    const res = await getSerialHistoryApi({
      page: qFilter.page,
      page_size: pageSize,
      sn: qFilter.sn.trim() || undefined,
      product_id: qFilter.product_id ? Number(qFilter.product_id) : undefined,
      status: qFilter.status || undefined,
      start_date: qFilter.start_date || undefined,
      end_date: qFilter.end_date || undefined,
    })
    historyList.value = res.data.list
    historyTotal.value = res.data.total
  } catch (e) {
    errorText.value = fmtError(e, '查询失败')
  } finally {
    loading.value = false
  }
}

function prevPage() { if (qFilter.page > 1) { qFilter.page--; void fetchHistory() } }
function nextPage() {
  if (qFilter.page < Math.ceil(historyTotal.value / pageSize)) { qFilter.page++; void fetchHistory() }
}

onMounted(async () => {
  await fetchProducts()
  await fetchHistory()
})
</script>

<template>
  <section>
    <h2>序列号管理</h2>

    <!-- Tab 切换 -->
    <div class="scan-mode-tabs" style="margin-bottom:16px">
      <button class="scan-mode-tab" :class="{ 'is-active': tab === 'inbound' }" @click="tab = 'inbound'">序列号入库</button>
      <button class="scan-mode-tab" :class="{ 'is-active': tab === 'outbound' }" @click="tab = 'outbound'">序列号出库</button>
      <button class="scan-mode-tab" :class="{ 'is-active': tab === 'query' }" @click="tab = 'query'; fetchHistory(true)">查询 / 历史</button>
    </div>

    <p v-if="errorText" class="error-text">{{ errorText }}</p>
    <p v-if="successText" class="success-text">{{ successText }}</p>

    <!-- ── 入库 ── -->
    <template v-if="tab === 'inbound'">
      <div class="card-panel">
        <h3>序列号入库</h3>
        <p style="font-size:13px;color:#9aa5ba;margin:6px 0 12px">每行一个 SN 码，支持批量录入</p>
        <div class="form-inline form-inline-compact">
          <label class="form-label inline">
            <span>商品 *</span>
            <select v-model="inForm.product_id" :disabled="saving" style="width:200px">
              <option value="">请选择商品</option>
              <option v-for="p in products" :key="p.id" :value="String(p.id)">{{ p.name }}（{{ p.sku }}）</option>
            </select>
          </label>
          <label class="form-label inline">
            <span>单价（选填）</span>
            <input v-model="inForm.unit_cost" :disabled="saving" placeholder="如：299.00" style="width:120px" />
          </label>
          <label class="form-label inline">
            <span>批次ID（选填）</span>
            <input v-model="inForm.batch_id" :disabled="saving" placeholder="如：123" style="width:100px" />
          </label>
        </div>
        <div style="margin-top:12px">
          <label class="form-label">
            <span>SN 码列表 *（每行一个）</span>
            <textarea
              v-model="inForm.snText"
              :disabled="saving"
              rows="6"
              placeholder="SN001&#10;SN002&#10;SN003"
              style="width:100%;font-family:monospace;padding:8px;border:1px solid #e7ebf4;border-radius:8px;resize:vertical"
            />
          </label>
          <p style="font-size:12px;color:#9aa5ba;margin-top:4px">已输入 {{ parseSns(inForm.snText).length }} 个 SN 码</p>
        </div>
        <div class="form-actions" style="margin-top:12px">
          <button class="btn" :disabled="saving" @click="submitInbound">{{ saving ? '提交中...' : '确认入库' }}</button>
          <button class="btn btn-secondary" @click="inForm.snText = ''; inResult = null">清空</button>
        </div>
        <div v-if="inResult" class="result-panel" style="margin-top:12px">
          <p>✅ 入库单号：<strong>{{ inResult.biz_no }}</strong></p>
          <p>成功录入 <strong>{{ inResult.count }}</strong> 个序列号</p>
        </div>
      </div>
    </template>

    <!-- ── 出库 ── -->
    <template v-if="tab === 'outbound'">
      <div class="card-panel">
        <h3>序列号出库</h3>
        <p style="font-size:13px;color:#9aa5ba;margin:6px 0 12px">扫码或手动输入待出库的 SN 码</p>
        <label class="form-label inline" style="max-width:200px">
          <span>售价（选填）</span>
          <input v-model="outForm.sell_price" :disabled="saving" placeholder="如：399.00" />
        </label>
        <div style="margin-top:12px">
          <label class="form-label">
            <span>SN 码列表 *（每行一个）</span>
            <textarea
              v-model="outForm.snText"
              :disabled="saving"
              rows="6"
              placeholder="SN001&#10;SN002"
              style="width:100%;font-family:monospace;padding:8px;border:1px solid #e7ebf4;border-radius:8px;resize:vertical"
            />
          </label>
          <p style="font-size:12px;color:#9aa5ba;margin-top:4px">已输入 {{ parseSns(outForm.snText).length }} 个 SN 码</p>
        </div>
        <div class="form-actions" style="margin-top:12px">
          <button class="btn" :disabled="saving" @click="submitOutbound">{{ saving ? '提交中...' : '确认出库' }}</button>
          <button class="btn btn-secondary" @click="outForm.snText = ''; outResult = null">清空</button>
        </div>
        <div v-if="outResult" class="result-panel" style="margin-top:12px">
          <p>✅ 出库单号：<strong>{{ outResult.biz_no }}</strong></p>
          <p>成功出库 <strong>{{ outResult.count }}</strong> 个序列号</p>
          <div class="table-wrapper" v-if="outResult.list.length">
            <table class="data-table data-table-compact">
              <thead><tr><th>SN</th><th>状态</th><th>售价</th></tr></thead>
              <tbody>
                <tr v-for="sn in outResult.list" :key="sn.id">
                  <td><code>{{ sn.sn }}</code></td>
                  <td><span class="badge" :class="statusClass(sn.status)">{{ statusLabel(sn.status) }}</span></td>
                  <td>{{ sn.sell_price ? `¥${sn.sell_price}` : '—' }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </template>

    <!-- ── 查询 ── -->
    <template v-if="tab === 'query'">
      <div class="card-panel">
        <div class="form-inline form-inline-compact">
          <label class="form-label inline">
            <span>SN 码</span>
            <input v-model="qFilter.sn" placeholder="精确 SN 码" style="width:160px" />
          </label>
          <label class="form-label inline">
            <span>商品</span>
            <select v-model="qFilter.product_id" style="width:160px">
              <option value="">全部</option>
              <option v-for="p in products" :key="p.id" :value="String(p.id)">{{ p.name }}</option>
            </select>
          </label>
          <label class="form-label inline">
            <span>状态</span>
            <select v-model="qFilter.status" style="width:110px">
              <option value="">全部</option>
              <option value="IN_STOCK">在库</option>
              <option value="SOLD">已售</option>
              <option value="RETURNED">已退</option>
            </select>
          </label>
          <label class="form-label inline">
            <span>开始日期</span>
            <input v-model="qFilter.start_date" type="date" />
          </label>
          <label class="form-label inline">
            <span>结束日期</span>
            <input v-model="qFilter.end_date" type="date" />
          </label>
        </div>
        <div class="form-actions">
          <button class="btn" :disabled="loading" @click="fetchHistory(true)">{{ loading ? '查询中...' : '查询' }}</button>
        </div>
      </div>

      <p class="table-summary">共 {{ historyTotal }} 条（第 {{ qFilter.page }} 页）</p>

      <div class="table-wrapper">
        <table class="data-table">
          <thead>
            <tr>
              <th>SN 码</th>
              <th>商品 ID</th>
              <th>状态</th>
              <th>入库单号</th>
              <th>出库单号</th>
              <th>单价</th>
              <th>售价</th>
              <th>创建时间</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="sn in historyList" :key="sn.id">
              <td><code>{{ sn.sn }}</code></td>
              <td>{{ sn.product_id }}</td>
              <td><span class="badge" :class="statusClass(sn.status)">{{ statusLabel(sn.status) }}</span></td>
              <td><small>{{ sn.inbound_biz_no ?? '—' }}</small></td>
              <td><small>{{ sn.outbound_biz_no ?? '—' }}</small></td>
              <td>{{ sn.unit_cost ? `¥${sn.unit_cost}` : '—' }}</td>
              <td>{{ sn.sell_price ? `¥${sn.sell_price}` : '—' }}</td>
              <td><small>{{ sn.created_at.substring(0, 16).replace('T', ' ') }}</small></td>
            </tr>
            <tr v-if="!loading && historyList.length === 0">
              <td colspan="8" class="empty-cell">暂无序列号记录</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="pagination">
        <button class="btn btn-secondary" :disabled="qFilter.page <= 1 || loading" @click="prevPage">上一页</button>
        <span class="page-info">第 {{ qFilter.page }} 页 / 共 {{ Math.max(1, Math.ceil(historyTotal / pageSize)) }} 页</span>
        <button class="btn btn-secondary" :disabled="qFilter.page >= Math.ceil(historyTotal / pageSize) || loading" @click="nextPage">下一页</button>
      </div>
    </template>
  </section>
</template>
