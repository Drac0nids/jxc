<script setup lang="ts">
import { computed, nextTick, onMounted, reactive, ref, watch } from 'vue'

import { outboundApi } from '@/api/inventory'
import { scanProductApi } from '@/api/products'
import { ApiClientError } from '@/api/http'
import { useAuthStore } from '@/stores/auth'
import type {
  OutboundItemRequest,
  OutboundRequest,
  OutboundResponseData,
  StockInsufficientErrorData,
} from '@/types/api'
import {
  readStoredScanMode,
  resolveScanPreferenceScope,
  writeStoredScanMode,
} from '@/utils/sessionStorage'

interface OutboundFormItem {
  local_id: number
  product_id: string
  product_name: string
  qty: string
  sell_price: string
  expected_version: string
  editable: boolean
}

type OutboundScanMode = 'scan_confirm' | 'continuous_scan'
const OUTBOUND_SCAN_MODES: OutboundScanMode[] = ['scan_confirm', 'continuous_scan']

const loading = ref(false)
const scanLoading = ref(false)
const errorText = ref('')
const successText = ref('')
const scanErrorText = ref('')
const scanSuccessText = ref('')
const result = ref<OutboundResponseData | null>(null)

const authStore = useAuthStore()
const scanBarcodeInputRef = ref<HTMLInputElement | null>(null)
const scanConfirmQtyInputRef = ref<HTMLInputElement | null>(null)

let itemSeed = 1

function createFormItem(): OutboundFormItem {
  return {
    local_id: itemSeed++,
    product_id: '',
    product_name: '',
    qty: '1',
    sell_price: '',
    expected_version: '',
    editable: false,
  }
}

const form = reactive({
  customer_id: '',
  expected_version: '',
  remark: '',
  items: [] as OutboundFormItem[],
})

const scanForm = reactive({
  barcode: '',
  sell_price: '',
})

const scanMode = ref<OutboundScanMode>('scan_confirm')
const orderHeaderExpanded = ref(false)
const orderAdvancedExpanded = ref(false)
const scanConfirmSessionActive = ref(false)
const scanConfirmProcessedCount = ref(0)
const continuousSessionActive = ref(false)
const continuousProcessedCount = ref(0)
const continuousLastBarcode = ref('')
const continuousLastAt = ref(0)

const scanConfirmForm = reactive({
  visible: false,
  product_id: '',
  product_name: '',
  qty: '1',
  sell_price: '',
  expected_version: '',
})

const canSubmit = computed(() => {
  const role = authStore.session?.user.role
  return role === 'OWNER' || role === 'SALES'
})

function parseStockInsufficientData(data: unknown): StockInsufficientErrorData | null {
  if (!data || typeof data !== 'object') {
    return null
  }

  const record = data as Record<string, unknown>
  const parseOptionalNumber = (value: unknown): number | undefined =>
    typeof value === 'number' && Number.isFinite(value) ? value : undefined

  const parsed: StockInsufficientErrorData = {
    failed_product_id: parseOptionalNumber(record.failed_product_id),
    available_stock: parseOptionalNumber(record.available_stock),
    required_qty: parseOptionalNumber(record.required_qty),
  }

  if (
    typeof parsed.failed_product_id !== 'number' ||
    typeof parsed.available_stock !== 'number' ||
    typeof parsed.required_qty !== 'number'
  ) {
    return null
  }

  return parsed
}

function formatApiError(error: unknown, fallback: string): string {
  if (error instanceof ApiClientError) {
    if (error.code === 4001) {
      const detail = parseStockInsufficientData(error.data)
      if (detail) {
        return `库存不足：商品ID=${detail.failed_product_id}，可用库存=${detail.available_stock}，需求数量=${detail.required_qty}（code=${error.code}${error.requestId ? `, request_id=${error.requestId}` : ''}）`
      }
    }

    return `${error.message}（code=${error.code}${error.requestId ? `, request_id=${error.requestId}` : ''}）`
  }

  return error instanceof Error ? error.message : fallback
}

function resolveOutboundStoredMode(storedMode: string | null): OutboundScanMode | null {
  if (!storedMode) {
    return null
  }

  if (storedMode === 'quick_accumulate') {
    return 'continuous_scan'
  }

  if (OUTBOUND_SCAN_MODES.includes(storedMode as OutboundScanMode)) {
    return storedMode as OutboundScanMode
  }

  return null
}

function resetScanForm(): void {
  scanForm.barcode = ''
  scanForm.sell_price = ''
  scanConfirmSessionActive.value = false
  scanConfirmProcessedCount.value = 0
  continuousSessionActive.value = false
  continuousProcessedCount.value = 0
  continuousLastBarcode.value = ''
  continuousLastAt.value = 0
  scanConfirmForm.visible = false
  scanConfirmForm.product_id = ''
  scanConfirmForm.product_name = ''
  scanConfirmForm.qty = '1'
  scanConfirmForm.sell_price = ''
  scanConfirmForm.expected_version = ''
  scanErrorText.value = ''
  scanSuccessText.value = ''
}

function clearScanConfirmDraft(): void {
  scanConfirmForm.visible = false
  scanConfirmForm.product_id = ''
  scanConfirmForm.product_name = ''
  scanConfirmForm.qty = '1'
  scanConfirmForm.sell_price = ''
  scanConfirmForm.expected_version = ''
}

function openScanConfirmDialog(payload: {
  product_id: string
  product_name: string
  sell_price: string
}): void {
  scanConfirmSessionActive.value = true
  scanConfirmForm.visible = true
  scanConfirmForm.product_id = payload.product_id
  scanConfirmForm.product_name = payload.product_name
  scanConfirmForm.qty = '1'
  scanConfirmForm.sell_price = payload.sell_price
  scanConfirmForm.expected_version = ''
  scanSuccessText.value = `扫码成功：#${payload.product_id} ${payload.product_name}，请确认后写入明细`

  nextTick(() => {
    const input = scanConfirmQtyInputRef.value
    input?.focus()
    input?.select()
  })
}

function cancelScanConfirmDialog(): void {
  clearScanConfirmDraft()
  scanForm.barcode = ''

  if (scanMode.value === 'scan_confirm') {
    scanConfirmSessionActive.value = true
    scanSuccessText.value = '已取消本次确认，可继续扫码'
    focusScanBarcodeInput()
  }
}

function focusScanBarcodeInput(): void {
  nextTick(() => {
    scanBarcodeInputRef.value?.focus()
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

function isBlankItem(item: OutboundFormItem): boolean {
  return (
    !item.product_id.trim() &&
    !item.product_name.trim() &&
    !item.qty.trim() &&
    !item.sell_price.trim() &&
    !item.expected_version.trim()
  )
}

function getEffectiveItems(items: OutboundFormItem[]): OutboundFormItem[] {
  return items.filter((item) => !isBlankItem(item))
}

function mergeScannedItem(payload: {
  product_id: string
  product_name: string
  qty: number
  sell_price: string
  expected_version?: string
}): void {
  const nextItems = getEffectiveItems(form.items)
  if (nextItems.length !== form.items.length) {
    form.items = nextItems
  }

  const existed = form.items.find((item) => item.product_id.trim() === payload.product_id)
  if (existed) {
    const currentQty = Number(existed.qty.trim())
    const safeQty = Number.isInteger(currentQty) && currentQty > 0 ? currentQty : 0
    const nextQty = safeQty + payload.qty
    existed.qty = String(nextQty)

    if (!existed.sell_price.trim()) {
      existed.sell_price = payload.sell_price
    }
    if (!existed.expected_version.trim() && payload.expected_version?.trim()) {
      existed.expected_version = payload.expected_version.trim()
    }
    if (!existed.product_name.trim() && payload.product_name.trim()) {
      existed.product_name = payload.product_name.trim()
    }

    scanSuccessText.value = `扫码成功：#${payload.product_id} ${payload.product_name}，已累加到第 1 个匹配明细，数量=${nextQty}`
    return
  }

  const nextItem = createFormItem()
  nextItem.product_id = payload.product_id
  nextItem.product_name = payload.product_name
  nextItem.qty = String(payload.qty)
  nextItem.sell_price = payload.sell_price
  nextItem.expected_version = payload.expected_version?.trim() ?? ''
  form.items.push(nextItem)
  scanSuccessText.value = `扫码成功：#${payload.product_id} ${payload.product_name}，已新增出库明细（数量=${payload.qty}）`
}

function applyScanConfirm(): void {
  const qty = Number(scanConfirmForm.qty.trim())
  if (!Number.isInteger(qty) || qty <= 0) {
    scanErrorText.value = '确认区“数量”必须为正整数'
    return
  }

  const sellPrice = scanConfirmForm.sell_price.trim()
  if (!/^\d+(\.\d{1,4})?$/.test(sellPrice)) {
    scanErrorText.value = '确认区“销售单价”格式错误（示例：3.50）'
    return
  }

  if (scanConfirmForm.expected_version.trim()) {
    const expectedVersion = Number(scanConfirmForm.expected_version.trim())
    if (!Number.isInteger(expectedVersion) || expectedVersion < 0) {
      scanErrorText.value = '确认区“版本”必须为大于等于 0 的整数'
      return
    }
  }

  mergeScannedItem({
    product_id: scanConfirmForm.product_id,
    product_name: scanConfirmForm.product_name,
    qty,
    sell_price: sellPrice,
    expected_version: scanConfirmForm.expected_version,
  })
  scanConfirmForm.visible = false
  clearScanConfirmDraft()
  scanForm.barcode = ''

  if (scanMode.value === 'scan_confirm') {
    scanConfirmSessionActive.value = true
    scanConfirmProcessedCount.value += 1
    scanSuccessText.value = `确认写入成功，确认续扫会话进行中（已处理 ${scanConfirmProcessedCount.value} 条）`
    focusScanBarcodeInput()
  }
}

async function scanAndAccumulate(options?: { forceQuickAccumulate?: boolean; fromContinuousSession?: boolean }): Promise<void> {
  if (loading.value || scanLoading.value) {
    return
  }

  scanErrorText.value = ''
  scanSuccessText.value = ''

  if (!canSubmit.value) {
    scanErrorText.value = '当前角色无销售出库权限，仅 OWNER/SALES 可操作'
    return
  }

  const barcode = scanForm.barcode.trim()
  if (!barcode) {
    scanErrorText.value = options?.fromContinuousSession ? '连续扫码会话中请先输入条码' : '请输入条码后再扫码出库'
    return
  }

  if (options?.fromContinuousSession && isContinuousDuplicate(barcode)) {
    scanErrorText.value = `已忽略短时间重复条码：${barcode}`
    return
  }

  const customSellPrice = scanForm.sell_price.trim()
  if (customSellPrice && !/^\d+(\.\d{1,4})?$/.test(customSellPrice)) {
    scanErrorText.value = '扫码区“销售单价”格式错误（示例：3.50）'
    return
  }

  scanLoading.value = true
  try {
    const response = await scanProductApi(barcode)
    const scannedProductId = String(response.data.id)
    const sellPrice = customSellPrice || response.data.retail_price

    if (scanMode.value === 'scan_confirm' && !options?.forceQuickAccumulate) {
      openScanConfirmDialog({
        product_id: scannedProductId,
        product_name: response.data.name,
        sell_price: sellPrice,
      })
      return
    }

    mergeScannedItem({
      product_id: scannedProductId,
      product_name: response.data.name,
      qty: 1,
      sell_price: sellPrice,
    })

    if (options?.fromContinuousSession) {
      continuousProcessedCount.value += 1
      playContinuousSuccessVoice()
      scanForm.barcode = ''
    }
  } catch (error) {
    if (error instanceof ApiClientError && error.code === 4040) {
      scanErrorText.value = '未找到该条码对应商品，请先前往“商品管理”建档。'
      return
    }

    scanErrorText.value = formatApiError(error, '扫码出库失败')
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

function stopScanConfirmSession(): void {
  scanConfirmSessionActive.value = false
  scanSuccessText.value = `确认续扫会话已结束，本次共处理 ${scanConfirmProcessedCount.value} 条`
}

function stopContinuousScanSession(): void {
  continuousSessionActive.value = false
  scanSuccessText.value = `连续扫码会话已结束，本次共处理 ${continuousProcessedCount.value} 条`
}

async function scanByCurrentMode(): Promise<void> {
  if (scanMode.value === 'continuous_scan') {
    if (!continuousSessionActive.value) {
      startContinuousScanSession()
      return
    }

    await scanAndAccumulate({
      forceQuickAccumulate: true,
      fromContinuousSession: true,
    })
    return
  }

  await scanAndAccumulate()
}

onMounted(() => {
  const scope = resolveScanPreferenceScope(authStore.session)
  if (!scope) {
    return
  }

  const storedMode = readStoredScanMode(scope, 'outbound')
  const resolvedMode = resolveOutboundStoredMode(storedMode)
  if (resolvedMode) {
    scanMode.value = resolvedMode

    if (storedMode !== resolvedMode) {
      writeStoredScanMode(scope, 'outbound', resolvedMode)
    }
  }
})

watch(scanMode, (value) => {
  if (value !== 'scan_confirm') {
    scanConfirmSessionActive.value = false
    scanConfirmProcessedCount.value = 0
  }

  if (value !== 'continuous_scan') {
    continuousSessionActive.value = false
  }

  const scope = resolveScanPreferenceScope(authStore.session)
  if (scope) {
    writeStoredScanMode(scope, 'outbound', value)
  }
})

function addItem(): void {
  form.items.push(createFormItem())
}

function toggleItemEditable(localId: number): void {
  const target = form.items.find((item) => item.local_id === localId)
  if (!target) {
    return
  }
  target.editable = !target.editable
}

function removeItem(localId: number): void {
  const index = form.items.findIndex((item) => item.local_id === localId)
  if (index >= 0) {
    form.items.splice(index, 1)
  }
}

function resetForm(): void {
  form.customer_id = ''
  form.expected_version = ''
  form.remark = ''
  form.items = []

  errorText.value = ''
  successText.value = ''
  result.value = null
  resetScanForm()
}

function validateForm(): string | null {
  if (!canSubmit.value) {
    return '当前角色无销售出库权限，仅 OWNER/SALES 可操作'
  }

  if (form.customer_id.trim()) {
    const customerId = Number(form.customer_id.trim())
    if (!Number.isInteger(customerId) || customerId <= 0) {
      return '客户ID必须为正整数'
    }
  }

  if (form.expected_version.trim()) {
    const expectedVersion = Number(form.expected_version.trim())
    if (!Number.isInteger(expectedVersion) || expectedVersion < 0) {
      return '单据版本必须为大于等于 0 的整数'
    }
  }

  const effectiveItems = getEffectiveItems(form.items)
  if (effectiveItems.length === 0) {
    return '请至少添加一条出库明细'
  }

  for (let index = 0; index < effectiveItems.length; index += 1) {
    const item = effectiveItems[index]
    const row = index + 1

    if (!item) {
      return `第 ${row} 行明细不存在，请重试`
    }

    const productId = Number(item.product_id.trim())
    if (!Number.isInteger(productId) || productId <= 0) {
      return `第 ${row} 行商品ID必须为正整数`
    }

    const qty = Number(item.qty.trim())
    if (!Number.isInteger(qty) || qty <= 0) {
      return `第 ${row} 行数量必须为正整数`
    }

    const sellPrice = item.sell_price.trim()
    if (!/^\d+(\.\d{1,4})?$/.test(sellPrice)) {
      return `第 ${row} 行销售单价格式错误（示例：3.50）`
    }

    if (item.expected_version.trim()) {
      const expectedVersion = Number(item.expected_version.trim())
      if (!Number.isInteger(expectedVersion) || expectedVersion < 0) {
        return `第 ${row} 行版本必须为大于等于 0 的整数`
      }
    }
  }

  return null
}

function buildPayload(): OutboundRequest {
  const items: OutboundItemRequest[] = getEffectiveItems(form.items).map((item) => {
    const nextItem: OutboundItemRequest = {
      product_id: Number(item.product_id.trim()),
      qty: Number(item.qty.trim()),
      sell_price: item.sell_price.trim(),
    }

    const expectedVersion = item.expected_version.trim()
    if (expectedVersion) {
      nextItem.expected_version = Number(expectedVersion)
    }

    return nextItem
  })

  const payload: OutboundRequest = {
    items,
  }

  const customerId = form.customer_id.trim()
  if (customerId) {
    payload.customer_id = Number(customerId)
  }

  const expectedVersion = form.expected_version.trim()
  if (expectedVersion) {
    payload.expected_version = Number(expectedVersion)
  }

  const remark = form.remark.trim()
  if (remark) {
    payload.remark = remark
  }

  return payload
}

async function submit(): Promise<void> {
  if (loading.value) {
    return
  }

  errorText.value = ''
  successText.value = ''

  const validationError = validateForm()
  if (validationError) {
    errorText.value = validationError
    return
  }

  loading.value = true
  try {
    const response = await outboundApi(buildPayload())
    result.value = response.data
    successText.value = `出库成功：${response.data.biz_no}，总金额 ${response.data.total_amount}`

    form.customer_id = ''
    form.expected_version = ''
    form.remark = ''
    form.items = []
  } catch (error) {
    errorText.value = formatApiError(error, '出库失败')
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <section>
    <h2>销售出库</h2>

    <p v-if="!canSubmit" class="warn-text">当前角色无销售出库权限，仅 OWNER/SALES 可操作。</p>

    <div class="card-panel form-grid" style="margin-bottom: 12px">
      <h3>条码出库（模式化）</h3>

      <form class="form-inline" @submit.prevent="scanByCurrentMode">
        <div class="form-label inline">
          <span>扫码模式</span>
          <div class="scan-mode-tabs" role="tablist" aria-label="扫码模式">
            <button
              class="scan-mode-tab"
              :class="{ 'is-active': scanMode === 'scan_confirm' }"
              type="button"
              :disabled="loading || scanLoading"
              @click="scanMode = 'scan_confirm'"
            >
              确认写入
            </button>
            <button
              class="scan-mode-tab"
              :class="{ 'is-active': scanMode === 'continuous_scan' }"
              type="button"
              :disabled="loading || scanLoading"
              @click="scanMode = 'continuous_scan'"
            >
              连续扫码
            </button>
          </div>
        </div>

        <label class="form-label inline">
          <span>条码</span>
          <input
            ref="scanBarcodeInputRef"
            v-model="scanForm.barcode"
            :disabled="loading || scanLoading"
            placeholder="扫码枪回车，例如：690123456789"
          />
        </label>

        <label class="form-label inline">
          <span>销售单价（可选）</span>
          <input
            v-model="scanForm.sell_price"
            :disabled="loading || scanLoading"
            placeholder="留空使用商品零售价"
          />
        </label>

        <button class="btn" type="submit" :disabled="loading || scanLoading || !canSubmit">
          <span class="btn-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24"><path d="M4 7h16v10H4z" /><path d="M12 7V5M12 19v-5" /></svg>
          </span>
          {{
            scanMode === 'continuous_scan'
              ? continuousSessionActive
                ? (scanLoading ? '识别中...' : '处理当前条码并累加')
                : '开始连续扫码会话'
              : (scanLoading ? '识别中...' : '按条码加入明细')
          }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="loading || scanLoading" @click="resetScanForm">
          <span class="btn-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24"><path d="M4 12a8 8 0 1 0 2-5.3" /><path d="M4 4v4h4" /></svg>
          </span>
          清空
        </button>
        <button
          v-if="scanMode === 'scan_confirm' && scanConfirmSessionActive"
          class="btn btn-secondary"
          type="button"
          :disabled="loading || scanLoading"
          @click="stopScanConfirmSession"
        >
          结束确认续扫
        </button>
        <button
          v-if="scanMode === 'continuous_scan' && continuousSessionActive"
          class="btn btn-secondary"
          type="button"
          :disabled="loading || scanLoading"
          @click="stopContinuousScanSession"
        >
          结束扫码
        </button>
      </form>

      <p class="table-summary">命中商品后自动填充出库明细；重复扫码同一商品自动累加数量。</p>
      <p v-if="scanMode === 'continuous_scan'" class="table-summary">
        连续扫码会话：{{ continuousSessionActive ? '进行中' : '未开始' }}，已处理 {{ continuousProcessedCount }} 条。
      </p>
      <p v-if="scanMode === 'scan_confirm'" class="table-summary">
        确认续扫会话：{{ scanConfirmSessionActive ? '进行中' : '未开始' }}，已处理 {{ scanConfirmProcessedCount }} 条。
      </p>
      <p v-if="scanErrorText" class="error-text">{{ scanErrorText }}</p>
      <p v-if="scanSuccessText" class="success-text">{{ scanSuccessText }}</p>
    </div>

    <div
      v-if="scanConfirmForm.visible"
      class="modal-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby="outbound-scan-confirm-title"
      @click.self="cancelScanConfirmDialog"
    >
      <div class="modal-card" @keydown.esc.prevent="cancelScanConfirmDialog">
        <h3 id="outbound-scan-confirm-title">确认写入</h3>
        <p class="table-summary">已命中商品：#{{ scanConfirmForm.product_id }} {{ scanConfirmForm.product_name }}</p>
        <div class="form-inline form-inline-compact">
          <label class="form-label inline">
            <span>数量 *</span>
            <input
              ref="scanConfirmQtyInputRef"
              v-model="scanConfirmForm.qty"
              :disabled="loading || scanLoading"
              placeholder="1"
              @keydown.enter.prevent="applyScanConfirm"
              @keydown.esc.prevent="cancelScanConfirmDialog"
            />
          </label>
          <label class="form-label inline">
            <span>销售单价 *</span>
            <input
              v-model="scanConfirmForm.sell_price"
              :disabled="loading || scanLoading"
              placeholder="3.50"
              @keydown.enter.prevent="applyScanConfirm"
              @keydown.esc.prevent="cancelScanConfirmDialog"
            />
          </label>
        </div>
        <div class="form-actions">
          <button class="btn" type="button" :disabled="loading || scanLoading" @click="applyScanConfirm">
            <span class="btn-icon" aria-hidden="true">
              <svg viewBox="0 0 24 24"><path d="m5 12 4 4 10-10" /></svg>
            </span>
            确认写入（Enter）
          </button>
          <button class="btn btn-secondary" type="button" :disabled="loading || scanLoading" @click="cancelScanConfirmDialog">
            取消（Esc）
          </button>
        </div>
      </div>
    </div>

    <form class="form-grid outbound-form" @submit.prevent="submit">
      <div class="card-panel form-grid">
        <div class="card-panel-header">
          <h3>单据头（可选）</h3>
          <button class="btn btn-secondary" type="button" :disabled="loading" @click="orderHeaderExpanded = !orderHeaderExpanded">
            {{ orderHeaderExpanded ? '收起' : '展开' }}
          </button>
        </div>

        <p v-if="!orderHeaderExpanded" class="table-summary">默认折叠。需要补充客户、备注或高级字段时可展开。</p>

        <template v-else>
          <div class="form-inline form-inline-compact">
            <label class="form-label inline">
              <span>客户ID</span>
              <input v-model="form.customer_id" placeholder="可选，正整数" />
            </label>
          </div>

          <label class="form-label">
            <span>备注</span>
            <input v-model="form.remark" placeholder="可选备注" />
          </label>

          <div class="form-grid" style="gap: 8px">
            <div class="form-actions">
              <button class="btn btn-secondary" type="button" :disabled="loading" @click="orderAdvancedExpanded = !orderAdvancedExpanded">
                {{ orderAdvancedExpanded ? '收起高级字段' : '展开高级字段' }}
              </button>
            </div>

            <div v-if="orderAdvancedExpanded" class="form-inline form-inline-compact">
              <label class="form-label inline">
                <span>单据版本（高级）</span>
                <input v-model="form.expected_version" placeholder="可选 expected_version" />
              </label>
            </div>
          </div>
        </template>
      </div>

      <div class="card-panel">
        <div class="card-panel-header">
          <h3>出库明细</h3>
          <button class="btn btn-secondary" type="button" :disabled="loading" @click="addItem">
            <span class="btn-icon" aria-hidden="true">
              <svg viewBox="0 0 24 24"><path d="M12 5v14M5 12h14" /></svg>
            </span>
            新增明细
          </button>
        </div>

        <div class="table-wrapper">
          <table class="data-table data-table-compact">
            <thead>
              <tr>
                <th>#</th>
                <th>商品ID *</th>
                <th>商品名称</th>
                <th>数量 *</th>
                <th>销售单价 *</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(item, index) in form.items" :key="item.local_id">
                <td>{{ index + 1 }}</td>
                <td>
                  <input
                    v-model="item.product_id"
                    class="table-input"
                    :readonly="!item.editable || loading"
                    placeholder="1001"
                  />
                </td>
                <td>
                  <input
                    v-model="item.product_name"
                    class="table-input"
                    :readonly="!item.editable || loading"
                    placeholder="可选，默认扫码回填"
                  />
                </td>
                <td>
                  <input
                    v-model="item.qty"
                    class="table-input"
                    :readonly="!item.editable || loading"
                    placeholder="1"
                  />
                </td>
                <td>
                  <input
                    v-model="item.sell_price"
                    class="table-input"
                    :readonly="!item.editable || loading"
                    placeholder="3.50"
                  />
                </td>
                <td>
                  <button class="btn btn-secondary" type="button" :disabled="loading" @click="toggleItemEditable(item.local_id)">
                    <span class="btn-icon" aria-hidden="true">
                      <svg viewBox="0 0 24 24"><path d="M4 20h4l10-10-4-4L4 16v4z" /><path d="m12 6 4 4" /></svg>
                    </span>
                    {{ item.editable ? '完成' : '编辑' }}
                  </button>
                  <button
                    class="btn btn-danger"
                    type="button"
                    :disabled="loading"
                    @click="removeItem(item.local_id)"
                  >
                    删除
                  </button>
                </td>
              </tr>
              <tr v-if="form.items.length === 0">
                <td colspan="6" class="empty-cell">暂无明细，请先扫码或点击“新增明细”。</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div class="form-actions">
        <button class="btn" type="submit" :disabled="loading">
          <span class="btn-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24"><path d="M12 5v14M5 12h14" /></svg>
          </span>
          {{ loading ? '提交中...' : '提交出库' }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="loading" @click="resetForm">
          重置
        </button>
      </div>

      <p v-if="errorText" class="error-text">{{ errorText }}</p>
      <p v-if="successText" class="success-text">{{ successText }}</p>
    </form>

    <div v-if="result" class="result-panel">
      <h3>最近一次出库结果</h3>
      <p>业务单号：{{ result.biz_no }}</p>
      <p>总金额：{{ result.total_amount }}</p>

      <div class="table-wrapper">
        <table class="data-table data-table-compact">
          <thead>
            <tr>
              <th>商品ID</th>
              <th>出库数量</th>
              <th>剩余库存</th>
              <th>版本</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in result.items" :key="`${item.product_id}-${item.version}`">
              <td>{{ item.product_id }}</td>
              <td>{{ item.qty }}</td>
              <td>{{ item.current_stock }}</td>
              <td>{{ item.version }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </section>
</template>