<script setup lang="ts">
import { computed, reactive, ref } from 'vue'

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

interface OutboundFormItem {
  local_id: number
  product_id: string
  qty: string
  sell_price: string
  expected_version: string
}

const loading = ref(false)
const scanLoading = ref(false)
const errorText = ref('')
const successText = ref('')
const scanErrorText = ref('')
const scanSuccessText = ref('')
const result = ref<OutboundResponseData | null>(null)

const authStore = useAuthStore()

let itemSeed = 1

function generateBizNo(): string {
  const now = new Date()
  const pad = (value: number): string => value.toString().padStart(2, '0')
  const timePart = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`
  const randomPart = Math.random().toString(36).slice(2, 6).toUpperCase()
  return `SO-${timePart}-${randomPart}`
}

function createFormItem(): OutboundFormItem {
  return {
    local_id: itemSeed++,
    product_id: '',
    qty: '1',
    sell_price: '',
    expected_version: '',
  }
}

const form = reactive({
  biz_no: generateBizNo(),
  customer_id: '',
  expected_version: '',
  remark: '',
  items: [createFormItem()] as OutboundFormItem[],
})

const scanForm = reactive({
  barcode: '',
  sell_price: '',
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

function resetScanForm(): void {
  scanForm.barcode = ''
  scanForm.sell_price = ''
  scanErrorText.value = ''
  scanSuccessText.value = ''
}

async function scanAndAccumulate(): Promise<void> {
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
    scanErrorText.value = '请输入条码后再扫码出库'
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
    const existed = form.items.find((item) => item.product_id.trim() === scannedProductId)

    if (existed) {
      const currentQty = Number(existed.qty.trim())
      const safeQty = Number.isInteger(currentQty) && currentQty > 0 ? currentQty : 0
      const nextQty = safeQty + 1
      existed.qty = String(nextQty)

      if (!existed.sell_price.trim()) {
        existed.sell_price = sellPrice
      }

      scanSuccessText.value = `扫码成功：#${response.data.id} ${response.data.name}，已累加到第 1 个匹配明细，数量=${nextQty}`
    } else {
      const nextItem = createFormItem()
      nextItem.product_id = scannedProductId
      nextItem.qty = '1'
      nextItem.sell_price = sellPrice
      form.items.push(nextItem)
      scanSuccessText.value = `扫码成功：#${response.data.id} ${response.data.name}，已新增出库明细（数量=1）`
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

function addItem(): void {
  form.items.push(createFormItem())
}

function removeItem(localId: number): void {
  if (form.items.length <= 1) {
    return
  }

  const index = form.items.findIndex((item) => item.local_id === localId)
  if (index >= 0) {
    form.items.splice(index, 1)
  }
}

function resetForm(): void {
  form.biz_no = generateBizNo()
  form.customer_id = ''
  form.expected_version = ''
  form.remark = ''
  form.items = [createFormItem()]

  errorText.value = ''
  successText.value = ''
  result.value = null
  resetScanForm()
}

function validateForm(): string | null {
  if (!canSubmit.value) {
    return '当前角色无销售出库权限，仅 OWNER/SALES 可操作'
  }

  if (!form.biz_no.trim()) {
    return '请输入业务单号'
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

  if (form.items.length === 0) {
    return '请至少添加一条出库明细'
  }

  for (let index = 0; index < form.items.length; index += 1) {
    const item = form.items[index]
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
  const items: OutboundItemRequest[] = form.items.map((item) => {
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
    biz_no: form.biz_no.trim(),
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

    form.biz_no = generateBizNo()
    form.customer_id = ''
    form.expected_version = ''
    form.remark = ''
    form.items = [createFormItem()]
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
      <h3>扫码出库（快速累加）</h3>

      <form class="form-inline" @submit.prevent="scanAndAccumulate">
        <label class="form-label inline">
          <span>条码</span>
          <input
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
          {{ scanLoading ? '识别中...' : '扫码并累加' }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="loading || scanLoading" @click="resetScanForm">
          清空
        </button>
      </form>

      <p class="table-summary">命中商品后自动填充出库明细；重复扫码同一商品自动累加数量。</p>
      <p v-if="scanErrorText" class="error-text">{{ scanErrorText }}</p>
      <p v-if="scanSuccessText" class="success-text">{{ scanSuccessText }}</p>
    </div>

    <form class="form-grid outbound-form" @submit.prevent="submit">
      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>业务单号 *</span>
          <input v-model="form.biz_no" placeholder="例如：SO-20260305-0001" />
        </label>

        <label class="form-label inline">
          <span>客户ID</span>
          <input v-model="form.customer_id" placeholder="可选，正整数" />
        </label>

        <label class="form-label inline">
          <span>单据版本</span>
          <input v-model="form.expected_version" placeholder="可选 expected_version" />
        </label>
      </div>

      <label class="form-label">
        <span>备注</span>
        <input v-model="form.remark" placeholder="可选备注" />
      </label>

      <div class="card-panel">
        <div class="card-panel-header">
          <h3>出库明细</h3>
          <button class="btn btn-secondary" type="button" :disabled="loading" @click="addItem">
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
                <th>版本</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(item, index) in form.items" :key="item.local_id">
                <td>{{ index + 1 }}</td>
                <td>
                  <input v-model="item.product_id" class="table-input" placeholder="1001" />
                </td>
                <td>
                  <input v-model="item.qty" class="table-input" placeholder="1" />
                </td>
                <td>
                  <input v-model="item.sell_price" class="table-input" placeholder="3.50" />
                </td>
                <td>
                  <input v-model="item.expected_version" class="table-input" placeholder="可选" />
                </td>
                <td>
                  <button
                    class="btn btn-danger"
                    type="button"
                    :disabled="loading || form.items.length <= 1"
                    @click="removeItem(item.local_id)"
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
        <button class="btn" type="submit" :disabled="loading">
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