<script setup lang="ts">
import { computed, reactive, ref } from 'vue'

import {
  confirmSalesOrderApi,
  createSalesOrderApi,
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
}

interface SalesReturnItemForm {
  local_id: number
  product_id: string
  qty: string
}

const authStore = useAuthStore()

const loadingCreate = ref(false)
const loadingQuery = ref(false)
const loadingConfirm = ref(false)
const loadingVoid = ref(false)
const loadingReturn = ref(false)
const scanLoading = ref(false)

const errorText = ref('')
const successText = ref('')
const scanErrorText = ref('')
const scanSuccessText = ref('')
const orderResult = ref<SalesOrderData | null>(null)

let createItemSeed = 1
let returnItemSeed = 1

function generateBizNo(): string {
  const now = new Date()
  const pad = (value: number): string => value.toString().padStart(2, '0')
  const timePart = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`
  const randomPart = Math.random().toString(36).slice(2, 6).toUpperCase()
  return `SO-${timePart}-${randomPart}`
}

function createCreateItemForm(): SalesCreateItemForm {
  return {
    local_id: createItemSeed++,
    product_id: '',
    qty: '1',
    sell_price: '',
  }
}

function createReturnItemForm(): SalesReturnItemForm {
  return {
    local_id: returnItemSeed++,
    product_id: '',
    qty: '1',
  }
}

const createForm = reactive({
  biz_no: generateBizNo(),
  customer_id: '',
  remark: '',
  items: [createCreateItemForm()] as SalesCreateItemForm[],
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
  items: [createReturnItemForm()] as SalesReturnItemForm[],
})

const canOperate = computed(() => {
  const role = authStore.session?.user.role
  return role === 'OWNER' || role === 'SALES'
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

function removeCreateItem(localId: number): void {
  if (createForm.items.length <= 1) {
    return
  }

  const index = createForm.items.findIndex((item) => item.local_id === localId)
  if (index >= 0) {
    createForm.items.splice(index, 1)
  }
}

function addReturnItem(): void {
  returnForm.items.push(createReturnItemForm())
}

function removeReturnItem(localId: number): void {
  if (returnForm.items.length <= 1) {
    return
  }

  const index = returnForm.items.findIndex((item) => item.local_id === localId)
  if (index >= 0) {
    returnForm.items.splice(index, 1)
  }
}

function resetCreateForm(): void {
  createForm.biz_no = generateBizNo()
  createForm.customer_id = ''
  createForm.remark = ''
  createForm.items = [createCreateItemForm()]
  resetScanForm()
}

function resetReturnForm(): void {
  returnForm.expected_version = orderResult.value ? String(orderResult.value.version) : ''
  returnForm.remark = ''
  returnForm.items = [createReturnItemForm()]
}

function validateCreateForm(): string | null {
  if (!canOperate.value) {
    return '当前角色无销售单操作权限，仅 OWNER/SALES 可操作'
  }

  if (!createForm.biz_no.trim()) {
    return '请输入销售单业务单号'
  }

  if (createForm.customer_id.trim()) {
    const customerId = Number(createForm.customer_id.trim())
    if (!Number.isInteger(customerId) || customerId <= 0) {
      return '客户ID必须为正整数'
    }
  }

  if (createForm.items.length === 0) {
    return '请至少填写一条销售明细'
  }

  for (let index = 0; index < createForm.items.length; index += 1) {
    const row = index + 1
    const item = createForm.items[index]
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

  if (returnForm.items.length === 0) {
    return '请至少填写一条退货明细'
  }

  for (let index = 0; index < returnForm.items.length; index += 1) {
    const row = index + 1
    const item = returnForm.items[index]
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
  const items: SalesOrderCreateItemRequest[] = createForm.items.map((item) => ({
    product_id: Number(item.product_id.trim()),
    qty: Number(item.qty.trim()),
    sell_price: item.sell_price.trim(),
  }))

  const payload: SalesOrderCreateRequest = {
    biz_no: createForm.biz_no.trim(),
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
  const items: SalesOrderReturnItemRequest[] = returnForm.items.map((item) => ({
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
</script>

<template>
  <section>
    <h2>销售单状态流（创建 / 查询 / 确认 / 作废 / 退货）</h2>

    <p v-if="!canOperate" class="warn-text">当前角色无销售单操作权限，仅 OWNER/SALES 可操作。</p>

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
          {{ scanLoading ? '识别中...' : '扫码并累加' }}
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
          <span>业务单号 *</span>
          <input v-model="createForm.biz_no" :disabled="busy" placeholder="例如：SO-20260306-0001" />
        </label>

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
                  <input v-model="item.product_id" class="table-input" :disabled="busy" placeholder="1001" />
                </td>
                <td>
                  <input v-model="item.qty" class="table-input" :disabled="busy" placeholder="1" />
                </td>
                <td>
                  <input v-model="item.sell_price" class="table-input" :disabled="busy" placeholder="3.50" />
                </td>
                <td>
                  <button
                    class="btn btn-danger"
                    type="button"
                    :disabled="busy || createForm.items.length <= 1"
                    @click="removeCreateItem(item.local_id)"
                  >
                    删除
                  </button>
                </td>
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
                  <input v-model="item.product_id" class="table-input" :disabled="busy" placeholder="1001" />
                </td>
                <td>
                  <input v-model="item.qty" class="table-input" :disabled="busy" placeholder="1" />
                </td>
                <td>
                  <button
                    class="btn btn-danger"
                    type="button"
                    :disabled="busy || returnForm.items.length <= 1"
                    @click="removeReturnItem(item.local_id)"
                  >
                    删除
                  </button>
                </td>
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
              <th>销售数量</th>
              <th>销售单价</th>
              <th>已退数量</th>
              <th>可退数量</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(item, index) in orderResult.items" :key="`${item.product_id}-${index}`">
              <td>{{ item.product_id }}</td>
              <td>{{ item.qty }}</td>
              <td>{{ item.sell_price }}</td>
              <td>{{ item.returned_qty }}</td>
              <td>{{ Math.max(item.qty - item.returned_qty, 0) }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </section>
</template>
