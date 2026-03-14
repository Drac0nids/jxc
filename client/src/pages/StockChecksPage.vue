<script setup lang="ts">
import { computed, reactive, ref } from 'vue'

import {
  confirmStockCheckApi,
  createStockCheckApi,
  getStockCheckApi,
  startStockCheckApi,
} from '@/api/inventory'
import { scanProductApi } from '@/api/products'
import { ApiClientError } from '@/api/http'
import { useAuthStore } from '@/stores/auth'
import type {
  StockCheckConfirmItemRequest,
  StockCheckConfirmRequest,
  StockCheckCreateItemRequest,
  StockCheckCreateRequest,
  StockCheckData,
} from '@/types/api'

interface StockCheckCreateItemForm {
  local_id: number
  product_id: string
}

interface StockCheckConfirmItemForm {
  local_id: number
  product_id: number
  product_name: string
  book_stock: number
  actual_stock: string
}

const authStore = useAuthStore()

const loadingCreate = ref(false)
const loadingQuery = ref(false)
const loadingStart = ref(false)
const loadingConfirm = ref(false)
const scanLoading = ref(false)

const errorText = ref('')
const successText = ref('')
const scanErrorText = ref('')
const scanSuccessText = ref('')
const checkResult = ref<StockCheckData | null>(null)

let createItemSeed = 1
let confirmItemSeed = 1

function createCreateItemForm(): StockCheckCreateItemForm {
  return {
    local_id: createItemSeed++,
    product_id: '',
  }
}

const createForm = reactive({
  remark: '',
  items: [createCreateItemForm()] as StockCheckCreateItemForm[],
})

const scanForm = reactive({
  barcode: '',
})

const queryForm = reactive({
  id: '',
})

const startForm = reactive({
  expected_version: '',
})

const confirmForm = reactive({
  expected_version: '',
  remark: '',
  items: [] as StockCheckConfirmItemForm[],
})

const canOperate = computed(() => {
  const role = authStore.session?.user.role
  return role === 'OWNER' || role === 'PURCHASER'
})

const busy = computed(
  () =>
    loadingCreate.value ||
    loadingQuery.value ||
    loadingStart.value ||
    loadingConfirm.value ||
    scanLoading.value,
)

function clearMessage(): void {
  errorText.value = ''
  successText.value = ''
}

function resetScanForm(): void {
  scanForm.barcode = ''
  scanErrorText.value = ''
  scanSuccessText.value = ''
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

function resetCreateForm(): void {
  createForm.remark = ''
  createForm.items = [createCreateItemForm()]
  resetScanForm()
}

function parseExpectedVersion(raw: string): number | undefined {
  const trimmed = raw.trim()
  if (!trimmed) {
    return undefined
  }
  return Number(trimmed)
}

function parseStockCheckIdOrNull(): number | null {
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

function syncCheckContext(check: StockCheckData): void {
  checkResult.value = check
  queryForm.id = String(check.id)
  startForm.expected_version = String(check.version)
  confirmForm.expected_version = String(check.version)
  confirmForm.remark = check.remark ?? ''
  confirmForm.items = check.items.map((item) => ({
    local_id: confirmItemSeed++,
    product_id: item.product_id,
    product_name: item.product_name || `商品#${item.product_id}`,
    book_stock: item.book_stock,
    actual_stock: String(item.actual_stock ?? item.book_stock),
  }))
}

function validateCreateForm(): string | null {
  if (!canOperate.value) {
    return '当前角色无库存盘点操作权限，仅 OWNER/PURCHASER 可操作'
  }

  if (createForm.items.length === 0) {
    return '请至少填写一条盘点明细'
  }

  const seen = new Set<number>()
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

    if (seen.has(productId)) {
      return `第 ${row} 行商品ID重复，请检查`
    }
    seen.add(productId)
  }

  return null
}

function validateQueryForm(): string | null {
  const id = parseStockCheckIdOrNull()
  if (id === null) {
    return '请输入正确的盘点单ID（正整数）'
  }
  return null
}

function validateStartForm(): string | null {
  if (!canOperate.value) {
    return '当前角色无库存盘点操作权限，仅 OWNER/PURCHASER 可操作'
  }

  if (!checkResult.value) {
    return '请先查询盘点单'
  }

  if (checkResult.value.status !== 'DRAFT') {
    return `当前状态为 ${checkResult.value.status}，仅 DRAFT 状态可开始盘点`
  }

  const expectedVersion = parseExpectedVersion(startForm.expected_version)
  if (typeof expectedVersion === 'number' && (!Number.isInteger(expectedVersion) || expectedVersion < 0)) {
    return '开始盘点版本必须为大于等于 0 的整数'
  }

  return null
}

function validateConfirmForm(): string | null {
  if (!canOperate.value) {
    return '当前角色无库存盘点操作权限，仅 OWNER/PURCHASER 可操作'
  }

  if (!checkResult.value) {
    return '请先查询盘点单'
  }

  if (checkResult.value.status !== 'COUNTING') {
    return `当前状态为 ${checkResult.value.status}，仅 COUNTING 状态可确认盘点`
  }

  const expectedVersion = parseExpectedVersion(confirmForm.expected_version)
  if (typeof expectedVersion === 'number' && (!Number.isInteger(expectedVersion) || expectedVersion < 0)) {
    return '确认盘点版本必须为大于等于 0 的整数'
  }

  if (confirmForm.items.length === 0) {
    return '盘点确认明细不能为空，请重新查询后重试'
  }

  const expectedProductIds = new Set(checkResult.value.items.map((item) => item.product_id))
  const seen = new Set<number>()

  for (let index = 0; index < confirmForm.items.length; index += 1) {
    const row = index + 1
    const item = confirmForm.items[index]
    if (!item) {
      return `第 ${row} 行确认明细不存在，请重试`
    }

    if (seen.has(item.product_id)) {
      return `第 ${row} 行商品ID重复，请检查`
    }
    seen.add(item.product_id)

    if (!expectedProductIds.has(item.product_id)) {
      return `第 ${row} 行商品ID不在盘点单明细中`
    }

    const actualStock = Number(item.actual_stock.trim())
    if (!Number.isInteger(actualStock) || actualStock < 0) {
      return `第 ${row} 行实盘库存必须为大于等于 0 的整数`
    }
  }

  if (seen.size !== expectedProductIds.size) {
    return '确认明细与盘点单明细不一致，请重新查询后再确认'
  }

  return null
}

function buildCreatePayload(): StockCheckCreateRequest {
  const items: StockCheckCreateItemRequest[] = createForm.items.map((item) => ({
    product_id: Number(item.product_id.trim()),
  }))

  const payload: StockCheckCreateRequest = {
    items,
  }

  const remark = createForm.remark.trim()
  if (remark) {
    payload.remark = remark
  }

  return payload
}

function buildConfirmPayload(): StockCheckConfirmRequest {
  const items: StockCheckConfirmItemRequest[] = confirmForm.items.map((item) => ({
    product_id: item.product_id,
    actual_stock: Number(item.actual_stock.trim()),
  }))

  const payload: StockCheckConfirmRequest = {
    items,
  }

  const expectedVersion = parseExpectedVersion(confirmForm.expected_version)
  if (typeof expectedVersion === 'number') {
    payload.expected_version = expectedVersion
  }

  const remark = confirmForm.remark.trim()
  if (remark) {
    payload.remark = remark
  }

  return payload
}

function formatApiError(error: unknown, fallback: string): string {
  if (error instanceof ApiClientError) {
    return `${error.message}（code=${error.code}${error.requestId ? `, request_id=${error.requestId}` : ''}）`
  }
  return error instanceof Error ? error.message : fallback
}

async function scanAndAppendCreateItem(): Promise<void> {
  if (busy.value) {
    return
  }

  scanErrorText.value = ''
  scanSuccessText.value = ''

  if (!canOperate.value) {
    scanErrorText.value = '当前角色无库存盘点操作权限，仅 OWNER/PURCHASER 可操作'
    return
  }

  const barcode = scanForm.barcode.trim()
  if (!barcode) {
    scanErrorText.value = '请输入条码后再扫码'
    return
  }

  scanLoading.value = true
  try {
    const response = await scanProductApi(barcode)
    const productId = response.data.id

    const exists = createForm.items.some((item) => Number(item.product_id.trim()) === productId)
    if (exists) {
      scanSuccessText.value = `商品 #${productId} ${response.data.name} 已在盘点明细中，无需重复添加`
      return
    }

    const nextItem = createCreateItemForm()
    nextItem.product_id = String(productId)
    createForm.items.push(nextItem)
    scanSuccessText.value = `扫码成功：#${productId} ${response.data.name}，已加入盘点明细`
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

async function createStockCheck(): Promise<void> {
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
    const response = await createStockCheckApi(buildCreatePayload())
    syncCheckContext(response.data)
    successText.value = `盘点单创建成功：#${response.data.id}（${response.data.biz_no}）`
    resetCreateForm()
  } catch (error) {
    errorText.value = formatApiError(error, '盘点单创建失败')
  } finally {
    loadingCreate.value = false
  }
}

async function queryStockCheck(): Promise<void> {
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
    const response = await getStockCheckApi(id)
    syncCheckContext(response.data)
    successText.value = `盘点单查询成功：#${response.data.id}（状态：${response.data.status}）`
  } catch (error) {
    errorText.value = formatApiError(error, '盘点单查询失败')
  } finally {
    loadingQuery.value = false
  }
}

async function startStockCheck(): Promise<void> {
  if (busy.value) {
    return
  }

  clearMessage()
  const validationError = validateStartForm()
  if (validationError) {
    errorText.value = validationError
    return
  }

  loadingStart.value = true
  try {
    const id = checkResult.value?.id ?? Number(queryForm.id.trim())
    const expectedVersion = parseExpectedVersion(startForm.expected_version)
    const response = await startStockCheckApi(id, {
      expected_version: expectedVersion,
    })
    syncCheckContext(response.data)
    successText.value = `盘点单开始成功：#${response.data.id}（状态：${response.data.status}）`
  } catch (error) {
    errorText.value = formatApiError(error, '盘点单开始失败')
  } finally {
    loadingStart.value = false
  }
}

async function confirmStockCheck(): Promise<void> {
  if (busy.value) {
    return
  }

  clearMessage()
  const validationError = validateConfirmForm()
  if (validationError) {
    errorText.value = validationError
    return
  }

  loadingConfirm.value = true
  try {
    const id = checkResult.value?.id ?? Number(queryForm.id.trim())
    const response = await confirmStockCheckApi(id, buildConfirmPayload())
    syncCheckContext(response.data)
    successText.value = `盘点单确认成功：#${response.data.id}（状态：${response.data.status}）`
  } catch (error) {
    errorText.value = formatApiError(error, '盘点单确认失败')
  } finally {
    loadingConfirm.value = false
  }
}
</script>

<template>
  <section>
    <h2>库存盘点状态流（创建 / 查询 / 开始 / 确认）</h2>

    <p v-if="!canOperate" class="warn-text">当前角色无库存盘点操作权限，仅 OWNER/PURCHASER 可操作。</p>

    <div class="card-panel form-grid" style="margin-bottom: 12px">
      <h3>扫码选品（创建盘点单）</h3>

      <form class="form-inline" @submit.prevent="scanAndAppendCreateItem">
        <label class="form-label inline">
          <span>条码</span>
          <input
            v-model="scanForm.barcode"
            :disabled="busy"
            placeholder="扫码枪回车，例如：690123456789"
          />
        </label>

        <button class="btn" type="submit" :disabled="busy || !canOperate">
          {{ scanLoading ? '识别中...' : '按条码加入明细' }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="busy" @click="resetScanForm">
          清空
        </button>
      </form>

      <p class="table-summary">扫码命中后会自动追加到创建盘点明细；重复商品不会重复添加。</p>
      <p v-if="scanErrorText" class="error-text">{{ scanErrorText }}</p>
      <p v-if="scanSuccessText" class="success-text">{{ scanSuccessText }}</p>
    </div>

    <div class="card-panel form-grid">
      <h3>1）创建盘点单</h3>

      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>备注</span>
          <input v-model="createForm.remark" :disabled="busy" placeholder="可选备注" />
        </label>
      </div>

      <div class="card-panel">
        <div class="card-panel-header">
          <h3>盘点明细</h3>
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
        <button class="btn" type="button" :disabled="busy || !canOperate" @click="createStockCheck">
          {{ loadingCreate ? '创建中...' : '创建盘点单' }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="busy" @click="resetCreateForm">
          重置创建表单
        </button>
      </div>
    </div>

    <div class="card-panel form-grid" style="margin-top: 12px">
      <h3>2）查询 / 开始盘点</h3>

      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>盘点单ID *</span>
          <input v-model="queryForm.id" :disabled="busy" placeholder="输入盘点单ID，例如 1" />
        </label>

        <button class="btn" type="button" :disabled="busy" @click="queryStockCheck">
          {{ loadingQuery ? '查询中...' : '查询盘点单' }}
        </button>
      </div>

      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>开始盘点 expected_version</span>
          <input v-model="startForm.expected_version" :disabled="busy" placeholder="可选，建议默认当前版本" />
        </label>

        <button class="btn" type="button" :disabled="busy || !canOperate || !checkResult" @click="startStockCheck">
          {{ loadingStart ? '开始中...' : '开始盘点' }}
        </button>
      </div>
    </div>

    <div class="card-panel form-grid" style="margin-top: 12px">
      <h3>3）确认盘点</h3>

      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>确认盘点 expected_version</span>
          <input
            v-model="confirmForm.expected_version"
            :disabled="busy"
            placeholder="可选，建议默认当前版本"
          />
        </label>

        <label class="form-label inline">
          <span>确认备注</span>
          <input v-model="confirmForm.remark" :disabled="busy" placeholder="可选备注" />
        </label>
      </div>

      <div class="table-wrapper">
        <table class="data-table data-table-compact">
          <thead>
            <tr>
              <th>商品ID</th>
              <th>商品名称</th>
              <th>账面库存</th>
              <th>实盘库存 *</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="confirmForm.items.length === 0">
              <td class="empty-cell" colspan="4">请先查询盘点单以加载确认明细</td>
            </tr>
            <tr v-for="item in confirmForm.items" v-else :key="item.local_id">
              <td>{{ item.product_id }}</td>
              <td>{{ item.product_name }}</td>
              <td>{{ item.book_stock }}</td>
              <td>
                <input v-model="item.actual_stock" class="table-input" :disabled="busy" placeholder="请输入实盘库存" />
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="form-actions">
        <button class="btn" type="button" :disabled="busy || !canOperate || !checkResult" @click="confirmStockCheck">
          {{ loadingConfirm ? '确认中...' : '确认盘点' }}
        </button>
      </div>
    </div>

    <p v-if="errorText" class="error-text">{{ errorText }}</p>
    <p v-if="successText" class="success-text">{{ successText }}</p>

    <div v-if="checkResult" class="result-panel">
      <h3>最近一次盘点单结果</h3>
      <p>盘点单ID：{{ checkResult.id }}</p>
      <p>业务单号：{{ checkResult.biz_no }}</p>
      <p>状态：{{ checkResult.status }}</p>
      <p>版本：{{ checkResult.version }}</p>
      <p>开始时间：{{ checkResult.counting_at ?? '-' }}</p>
      <p>确认时间：{{ checkResult.confirmed_at ?? '-' }}</p>
      <p>创建时间：{{ checkResult.created_at }}</p>
      <p>更新时间：{{ checkResult.updated_at }}</p>
      <p>备注：{{ checkResult.remark ?? '-' }}</p>

      <div class="table-wrapper">
        <table class="data-table data-table-compact">
          <thead>
            <tr>
              <th>商品ID</th>
              <th>商品名称</th>
              <th>账面库存</th>
              <th>实盘库存</th>
              <th>差异数量</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in checkResult.items" :key="item.product_id">
              <td>{{ item.product_id }}</td>
              <td>{{ item.product_name || '-' }}</td>
              <td>{{ item.book_stock }}</td>
              <td>{{ item.actual_stock ?? '-' }}</td>
              <td>{{ item.delta_qty ?? '-' }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </section>
</template>