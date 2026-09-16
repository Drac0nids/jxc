<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { inboundApi, inboundBatchApi } from '@/api/inventory'
import { scanProductApi } from '@/api/products'
import { listProductBatchesApi, createProductBatchApi } from '@/api/catalog'
import { ApiClientError } from '@/api/http'
import { useAuthStore } from '@/stores/auth'
import type {
  InboundBatchItemRequest,
  InboundBatchResponseData,
  InboundRequest,
  InboundResponseData,
  BatchData,
} from '@/types/api'
const authStore = useAuthStore()
const router = useRouter()
const route = useRoute()

const loading = ref(false)
const scanLoading = ref(false)
const errorText = ref('')
const successText = ref('')
const scanErrorText = ref('')
const scanSuccessText = ref('')
const result = ref<InboundResponseData | null>(null)
const batchResult = ref<InboundBatchResponseData | null>(null)

// Batch association modal
interface BatchAssocTarget { product_id: number }
const batchAssocQueue = ref<BatchAssocTarget[]>([])
const batchAssocCurrent = ref<BatchAssocTarget | null>(null)
const batchAssocExisting = ref<BatchData[]>([])
const batchAssocLoading = ref(false)
const batchAssocError = ref('')
const batchAssocNewForm = reactive({
  inbound_at: new Date().toISOString().substring(0, 10),
  lot_number: '',
  expires_at: '',
  notes: '',
})
const batchAssocTab = ref<'pick' | 'create'>('pick')

async function openBatchAssocForNext(): Promise<void> {
  const next = batchAssocQueue.value.shift()
  if (!next) { batchAssocCurrent.value = null; return }
  batchAssocCurrent.value = next
  batchAssocError.value = ''
  batchAssocTab.value = 'pick'
  batchAssocExisting.value = []
  batchAssocLoading.value = true
  try {
    const res = await listProductBatchesApi(next.product_id, { only_active: true })
    batchAssocExisting.value = res.data.batches ?? []
  } catch (e) {
    batchAssocError.value = e instanceof Error ? e.message : '获取批次失败'
  } finally {
    batchAssocLoading.value = false
  }
}

function batchAssocSkip() { void openBatchAssocForNext() }

async function batchAssocCreate() {
  const cur = batchAssocCurrent.value
  if (!cur) return
  if (!batchAssocNewForm.inbound_at) { batchAssocError.value = '入库日期必填'; return }
  batchAssocLoading.value = true; batchAssocError.value = ''
  try {
    await createProductBatchApi(cur.product_id, {
      inbound_at: batchAssocNewForm.inbound_at,
      lot_number: batchAssocNewForm.lot_number || undefined,
      expires_at: batchAssocNewForm.expires_at || undefined,
      notes: batchAssocNewForm.notes || undefined,
    })
    void openBatchAssocForNext()
  } catch (e) {
    batchAssocError.value = e instanceof Error ? e.message : '创建批次失败'
  } finally {
    batchAssocLoading.value = false
  }
}

function triggerBatchAssoc(items: { product_id: number; track_batches?: boolean }[]): void {
  const seen = new Set<number>()
  const targets: BatchAssocTarget[] = []
  for (const it of items) {
    if (it.track_batches && !seen.has(it.product_id)) {
      seen.add(it.product_id)
      targets.push({ product_id: it.product_id })
    }
  }
  if (!targets.length) return
  batchAssocQueue.value = targets
  batchAssocNewForm.inbound_at = new Date().toISOString().substring(0, 10)
  batchAssocNewForm.lot_number = ''
  batchAssocNewForm.expires_at = ''
  batchAssocNewForm.notes = ''
  void openBatchAssocForNext()
}

interface InboundDraftItem {
  local_id: number
  product_id: string
  barcode: string
  product_name: string
  qty: string
  unit_cost: string
  expected_version: string
  remark: string
}

let inboundDraftSeed = 1
const draftItems = ref<InboundDraftItem[]>([])

const form = reactive({
  product_id: '',
  barcode: '',
  qty: '1',
  unit_cost: '',
  expected_version: '',
  remark: '',
})

const scanForm = reactive({
  barcode: '',
})

const manualFormExpanded = ref(false)
const continuousSessionActive = ref(false)
const continuousProcessedCount = ref(0)
const continuousLastBarcode = ref('')
const continuousLastAt = ref(0)

const canSubmit = computed(() => {
  const role = authStore.session?.user.role
  return role === 'OWNER' || role === 'PURCHASER'
})

function firstQueryValue(value: unknown): string {
  if (typeof value === 'string') {
    return value
  }

  if (Array.isArray(value) && typeof value[0] === 'string') {
    return value[0]
  }

  return ''
}

function formatApiError(error: unknown, fallback: string): string {
  if (error instanceof ApiClientError) {
    return `${error.message}（code=${error.code}${error.requestId ? `, request_id=${error.requestId}` : ''}）`
  }

  return error instanceof Error ? error.message : fallback
}

function resetScanForm(): void {
  scanForm.barcode = ''
  continuousSessionActive.value = false
  continuousProcessedCount.value = 0
  continuousLastBarcode.value = ''
  continuousLastAt.value = 0
  scanErrorText.value = ''
  scanSuccessText.value = ''
}

function isDraftBlank(item: InboundDraftItem): boolean {
  return (
    !item.product_id.trim() &&
    !item.barcode.trim() &&
    !item.product_name.trim() &&
    !item.qty.trim() &&
    !item.unit_cost.trim() &&
    !item.expected_version.trim() &&
    !item.remark.trim()
  )
}

function getEffectiveDraftItems(): InboundDraftItem[] {
  return draftItems.value.filter((item) => !isDraftBlank(item))
}

function upsertDraftItem(payload: {
  product_id: string
  barcode: string
  product_name: string
  qty: number
  unit_cost?: string
  expected_version?: string
  remark?: string
}): void {
  const existed = draftItems.value.find(
    (item) => item.product_id.trim() === payload.product_id || item.barcode.trim() === payload.barcode,
  )

  if (existed) {
    const currentQty = Number(existed.qty.trim())
    const safeQty = Number.isInteger(currentQty) && currentQty > 0 ? currentQty : 0
    existed.qty = String(safeQty + payload.qty)
    if (!existed.product_name.trim()) {
      existed.product_name = payload.product_name
    }
    if (!existed.unit_cost.trim() && payload.unit_cost?.trim()) {
      existed.unit_cost = payload.unit_cost.trim()
    }
    if (!existed.expected_version.trim() && payload.expected_version?.trim()) {
      existed.expected_version = payload.expected_version.trim()
    }
    if (!existed.remark.trim() && payload.remark?.trim()) {
      existed.remark = payload.remark.trim()
    }
    return
  }

  draftItems.value.push({
    local_id: inboundDraftSeed++,
    product_id: payload.product_id,
    barcode: payload.barcode,
    product_name: payload.product_name,
    qty: String(payload.qty),
    unit_cost: payload.unit_cost?.trim() ?? '',
    expected_version: payload.expected_version?.trim() ?? '',
    remark: payload.remark?.trim() ?? '',
  })
}

function playContinuousSuccessVoice(): void {
  if (typeof window === 'undefined' || !('speechSynthesis' in window)) {
    return
  }

  const utterance = new SpeechSynthesisUtterance('扫码成功')
  utterance.lang = 'zh-CN'
  utterance.rate = 1
  window.speechSynthesis.cancel()
  window.speechSynthesis.speak(utterance)
}

function isContinuousDuplicate(barcode: string): boolean {
  const now = Date.now()
  const duplicated =
    continuousLastBarcode.value === barcode && now - continuousLastAt.value <= 1200

  continuousLastBarcode.value = barcode
  continuousLastAt.value = now
  return duplicated
}

function applyScannedInbound(payload: {
  product_id: string
  product_name: string
  barcode: string
  qty: number
  unit_cost?: string
}): boolean {
  upsertDraftItem(payload)

  const current = draftItems.value.find(
    (item) => item.product_id === payload.product_id || item.barcode === payload.barcode,
  )
  const qtyText = current?.qty ?? String(payload.qty)
  scanSuccessText.value = `扫码成功：#${payload.product_id} ${payload.product_name}，已加入入库明细，当前数量=${qtyText}`
  return true
}

async function scanAndAccumulate(options?: { forceQuickAccumulate?: boolean; fromContinuousSession?: boolean }): Promise<void> {
  if (scanLoading.value || loading.value) {
    return
  }

  scanErrorText.value = ''
  scanSuccessText.value = ''

  if (!canSubmit.value) {
    scanErrorText.value = '当前角色无采购入库权限，仅 OWNER/PURCHASER 可操作'
    return
  }

  const barcode = scanForm.barcode.trim()
  if (!barcode) {
    scanErrorText.value = options?.fromContinuousSession ? '连续扫码会话中请先输入条码' : '请输入条码后再扫码入库'
    return
  }

  if (options?.fromContinuousSession && isContinuousDuplicate(barcode)) {
    scanErrorText.value = `已忽略短时间重复条码：${barcode}`
    return
  }

  scanLoading.value = true
  try {
    const response = await scanProductApi(barcode)
    const defaultUnitCost =
      response.data.last_inbound_unit_cost?.trim() || response.data.cost_price?.trim() || ''
    const payload = {
      product_id: String(response.data.id),
      product_name: response.data.name,
      barcode,
      qty: 1,
      unit_cost: defaultUnitCost,
    }

    const ok = applyScannedInbound(payload)
    if (!ok) {
      return
    }

    if (options?.fromContinuousSession) {
      continuousProcessedCount.value += 1
      playContinuousSuccessVoice()
      scanForm.barcode = ''
    }
  } catch (error) {
    if (error instanceof ApiClientError && error.code === 4040) {
      scanErrorText.value = '未找到该条码对应商品。'

      if (canSubmit.value) {
        const confirmed = window.confirm(`条码 ${barcode} 未建档，是否立即前往“商品管理”新建商品？`)
        if (confirmed) {
          await router.push({
            name: 'products',
            query: {
              from: 'inbound',
              barcode,
            },
          })
        }
      }
      return
    }

    scanErrorText.value = formatApiError(error, '扫码入库失败')
  } finally {
    scanLoading.value = false
  }
}

function startContinuousScanSession(): void {
  scanErrorText.value = ''
  scanSuccessText.value = '连续扫码会话已开启，请持续扫码（回车）'
  continuousSessionActive.value = true
  continuousProcessedCount.value = 0
  continuousLastBarcode.value = ''
  continuousLastAt.value = 0
}

function stopContinuousScanSession(): void {
  continuousSessionActive.value = false
  scanSuccessText.value = `连续扫码会话已结束，本次共处理 ${continuousProcessedCount.value} 条`
}

async function scanByCurrentMode(): Promise<void> {
  if (!continuousSessionActive.value) {
    startContinuousScanSession()
    return
  }

  await scanAndAccumulate({
    forceQuickAccumulate: true,
    fromContinuousSession: true,
  })
}

function resetForm(): void {
  form.product_id = ''
  form.barcode = ''
  form.qty = '1'
  form.unit_cost = ''
  form.expected_version = ''
  form.remark = ''

  errorText.value = ''
  successText.value = ''
  result.value = null
  batchResult.value = null
  draftItems.value = []
  resetScanForm()
}

function addCurrentFormToDraft(): void {
  scanErrorText.value = ''
  const productId = form.product_id.trim()
  const barcode = form.barcode.trim()
  const qty = Number(form.qty.trim())

  if (!productId && !barcode) {
    errorText.value = '加入明细前请先填写商品ID或条码'
    return
  }
  if (productId) {
    const productIdNum = Number(productId)
    if (!Number.isInteger(productIdNum) || productIdNum <= 0) {
      errorText.value = '商品ID必须为正整数'
      return
    }
  }
  if (!Number.isInteger(qty) || qty <= 0) {
    errorText.value = '数量必须为正整数'
    return
  }

  upsertDraftItem({
    product_id: productId,
    barcode,
    product_name: form.remark.trim() || `商品#${productId || barcode}`,
    qty,
    unit_cost: form.unit_cost,
    expected_version: form.expected_version,
    remark: form.remark,
  })

  successText.value = `已加入入库明细（共 ${getEffectiveDraftItems().length} 条）`
  form.product_id = ''
  form.barcode = ''
  form.qty = '1'
  form.unit_cost = ''
  form.expected_version = ''
  form.remark = ''
}

function removeDraftItem(localId: number): void {
  draftItems.value = draftItems.value.filter((item) => item.local_id !== localId)
}

function updateDraftItemUnitCost(localId: number, rawValue: string): void {
  const value = rawValue.trim()
  if (value && !/^\d+(\.\d{1,4})?$/.test(value)) {
    errorText.value = '进货价格式错误（示例：2.20）'
    return
  }

  const target = draftItems.value.find((item) => item.local_id === localId)
  if (!target) {
    return
  }

  target.unit_cost = value
}

async function submit(): Promise<void> {
  if (loading.value) {
    return
  }

  errorText.value = ''
  successText.value = ''

  const effectiveItems = getEffectiveDraftItems()
  if (effectiveItems.length === 0) {
    errorText.value = '请先加入至少 1 条待提交明细，再提交入库。'
    return
  }

  loading.value = true
  try {
    if (effectiveItems.length >= 2) {
      const payloadItems: InboundBatchItemRequest[] = effectiveItems.map((item) => {
        const next: InboundBatchItemRequest = {
          qty: Number(item.qty.trim()),
        }

        if (item.product_id.trim()) {
          next.product_id = Number(item.product_id.trim())
        }
        if (item.barcode.trim()) {
          next.barcode = item.barcode.trim()
        }
        if (item.unit_cost.trim()) {
          next.unit_cost = item.unit_cost.trim()
        }
        if (item.expected_version.trim()) {
          next.expected_version = Number(item.expected_version.trim())
        }
        if (item.remark.trim()) {
          next.remark = item.remark.trim()
        }

        return next
      })

      const response = await inboundBatchApi({ items: payloadItems })
      batchResult.value = response.data
      result.value = null
      successText.value = `批量入库成功：${response.data.biz_no}，共 ${response.data.items.length} 条`
      triggerBatchAssoc(response.data.items)
    } else if (effectiveItems.length === 1) {
      const item = effectiveItems[0]!
      const payload: InboundRequest = {
        qty: Number(item.qty.trim()),
      }
      if (item.product_id.trim()) payload.product_id = Number(item.product_id.trim())
      if (item.barcode.trim()) payload.barcode = item.barcode.trim()
      if (item.unit_cost.trim()) payload.unit_cost = item.unit_cost.trim()
      if (item.expected_version.trim()) payload.expected_version = Number(item.expected_version.trim())
      if (item.remark.trim()) payload.remark = item.remark.trim()

      const response = await inboundApi(payload)
      result.value = response.data
      batchResult.value = null
      successText.value = `入库成功：${response.data.biz_no}，当前库存 ${response.data.current_stock}`
      if (response.data.track_batches) {
        triggerBatchAssoc([{ product_id: response.data.product_id, track_batches: true }])
      }
    }

    form.product_id = ''
    form.barcode = ''
    form.qty = '1'
    form.unit_cost = ''
    form.expected_version = ''
    form.remark = ''
    draftItems.value = []
    resetScanForm()
  } catch (error) {
    errorText.value = formatApiError(error, '入库失败')
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  const createdProductId = firstQueryValue(route.query.created_product_id).trim()
  const createdBarcode = firstQueryValue(route.query.created_barcode).trim()

  if (!createdProductId && !createdBarcode) {
    return
  }

  if (createdProductId) {
    form.product_id = createdProductId
  }
  if (createdBarcode) {
    form.barcode = createdBarcode
    scanForm.barcode = createdBarcode
  }

  scanSuccessText.value = `新建商品已回填：商品ID=${createdProductId || '-'}，条码=${createdBarcode || '-'}，请继续确认数量后提交入库。`

  void router.replace({
    name: 'inbound',
  })
})
</script>

<template>
  <section>
    <h2>采购入库</h2>

    <p v-if="!canSubmit" class="warn-text">当前角色无采购入库权限，仅 OWNER/PURCHASER 可操作。</p>

    <div class="card-panel form-grid" style="margin-bottom: 12px">
      <h3>条码入库（自动模式）</h3>

      <form class="form-inline" @submit.prevent="scanByCurrentMode">
        <label class="form-label inline">
          <span>条码</span>
          <input
            v-model="scanForm.barcode"
            :disabled="scanLoading || loading"
            placeholder="扫码枪回车，例如：690123456789"
          />
        </label>

        <button class="btn" type="submit" :disabled="scanLoading || loading || !canSubmit">
          <span class="btn-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24"><path d="M4 7h16v10H4z" /><path d="M8 7V5h8v2M12 12v5" /></svg>
          </span>
          {{
            continuousSessionActive
              ? (scanLoading ? '识别中...' : '处理当前条码并累加')
              : '开始扫码会话'
          }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="scanLoading || loading" @click="resetScanForm">
          <span class="btn-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24"><path d="M4 12a8 8 0 1 0 2-5.3" /><path d="M4 4v4h4" /></svg>
          </span>
          清空
        </button>
        <button
          v-if="continuousSessionActive"
          class="btn btn-secondary"
          type="button"
          :disabled="scanLoading || loading"
          @click="stopContinuousScanSession"
        >
          结束扫码
        </button>
      </form>

      <p class="table-summary">命中商品后自动回填并累加到明细，重复扫码同商品会继续叠加数量。</p>
      <p class="table-summary">扫码会话：{{ continuousSessionActive ? '进行中' : '未开始' }}，已处理 {{ continuousProcessedCount }} 条。</p>

      <p v-if="scanErrorText" class="error-text">{{ scanErrorText }}</p>
      <p v-if="scanSuccessText" class="success-text">{{ scanSuccessText }}</p>
    </div>

    <div class="form-grid">
      <div class="card-panel form-grid">
        <div class="card-panel-header">
          <h3>高级录入区（手工）</h3>
          <button class="btn btn-secondary" type="button" :disabled="loading" @click="manualFormExpanded = !manualFormExpanded">
            {{ manualFormExpanded ? '收起' : '展开' }}
          </button>
        </div>

        <p v-if="!manualFormExpanded" class="table-summary">默认折叠。需要手工补录时可展开并加入明细。</p>

        <template v-else>
          <div class="form-inline form-inline-compact">
            <label class="form-label inline">
              <span>商品ID</span>
              <input v-model="form.product_id" :disabled="loading" placeholder="可选，正整数" />
            </label>

            <label class="form-label inline">
              <span>条码</span>
              <input v-model="form.barcode" :disabled="loading" placeholder="可选，product_id 与 barcode 二选一即可" />
            </label>
          </div>

          <div class="form-inline form-inline-compact">
            <label class="form-label inline">
              <span>数量 *</span>
              <input v-model="form.qty" :disabled="loading" placeholder="正整数" />
            </label>

            <label class="form-label inline">
              <span>进货价格</span>
              <input v-model="form.unit_cost" :disabled="loading" placeholder="可选，例如：2.20" />
            </label>

            <label class="form-label inline">
              <span>版本</span>
              <input v-model="form.expected_version" :disabled="loading" placeholder="可选 expected_version" />
            </label>
          </div>

          <label class="form-label">
            <span>备注</span>
            <input v-model="form.remark" :disabled="loading" placeholder="可选备注" />
          </label>

          <div class="form-actions">
            <button class="btn btn-secondary" type="button" :disabled="loading || !canSubmit" @click="addCurrentFormToDraft">
              加入明细
            </button>
            <button class="btn btn-secondary" type="button" :disabled="loading" @click="resetForm">
              重置
            </button>
          </div>
        </template>
      </div>

      <p v-if="errorText" class="error-text">{{ errorText }}</p>
      <p v-if="successText" class="success-text">{{ successText }}</p>
    </div>

    <div class="card-panel" style="margin-top: 12px">
      <div class="card-panel-header">
        <h3>待提交入库明细（{{ getEffectiveDraftItems().length }}）</h3>
      </div>
      <div class="table-wrapper">
        <table class="data-table data-table-compact">
          <thead>
            <tr>
              <th>#</th>
              <th>商品ID</th>
              <th>商品名称</th>
              <th>条码</th>
              <th>数量</th>
              <th>进货价格</th>
              <th>版本</th>
              <th>备注</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(item, index) in getEffectiveDraftItems()" :key="item.local_id">
              <td>{{ index + 1 }}</td>
              <td>{{ item.product_id || '-' }}</td>
              <td>{{ item.product_name || '-' }}</td>
              <td>{{ item.barcode || '-' }}</td>
              <td>{{ item.qty }}</td>
              <td>
                <input
                  :value="item.unit_cost"
                  :disabled="loading"
                  placeholder="可选，例如：2.20"
                  @change="updateDraftItemUnitCost(item.local_id, ($event.target as HTMLInputElement).value)"
                />
              </td>
              <td>{{ item.expected_version || '-' }}</td>
              <td>{{ item.remark || '-' }}</td>
              <td>
                <button class="btn btn-danger" type="button" :disabled="loading" @click="removeDraftItem(item.local_id)">
                  删除
                </button>
              </td>
            </tr>
            <tr v-if="getEffectiveDraftItems().length === 0">
              <td colspan="9" class="empty-cell">暂无待提交明细，可扫码或填写表单后点击“加入明细”。</td>
            </tr>
          </tbody>
        </table>
      </div>
      <p class="table-summary">提交规则：1 条明细走单条接口；2 条及以上默认走批量入库接口。</p>
      <div class="form-actions" style="margin-top: 8px">
        <button class="btn btn-success" type="button" :disabled="loading || !canSubmit || getEffectiveDraftItems().length === 0" @click="submit">
          <span class="btn-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24"><path d="M12 5v14M5 12h14" /></svg>
          </span>
          {{ loading ? '提交中...' : '提交入库' }}
        </button>
        <p class="table-summary" style="margin: 0">
          {{ getEffectiveDraftItems().length === 0 ? '请先加入明细，再提交入库。' : '请核对明细后提交入库。' }}
        </p>
      </div>
    </div>

    <div v-if="result" class="result-panel">
      <h3>最近一次入库结果</h3>
      <p>业务单号：{{ result.biz_no }}</p>
      <p>商品ID：{{ result.product_id }}</p>
      <p>当前库存：{{ result.current_stock }}</p>
      <p>当前进货价格：{{ result.cost_price }}</p>
      <p>版本：{{ result.version }}</p>
    </div>

    <div v-if="batchResult" class="result-panel">
      <h3>最近一次批量入库结果</h3>
      <p>业务单号：{{ batchResult.biz_no }}</p>
      <div class="table-wrapper">
        <table class="data-table data-table-compact">
          <thead>
            <tr>
              <th>商品ID</th>
              <th>当前库存</th>
              <th>进货价格</th>
              <th>版本</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in batchResult.items" :key="`${item.product_id}-${item.version}`">
              <td>{{ item.product_id }}</td>
              <td>{{ item.current_stock }}</td>
              <td>{{ item.cost_price }}</td>
              <td>{{ item.version }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </section>

  <Teleport to="body">
    <div v-if="batchAssocCurrent" class="modal-backdrop">
      <div class="modal-card" style="max-width:480px;width:96vw">
        <h3 style="margin:0 0 6px">批次关联</h3>
        <p style="font-size:13px;color:#9aa5ba;margin:0 0 14px">
          商品 #{{ batchAssocCurrent.product_id }} 已启用批次追踪，请为本次入库选择或新建批次。
          <span v-if="batchAssocQueue.length">（还有 {{ batchAssocQueue.length }} 个商品待处理）</span>
        </p>
        <div class="scan-mode-tabs" style="margin-bottom:14px">
          <button class="scan-mode-tab" :class="{ 'is-active': batchAssocTab === 'pick' }" @click="batchAssocTab = 'pick'">选择已有批次</button>
          <button class="scan-mode-tab" :class="{ 'is-active': batchAssocTab === 'create' }" @click="batchAssocTab = 'create'">新建批次</button>
        </div>
        <p v-if="batchAssocError" class="error-text">{{ batchAssocError }}</p>
        <template v-if="batchAssocTab === 'pick'">
          <p v-if="batchAssocLoading" class="table-summary">加载批次中...</p>
          <p v-else-if="!batchAssocExisting.length" class="table-summary">暂无有效批次，请切换到「新建批次」</p>
          <div v-else class="table-wrapper" style="max-height:200px;overflow-y:auto">
            <table class="data-table data-table-compact">
              <thead><tr><th>批次号</th><th>入库日期</th><th>过期日期</th><th>状态</th></tr></thead>
              <tbody>
                <tr v-for="b in batchAssocExisting" :key="b.id" style="cursor:pointer" @click="batchAssocSkip()">
                  <td>{{ b.lot_number || '—' }}</td>
                  <td>{{ b.inbound_at }}</td>
                  <td>{{ b.expires_at || '—' }}</td>
                  <td><span class="badge" :class="b.expiry_level === 'OK' ? 'badge-ok' : 'badge-warning'">{{ b.expiry_level || '—' }}</span></td>
                </tr>
              </tbody>
            </table>
          </div>
          <p style="font-size:12px;color:#9aa5ba;margin-top:8px">点击任意行即视为选中该批次并继续</p>
        </template>
        <template v-if="batchAssocTab === 'create'">
          <div class="form-inline form-inline-compact">
            <label class="form-label inline">
              <span>入库日期 *</span>
              <input v-model="batchAssocNewForm.inbound_at" type="date" :disabled="batchAssocLoading" />
            </label>
            <label class="form-label inline">
              <span>批次号</span>
              <input v-model="batchAssocNewForm.lot_number" :disabled="batchAssocLoading" placeholder="可选" />
            </label>
          </div>
          <div class="form-inline form-inline-compact">
            <label class="form-label inline">
              <span>过期日期</span>
              <input v-model="batchAssocNewForm.expires_at" type="date" :disabled="batchAssocLoading" />
            </label>
            <label class="form-label inline">
              <span>备注</span>
              <input v-model="batchAssocNewForm.notes" :disabled="batchAssocLoading" placeholder="可选" />
            </label>
          </div>
        </template>
        <div class="form-actions" style="margin-top:14px">
          <button v-if="batchAssocTab === 'create'" class="btn" :disabled="batchAssocLoading" @click="batchAssocCreate">
            {{ batchAssocLoading ? '创建中...' : '创建并继续' }}
          </button>
          <button class="btn btn-secondary" :disabled="batchAssocLoading" @click="batchAssocSkip">跳过</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
