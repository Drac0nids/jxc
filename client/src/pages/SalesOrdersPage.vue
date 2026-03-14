<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { useRoute } from 'vue-router'

import {
  confirmSalesOrderApi,
  createSalesOrderApi,
  getDashboardOrdersDrilldownApi,
  getSalesOrderApi,
  returnSalesOrderApi,
  voidSalesOrderApi,
} from '@/api/inventory'
import { scanProductApi } from '@/api/products'
import { ApiClientError } from '@/api/http'
import { useAuthStore } from '@/stores/auth'
import type {
  SalesOrderCreateItemRequest,
  SalesOrderCreateRequest,
  DashboardOrdersDrilldownData,
  SalesOrderData,
  StockInsufficientErrorData,
  SalesOrderReturnItemRequest,
  SalesOrderReturnRequest,
} from '@/types/api'

interface SalesCreateItemForm {
  local_id: number
  product_id: string
  qty: string
  sell_price: string
  editable: boolean
}

interface SalesReturnItemForm {
  local_id: number
  product_id: string
  qty: string
  editable: boolean
}

const authStore = useAuthStore()
const route = useRoute()

function formatToday(): string {
  const now = new Date()
  const pad = (value: number): string => value.toString().padStart(2, '0')
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
}

function parseDateValue(raw: string): { normalized: string } | { error: string } {
  const value = raw.trim()
  const match = value.match(/^(\d{4})-(\d{2})-(\d{2})$/)
  if (!match) {
    return { error: '日期格式必须为 YYYY-MM-DD' }
  }

  const year = Number(match[1])
  const month = Number(match[2])
  const day = Number(match[3])
  const date = new Date(year, month - 1, day)

  if (
    Number.isNaN(date.getTime()) ||
    date.getFullYear() !== year ||
    date.getMonth() !== month - 1 ||
    date.getDate() !== day
  ) {
    return { error: '日期不合法，请重新选择' }
  }

  return { normalized: value }
}

const loadingCreate = ref(false)
const loadingQuery = ref(false)
const loadingConfirm = ref(false)
const loadingVoid = ref(false)
const loadingReturn = ref(false)
const scanLoading = ref(false)
const drilldownLoading = ref(false)

const errorText = ref('')
const successText = ref('')
const scanErrorText = ref('')
const scanSuccessText = ref('')
const drilldownErrorText = ref('')
const orderResult = ref<SalesOrderData | null>(null)
const drilldownResult = ref<DashboardOrdersDrilldownData | null>(null)

let createItemSeed = 1
let returnItemSeed = 1

function createCreateItemForm(): SalesCreateItemForm {
  return {
    local_id: createItemSeed++,
    product_id: '',
    qty: '1',
    sell_price: '',
    editable: false,
  }
}

function createReturnItemForm(): SalesReturnItemForm {
  return {
    local_id: returnItemSeed++,
    product_id: '',
    qty: '1',
    editable: false,
  }
}

const createForm = reactive({
  customer_id: '',
  remark: '',
  items: [] as SalesCreateItemForm[],
})

const scanForm = reactive({
  barcode: '',
  sell_price: '',
})

const queryForm = reactive({
  id: '',
})

const actionForm = reactive({
  expected_version: '',
})

const returnForm = reactive({
  expected_version: '',
  remark: '',
  items: [] as SalesReturnItemForm[],
})

const canOperate = computed(() => {
  const role = authStore.session?.user.role
  return role === 'OWNER' || role === 'SALES'
})

const drilldownQuery = reactive({
  start_date: formatToday(),
  end_date: formatToday(),
  page: '1',
  page_size: '20',
})

const drilldownCurrentPage = computed(() => {
  const page = Number(drilldownQuery.page)
  return Number.isInteger(page) && page > 0 ? page : 1
})

const drilldownPageSize = computed(() => {
  const pageSize = Number(drilldownQuery.page_size)
  return Number.isInteger(pageSize) && pageSize > 0 ? pageSize : 20
})

const drilldownTotalPages = computed(() => {
  const total = drilldownResult.value?.total ?? 0
  return Math.max(1, Math.ceil(total / drilldownPageSize.value))
})

const busy = computed(
  () =>
    loadingCreate.value ||
    loadingQuery.value ||
    loadingConfirm.value ||
    loadingVoid.value ||
    loadingReturn.value,
)

function clearMessage(): void {
  errorText.value = ''
  successText.value = ''
}

function readSingleQueryParam(raw: unknown): string | null {
  if (typeof raw === 'string') {
    return raw
  }
  if (Array.isArray(raw) && raw.length > 0) {
    const first = raw[0]
    return typeof first === 'string' ? first : null
  }
  return null
}

function parsePositiveIntOrDefault(raw: string | null, fallback: number): number {
  if (!raw) {
    return fallback
  }

  const value = Number(raw)
  if (!Number.isInteger(value) || value <= 0) {
    return fallback
  }

  return value
}

function initDrilldownFromRouteQuery(): void {
  const today = formatToday()
  const startDateFromQuery = readSingleQueryParam(route.query.start_date)
  const endDateFromQuery = readSingleQueryParam(route.query.end_date)

  const startParsed = startDateFromQuery ? parseDateValue(startDateFromQuery) : { normalized: today }
  const endParsed = endDateFromQuery ? parseDateValue(endDateFromQuery) : { normalized: today }

  drilldownQuery.start_date = 'error' in startParsed ? today : startParsed.normalized
  drilldownQuery.end_date = 'error' in endParsed ? today : endParsed.normalized
  drilldownQuery.page = String(parsePositiveIntOrDefault(readSingleQueryParam(route.query.page), 1))

  const pageSize = parsePositiveIntOrDefault(readSingleQueryParam(route.query.page_size), 20)
  drilldownQuery.page_size = String(Math.min(Math.max(pageSize, 1), 100))
}

function validateDrilldownQuery(): string | null {
  if (!canOperate.value) {
    return '当前角色无销售单下钻权限，仅 OWNER/SALES 可查看'
  }

  const startParsed = parseDateValue(drilldownQuery.start_date)
  if ('error' in startParsed) {
    return startParsed.error
  }

  const endParsed = parseDateValue(drilldownQuery.end_date)
  if ('error' in endParsed) {
    return endParsed.error
  }

  if (startParsed.normalized > endParsed.normalized) {
    return '开始日期不能晚于结束日期'
  }

  const page = Number(drilldownQuery.page)
  if (!Number.isInteger(page) || page <= 0) {
    return '页码必须为正整数'
  }

  const pageSize = Number(drilldownQuery.page_size)
  if (!Number.isInteger(pageSize) || pageSize < 1 || pageSize > 100) {
    return '每页条数必须为 1 ~ 100 的整数'
  }

  return null
}

async function fetchOrdersDrilldown(options?: { resetPage?: boolean }): Promise<void> {
  if (options?.resetPage) {
    drilldownQuery.page = '1'
  }

  drilldownErrorText.value = ''
  const validationError = validateDrilldownQuery()
  if (validationError) {
    drilldownErrorText.value = validationError
    drilldownResult.value = null
    return
  }

  drilldownLoading.value = true
  try {
    const response = await getDashboardOrdersDrilldownApi({
      start_date: drilldownQuery.start_date,
      end_date: drilldownQuery.end_date,
      page: Number(drilldownQuery.page),
      page_size: Number(drilldownQuery.page_size),
    })
    drilldownResult.value = response.data
    drilldownQuery.start_date = response.data.start_date
    drilldownQuery.end_date = response.data.end_date
    drilldownQuery.page = String(response.data.page)
    drilldownQuery.page_size = String(response.data.page_size)
  } catch (error) {
    drilldownErrorText.value = formatApiError(error, '订单下钻查询失败')
  } finally {
    drilldownLoading.value = false
  }
}

function resetDrilldownToToday(): void {
  const today = formatToday()
  drilldownQuery.start_date = today
  drilldownQuery.end_date = today
  drilldownQuery.page = '1'
  void fetchOrdersDrilldown()
}

function prevDrilldownPage(): void {
  if (drilldownCurrentPage.value <= 1) {
    return
  }
  drilldownQuery.page = String(drilldownCurrentPage.value - 1)
  void fetchOrdersDrilldown()
}

function nextDrilldownPage(): void {
  if (drilldownCurrentPage.value >= drilldownTotalPages.value) {
    return
  }
  drilldownQuery.page = String(drilldownCurrentPage.value + 1)
  void fetchOrdersDrilldown()
}

function resetScanForm(): void {
  scanForm.barcode = ''
  scanForm.sell_price = ''
  scanErrorText.value = ''
  scanSuccessText.value = ''
}

function parseExpectedVersion(raw: string): number | undefined {
  const trimmed = raw.trim()
  if (!trimmed) {
    return undefined
  }
  return Number(trimmed)
}

function parseOrderIdOrNull(): number | null {
  const raw = queryForm.id.trim()
  if (!raw) {
    return null
  }

  const id = Number(raw)
  if (!Number.isInteger(id) || id <= 0) {
    return null
  }

  return id
}

function addCreateItem(): void {
  createForm.items.push(createCreateItemForm())
}

function toggleCreateItemEditable(localId: number): void {
  const target = createForm.items.find((item) => item.local_id === localId)
  if (!target) {
    return
  }
  target.editable = !target.editable
}

function removeCreateItem(localId: number): void {
  const index = createForm.items.findIndex((item) => item.local_id === localId)
  if (index >= 0) {
    createForm.items.splice(index, 1)
  }
}

function addReturnItem(): void {
  returnForm.items.push(createReturnItemForm())
}

function toggleReturnItemEditable(localId: number): void {
  const target = returnForm.items.find((item) => item.local_id === localId)
  if (!target) {
    return
  }
  target.editable = !target.editable
}

function removeReturnItem(localId: number): void {
  const index = returnForm.items.findIndex((item) => item.local_id === localId)
  if (index >= 0) {
    returnForm.items.splice(index, 1)
  }
}

function resetCreateForm(): void {
  createForm.customer_id = ''
  createForm.remark = ''
  createForm.items = []
  resetScanForm()
}

function resetReturnForm(): void {
  returnForm.expected_version = orderResult.value ? String(orderResult.value.version) : ''
  returnForm.remark = ''
  returnForm.items = []
}

function isBlankCreateItem(item: SalesCreateItemForm): boolean {
  return !item.product_id.trim() && !item.qty.trim() && !item.sell_price.trim()
}

function getEffectiveCreateItems(items: SalesCreateItemForm[]): SalesCreateItemForm[] {
  return items.filter((item) => !isBlankCreateItem(item))
}

function isBlankReturnItem(item: SalesReturnItemForm): boolean {
  return !item.product_id.trim() && !item.qty.trim()
}

function getEffectiveReturnItems(items: SalesReturnItemForm[]): SalesReturnItemForm[] {
  return items.filter((item) => !isBlankReturnItem(item))
}

function validateCreateForm(): string | null {
  if (!canOperate.value) {
    return '当前角色无销售单操作权限，仅 OWNER/SALES 可操作'
  }

  if (createForm.customer_id.trim()) {
    const customerId = Number(createForm.customer_id.trim())
    if (!Number.isInteger(customerId) || customerId <= 0) {
      return '客户ID必须为正整数'
    }
  }

  const effectiveItems = getEffectiveCreateItems(createForm.items)
  if (effectiveItems.length === 0) {
    return '请至少填写一条销售明细'
  }

  for (let index = 0; index < effectiveItems.length; index += 1) {
    const row = index + 1
    const item = effectiveItems[index]
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

    if (!/^\d+(\.\d{1,4})?$/.test(item.sell_price.trim())) {
      return `第 ${row} 行销售单价格式错误（示例：3.50）`
    }
  }

  return null
}

function validateQueryForm(): string | null {
  const id = parseOrderIdOrNull()
  if (id === null) {
    return '请输入正确的销售单ID（正整数）'
  }
  return null
}

function validateActionForm(): string | null {
  if (!canOperate.value) {
    return '当前角色无销售单操作权限，仅 OWNER/SALES 可操作'
  }

  if (!orderResult.value) {
    return '请先查询销售单'
  }

  const expectedVersion = parseExpectedVersion(actionForm.expected_version)
  if (typeof expectedVersion === 'number' && (!Number.isInteger(expectedVersion) || expectedVersion < 0)) {
    return '版本必须为大于等于 0 的整数'
  }

  return null
}

function validateReturnForm(): string | null {
  if (!canOperate.value) {
    return '当前角色无销售单操作权限，仅 OWNER/SALES 可操作'
  }

  if (!orderResult.value) {
    return '请先查询销售单'
  }

  const expectedVersion = parseExpectedVersion(returnForm.expected_version)
  if (typeof expectedVersion === 'number' && (!Number.isInteger(expectedVersion) || expectedVersion < 0)) {
    return '退货版本必须为大于等于 0 的整数'
  }

  const effectiveItems = getEffectiveReturnItems(returnForm.items)
  if (effectiveItems.length === 0) {
    return '请至少填写一条退货明细'
  }

  for (let index = 0; index < effectiveItems.length; index += 1) {
    const row = index + 1
    const item = effectiveItems[index]
    if (!item) {
      return `第 ${row} 行退货明细不存在，请重试`
    }

    const productId = Number(item.product_id.trim())
    if (!Number.isInteger(productId) || productId <= 0) {
      return `第 ${row} 行商品ID必须为正整数`
    }

    const qty = Number(item.qty.trim())
    if (!Number.isInteger(qty) || qty <= 0) {
      return `第 ${row} 行退货数量必须为正整数`
    }
  }

  return null
}

function buildCreatePayload(): SalesOrderCreateRequest {
  const items: SalesOrderCreateItemRequest[] = getEffectiveCreateItems(createForm.items).map((item) => ({
    product_id: Number(item.product_id.trim()),
    qty: Number(item.qty.trim()),
    sell_price: item.sell_price.trim(),
  }))

  const payload: SalesOrderCreateRequest = {
    items,
  }

  const customerIdRaw = createForm.customer_id.trim()
  if (customerIdRaw) {
    payload.customer_id = Number(customerIdRaw)
  }

  const remark = createForm.remark.trim()
  if (remark) {
    payload.remark = remark
  }

  return payload
}

function buildReturnPayload(): SalesOrderReturnRequest {
  const items: SalesOrderReturnItemRequest[] = getEffectiveReturnItems(returnForm.items).map((item) => ({
    product_id: Number(item.product_id.trim()),
    qty: Number(item.qty.trim()),
  }))

  const payload: SalesOrderReturnRequest = {
    items,
  }

  const expectedVersion = parseExpectedVersion(returnForm.expected_version)
  if (typeof expectedVersion === 'number') {
    payload.expected_version = expectedVersion
  }

  const remark = returnForm.remark.trim()
  if (remark) {
    payload.remark = remark
  }

  return payload
}

function syncOrderContext(order: SalesOrderData): void {
  orderResult.value = order
  queryForm.id = String(order.id)
  actionForm.expected_version = String(order.version)
  returnForm.expected_version = String(order.version)
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

async function scanAndAccumulateCreateItem(): Promise<void> {
  if (busy.value || scanLoading.value) {
    return
  }

  scanErrorText.value = ''
  scanSuccessText.value = ''

  if (!canOperate.value) {
    scanErrorText.value = '当前角色无销售单操作权限，仅 OWNER/SALES 可操作'
    return
  }

  const barcode = scanForm.barcode.trim()
  if (!barcode) {
    scanErrorText.value = '请输入条码后再扫码'
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
    const productId = String(response.data.id)
    const sellPrice = customSellPrice || response.data.retail_price

    const effectiveItems = getEffectiveCreateItems(createForm.items)
    if (effectiveItems.length !== createForm.items.length) {
      createForm.items = effectiveItems
    }

    const existed = createForm.items.find((item) => item.product_id.trim() === productId)
    if (existed) {
      const currentQty = Number(existed.qty.trim())
      const safeQty = Number.isInteger(currentQty) && currentQty > 0 ? currentQty : 0
      const nextQty = safeQty + 1
      existed.qty = String(nextQty)
      if (!existed.sell_price.trim()) {
        existed.sell_price = sellPrice
      }
      scanSuccessText.value = `扫码成功：#${response.data.id} ${response.data.name}，已累加数量到 ${nextQty}`
      return
    }

    const nextItem = createCreateItemForm()
    nextItem.product_id = productId
    nextItem.qty = '1'
    nextItem.sell_price = sellPrice
    createForm.items.push(nextItem)
    scanSuccessText.value = `扫码成功：#${response.data.id} ${response.data.name}，已新增销售明细（数量=1）`
  } catch (error) {
    if (error instanceof ApiClientError && error.code === 4040) {
      scanErrorText.value = '未找到该条码对应商品，请先前往商品管理建档。'
      return
    }

    scanErrorText.value = formatApiError(error, '扫码选品失败')
  } finally {
    scanLoading.value = false
  }
}

async function createOrder(): Promise<void> {
  if (busy.value) {
    return
  }

  clearMessage()
  const validationError = validateCreateForm()
  if (validationError) {
    errorText.value = validationError
    return
  }

  loadingCreate.value = true
  try {
    const response = await createSalesOrderApi(buildCreatePayload())
    syncOrderContext(response.data)
    successText.value = `销售单创建成功：#${response.data.id}（${response.data.biz_no}）`
    resetCreateForm()
    resetReturnForm()
  } catch (error) {
    errorText.value = formatApiError(error, '销售单创建失败')
  } finally {
    loadingCreate.value = false
  }
}

async function queryOrder(): Promise<void> {
  if (busy.value) {
    return
  }

  clearMessage()
  const validationError = validateQueryForm()
  if (validationError) {
    errorText.value = validationError
    return
  }

  const id = Number(queryForm.id.trim())
  loadingQuery.value = true
  try {
    const response = await getSalesOrderApi(id)
    syncOrderContext(response.data)
    successText.value = `销售单查询成功：#${response.data.id}（状态：${response.data.status}）`
  } catch (error) {
    errorText.value = formatApiError(error, '销售单查询失败')
  } finally {
    loadingQuery.value = false
  }
}

async function confirmOrder(): Promise<void> {
  if (busy.value) {
    return
  }

  clearMessage()
  const validationError = validateActionForm()
  if (validationError) {
    errorText.value = validationError
    return
  }

  loadingConfirm.value = true
  try {
    const orderId = orderResult.value?.id ?? Number(queryForm.id.trim())
    const expectedVersion = parseExpectedVersion(actionForm.expected_version)
    const response = await confirmSalesOrderApi(orderId, {
      expected_version: expectedVersion,
    })
    syncOrderContext(response.data)
    successText.value = `销售单确认成功：#${response.data.id}（状态：${response.data.status}）`
  } catch (error) {
    errorText.value = formatApiError(error, '销售单确认失败')
  } finally {
    loadingConfirm.value = false
  }
}

async function voidOrder(): Promise<void> {
  if (busy.value) {
    return
  }

  clearMessage()
  const validationError = validateActionForm()
  if (validationError) {
    errorText.value = validationError
    return
  }

  loadingVoid.value = true
  try {
    const orderId = orderResult.value?.id ?? Number(queryForm.id.trim())
    const expectedVersion = parseExpectedVersion(actionForm.expected_version)
    const response = await voidSalesOrderApi(orderId, {
      expected_version: expectedVersion,
    })
    syncOrderContext(response.data)
    successText.value = `销售单作废成功：#${response.data.id}（状态：${response.data.status}）`
  } catch (error) {
    errorText.value = formatApiError(error, '销售单作废失败')
  } finally {
    loadingVoid.value = false
  }
}

async function returnOrder(): Promise<void> {
  if (busy.value) {
    return
  }

  clearMessage()
  const validationError = validateReturnForm()
  if (validationError) {
    errorText.value = validationError
    return
  }

  loadingReturn.value = true
  try {
    const orderId = orderResult.value?.id ?? Number(queryForm.id.trim())
    const response = await returnSalesOrderApi(orderId, buildReturnPayload())
    syncOrderContext(response.data)
    successText.value = `销售单退货成功：#${response.data.id}（状态：${response.data.status}）`
    resetReturnForm()
  } catch (error) {
    errorText.value = formatApiError(error, '销售单退货失败')
  } finally {
    loadingReturn.value = false
  }
}

onMounted(() => {
  initDrilldownFromRouteQuery()
  if (canOperate.value) {
    void fetchOrdersDrilldown()
  }
})
</script>

<template>
  <section>
    <h2>销售单状态流（创建 / 查询 / 确认 / 作废 / 退货）</h2>

    <p v-if="!canOperate" class="warn-text">当前角色无销售单操作权限，仅 OWNER/SALES 可操作。</p>

    <div class="card-panel form-grid" style="margin-bottom: 12px">
      <h3>订单下钻列表（来自经营看板订单数）</h3>

      <form class="form-inline" @submit.prevent="fetchOrdersDrilldown({ resetPage: true })">
        <label class="form-label inline">
          <span>开始日期</span>
          <input v-model="drilldownQuery.start_date" :disabled="drilldownLoading" type="date" />
        </label>

        <label class="form-label inline">
          <span>结束日期</span>
          <input v-model="drilldownQuery.end_date" :disabled="drilldownLoading" type="date" />
        </label>

        <label class="form-label inline">
          <span>每页</span>
          <input
            v-model="drilldownQuery.page_size"
            :disabled="drilldownLoading"
            type="number"
            min="1"
            max="100"
            style="width: 90px"
          />
        </label>

        <button class="btn" type="submit" :disabled="drilldownLoading || !canOperate">
          {{ drilldownLoading ? '查询中...' : '查询下钻订单' }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="drilldownLoading" @click="resetDrilldownToToday">
          重置为今天
        </button>
      </form>

      <p v-if="drilldownErrorText" class="error-text">{{ drilldownErrorText }}</p>

      <p v-if="drilldownResult" class="table-summary">
        区间：{{ drilldownResult.start_date }} ~ {{ drilldownResult.end_date }}；共 {{ drilldownResult.total }} 单；当前第
        {{ drilldownResult.page }} / {{ drilldownTotalPages }} 页
      </p>

      <div class="order-receipt-list" v-if="drilldownResult">
        <article
          v-for="(item, index) in drilldownResult.list"
          :key="`${item.biz_no}-${index}`"
          class="order-receipt-card"
        >
          <header class="order-receipt-head">
            <div>
              <p class="order-receipt-biz">{{ item.biz_no }}</p>
              <p class="order-receipt-id">销售单ID：{{ item.id }}</p>
            </div>

            <div class="order-receipt-head-meta">
              <span class="order-status-badge">{{ item.status }}</span>
              <strong class="order-receipt-total">总金额：{{ item.total_amount }}</strong>
            </div>
          </header>

          <div class="table-wrapper" v-if="item.items.length > 0">
            <table class="data-table data-table-compact">
              <thead>
                <tr>
                  <th>商品名称</th>
                  <th>数量</th>
                  <th>单价</th>
                  <th>小计</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="(line, lineIndex) in item.items" :key="`${item.biz_no}-${line.product_id}-${lineIndex}`">
                  <td>{{ line.product_name }}</td>
                  <td>{{ line.qty }}</td>
                  <td>{{ line.sell_price }}</td>
                  <td>{{ line.line_amount }}</td>
                </tr>
              </tbody>
            </table>
          </div>

          <p v-else class="order-empty-note">无明细（聚合行）</p>

          <footer class="order-receipt-foot">
            <p>创建时间：{{ item.created_at }}</p>
            <p>更新时间：{{ item.updated_at }}</p>
            <p>备注：{{ item.remark ?? '-' }}</p>
          </footer>
        </article>

        <p v-if="drilldownResult.list.length === 0" class="empty-cell order-receipt-empty">当前筛选条件下暂无订单。</p>
      </div>

      <div class="form-actions" v-if="drilldownResult">
        <button class="btn btn-secondary" type="button" :disabled="drilldownLoading || drilldownCurrentPage <= 1" @click="prevDrilldownPage">
          上一页
        </button>
        <button
          class="btn btn-secondary"
          type="button"
          :disabled="drilldownLoading || drilldownCurrentPage >= drilldownTotalPages"
          @click="nextDrilldownPage"
        >
          下一页
        </button>
      </div>
    </div>

    <div class="card-panel form-grid" style="margin-bottom: 12px">
      <h3>扫码选品（创建销售单）</h3>

      <form class="form-inline" @submit.prevent="scanAndAccumulateCreateItem">
        <label class="form-label inline">
          <span>条码</span>
          <input
            v-model="scanForm.barcode"
            :disabled="busy || scanLoading"
            placeholder="扫码枪回车，例如：690123456789"
          />
        </label>

        <label class="form-label inline">
          <span>销售单价（可选）</span>
          <input
            v-model="scanForm.sell_price"
            :disabled="busy || scanLoading"
            placeholder="留空使用商品零售价"
          />
        </label>

        <button class="btn" type="submit" :disabled="busy || scanLoading || !canOperate">
          {{ scanLoading ? '识别中...' : '按条码加入明细' }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="busy || scanLoading" @click="resetScanForm">
          清空
        </button>
      </form>

      <p class="table-summary">命中商品后自动写入“创建销售单”明细，重复扫码同一商品自动累加数量。</p>
      <p v-if="scanErrorText" class="error-text">{{ scanErrorText }}</p>
      <p v-if="scanSuccessText" class="success-text">{{ scanSuccessText }}</p>
    </div>

    <div class="card-panel form-grid">
      <h3>1）创建销售单</h3>

      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>客户ID</span>
          <input v-model="createForm.customer_id" :disabled="busy" placeholder="可选，正整数" />
        </label>
      </div>

      <label class="form-label">
        <span>备注</span>
        <input v-model="createForm.remark" :disabled="busy" placeholder="可选备注" />
      </label>

      <div class="card-panel">
        <div class="card-panel-header">
          <h3>销售明细</h3>
          <button class="btn btn-secondary" type="button" :disabled="busy" @click="addCreateItem">
            新增明细
          </button>
        </div>

        <div class="table-wrapper">
          <table class="data-table data-table-compact">
            <thead>
              <tr>
                <th>#</th>
                <th>商品ID *</th>
                <th>数量 *</th>
                <th>销售单价 *</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(item, index) in createForm.items" :key="item.local_id">
                <td>{{ index + 1 }}</td>
                <td>
                  <input
                    v-model="item.product_id"
                    class="table-input"
                    :readonly="!item.editable || busy"
                    :disabled="busy"
                    placeholder="1001"
                  />
                </td>
                <td>
                  <input
                    v-model="item.qty"
                    class="table-input"
                    :readonly="!item.editable || busy"
                    :disabled="busy"
                    placeholder="1"
                  />
                </td>
                <td>
                  <input
                    v-model="item.sell_price"
                    class="table-input"
                    :readonly="!item.editable || busy"
                    :disabled="busy"
                    placeholder="3.50"
                  />
                </td>
                <td>
                  <button class="btn btn-secondary" type="button" :disabled="busy" @click="toggleCreateItemEditable(item.local_id)">
                    {{ item.editable ? '完成' : '编辑' }}
                  </button>
                  <button
                    class="btn btn-danger"
                    type="button"
                    :disabled="busy"
                    @click="removeCreateItem(item.local_id)"
                  >
                    删除
                  </button>
                </td>
              </tr>
              <tr v-if="createForm.items.length === 0">
                <td colspan="5" class="empty-cell">暂无明细，请先扫码或点击“新增明细”。</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div class="form-actions">
        <button class="btn" type="button" :disabled="busy || !canOperate" @click="createOrder">
          {{ loadingCreate ? '创建中...' : '创建销售单' }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="busy" @click="resetCreateForm">
          重置创建表单
        </button>
      </div>
    </div>

    <div class="card-panel form-grid" style="margin-top: 12px">
      <h3>2）查询 / 确认 / 作废</h3>

      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>销售单ID *</span>
          <input v-model="queryForm.id" :disabled="busy" placeholder="输入销售单ID，例如 1" />
        </label>

        <button class="btn" type="button" :disabled="busy" @click="queryOrder">
          {{ loadingQuery ? '查询中...' : '查询销售单' }}
        </button>
      </div>

      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>expected_version</span>
          <input v-model="actionForm.expected_version" :disabled="busy" placeholder="可选，建议默认当前版本" />
        </label>

        <button class="btn" type="button" :disabled="busy || !canOperate || !orderResult" @click="confirmOrder">
          {{ loadingConfirm ? '确认中...' : '确认销售单' }}
        </button>
        <button class="btn btn-danger" type="button" :disabled="busy || !canOperate || !orderResult" @click="voidOrder">
          {{ loadingVoid ? '作废中...' : '作废销售单' }}
        </button>
      </div>
    </div>

    <div class="card-panel form-grid" style="margin-top: 12px">
      <h3>3）销售退货</h3>

      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>退货 expected_version</span>
          <input
            v-model="returnForm.expected_version"
            :disabled="busy"
            placeholder="可选，建议默认当前版本"
          />
        </label>

        <label class="form-label inline">
          <span>退货备注</span>
          <input v-model="returnForm.remark" :disabled="busy" placeholder="可选备注" />
        </label>
      </div>

      <div class="card-panel">
        <div class="card-panel-header">
          <h3>退货明细</h3>
          <button class="btn btn-secondary" type="button" :disabled="busy" @click="addReturnItem">
            新增退货明细
          </button>
        </div>

        <div class="table-wrapper">
          <table class="data-table data-table-compact">
            <thead>
              <tr>
                <th>#</th>
                <th>商品ID *</th>
                <th>退货数量 *</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(item, index) in returnForm.items" :key="item.local_id">
                <td>{{ index + 1 }}</td>
                <td>
                  <input
                    v-model="item.product_id"
                    class="table-input"
                    :readonly="!item.editable || busy"
                    :disabled="busy"
                    placeholder="1001"
                  />
                </td>
                <td>
                  <input
                    v-model="item.qty"
                    class="table-input"
                    :readonly="!item.editable || busy"
                    :disabled="busy"
                    placeholder="1"
                  />
                </td>
                <td>
                  <button class="btn btn-secondary" type="button" :disabled="busy" @click="toggleReturnItemEditable(item.local_id)">
                    {{ item.editable ? '完成' : '编辑' }}
                  </button>
                  <button
                    class="btn btn-danger"
                    type="button"
                    :disabled="busy"
                    @click="removeReturnItem(item.local_id)"
                  >
                    删除
                  </button>
                </td>
              </tr>
              <tr v-if="returnForm.items.length === 0">
                <td colspan="4" class="empty-cell">暂无退货明细，请点击“新增退货明细”。</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div class="form-actions">
        <button class="btn" type="button" :disabled="busy || !canOperate || !orderResult" @click="returnOrder">
          {{ loadingReturn ? '退货中...' : '提交退货' }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="busy" @click="resetReturnForm">
          重置退货表单
        </button>
      </div>
    </div>

    <p v-if="errorText" class="error-text">{{ errorText }}</p>
    <p v-if="successText" class="success-text">{{ successText }}</p>

    <div v-if="orderResult" class="result-panel">
      <h3>最近一次销售单结果</h3>
      <p>销售单ID：{{ orderResult.id }}</p>
      <p>业务单号：{{ orderResult.biz_no }}</p>
      <p>客户ID：{{ orderResult.customer_id ?? '-' }}</p>
      <p>状态：{{ orderResult.status }}</p>
      <p>总金额：{{ orderResult.total_amount }}</p>
      <p>版本：{{ orderResult.version }}</p>
      <p>确认时间：{{ orderResult.confirmed_at ?? '-' }}</p>
      <p>退货时间：{{ orderResult.returned_at ?? '-' }}</p>
      <p>作废时间：{{ orderResult.voided_at ?? '-' }}</p>
      <p>创建时间：{{ orderResult.created_at }}</p>
      <p>更新时间：{{ orderResult.updated_at }}</p>
      <p>备注：{{ orderResult.remark ?? '-' }}</p>

      <div class="table-wrapper">
        <table class="data-table data-table-compact">
          <thead>
            <tr>
              <th>商品ID</th>
              <th>商品名称</th>
              <th>销售数量</th>
              <th>销售单价</th>
              <th>行小计</th>
              <th>已退数量</th>
              <th>可退数量</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(item, index) in orderResult.items" :key="`${item.product_id}-${index}`">
              <td>{{ item.product_id }}</td>
              <td>{{ item.product_name }}</td>
              <td>{{ item.qty }}</td>
              <td>{{ item.sell_price }}</td>
              <td>{{ item.line_amount }}</td>
              <td>{{ item.returned_qty }}</td>
              <td>{{ Math.max(item.qty - item.returned_qty, 0) }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </section>
</template>
