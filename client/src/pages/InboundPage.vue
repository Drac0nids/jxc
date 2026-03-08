<script setup lang="ts">
import { computed, reactive, ref } from 'vue'

import { inboundApi } from '@/api/inventory'
import { scanProductApi } from '@/api/products'
import { ApiClientError } from '@/api/http'
import { useAuthStore } from '@/stores/auth'
import type { InboundRequest, InboundResponseData } from '@/types/api'

const authStore = useAuthStore()

const loading = ref(false)
const scanLoading = ref(false)
const errorText = ref('')
const successText = ref('')
const scanErrorText = ref('')
const scanSuccessText = ref('')
const result = ref<InboundResponseData | null>(null)

function generateBizNo(): string {
  const now = new Date()
  const pad = (value: number): string => value.toString().padStart(2, '0')
  const timePart = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`
  const randomPart = Math.random().toString(36).slice(2, 6).toUpperCase()
  return `PO-${timePart}-${randomPart}`
}

const form = reactive({
  biz_no: generateBizNo(),
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

const canSubmit = computed(() => {
  const role = authStore.session?.user.role
  return role === 'OWNER' || role === 'PURCHASER'
})

function formatApiError(error: unknown, fallback: string): string {
  if (error instanceof ApiClientError) {
    return `${error.message}（code=${error.code}${error.requestId ? `, request_id=${error.requestId}` : ''}）`
  }

  return error instanceof Error ? error.message : fallback
}

function resetScanForm(): void {
  scanForm.barcode = ''
  scanErrorText.value = ''
  scanSuccessText.value = ''
}

async function scanAndAccumulate(): Promise<void> {
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
    scanErrorText.value = '请输入条码后再扫码入库'
    return
  }

  scanLoading.value = true
  try {
    const response = await scanProductApi(barcode)
    const scannedProductId = String(response.data.id)
    const currentProductId = form.product_id.trim()
    const currentBarcode = form.barcode.trim()
    const hasSelectedProduct = Boolean(currentProductId || currentBarcode)

    if (
      hasSelectedProduct &&
      currentProductId !== scannedProductId &&
      currentBarcode !== barcode
    ) {
      scanErrorText.value = `当前入库表单已选择商品（商品ID=${currentProductId || '-'}，条码=${currentBarcode || '-'}），请先重置或提交后再扫描其它商品。`
      return
    }

    form.product_id = scannedProductId
    form.barcode = barcode

    const currentQty = Number(form.qty.trim())
    const safeQty = Number.isInteger(currentQty) && currentQty > 0 ? currentQty : 0
    const nextQty = safeQty + 1
    form.qty = String(nextQty)

    scanSuccessText.value = `扫码成功：#${response.data.id} ${response.data.name}，数量已累加到 ${nextQty}`
  } catch (error) {
    if (error instanceof ApiClientError && error.code === 4040) {
      scanErrorText.value = '未找到该条码对应商品，请先前往“商品管理”完成建档。'
      return
    }

    scanErrorText.value = formatApiError(error, '扫码入库失败')
  } finally {
    scanLoading.value = false
  }
}

function resetForm(): void {
  form.biz_no = generateBizNo()
  form.product_id = ''
  form.barcode = ''
  form.qty = '1'
  form.unit_cost = ''
  form.expected_version = ''
  form.remark = ''

  errorText.value = ''
  successText.value = ''
  result.value = null
  resetScanForm()
}

function validateForm(): string | null {
  if (!canSubmit.value) {
    return '当前角色无采购入库权限，仅 OWNER/PURCHASER 可操作'
  }

  if (!form.biz_no.trim()) {
    return '请输入业务单号'
  }

  if (!form.product_id.trim() && !form.barcode.trim()) {
    return 'product_id 或 barcode 至少填写一个'
  }

  if (form.product_id.trim()) {
    const productId = Number(form.product_id.trim())
    if (!Number.isInteger(productId) || productId <= 0) {
      return '商品ID必须为正整数'
    }
  }

  const qty = Number(form.qty.trim())
  if (!Number.isInteger(qty) || qty <= 0) {
    return '数量必须为正整数'
  }

  if (!/^\d+(\.\d{1,4})?$/.test(form.unit_cost.trim())) {
    return '单次进价格式错误（示例：2.20）'
  }

  if (form.expected_version.trim()) {
    const expectedVersion = Number(form.expected_version.trim())
    if (!Number.isInteger(expectedVersion) || expectedVersion < 0) {
      return '版本必须为大于等于 0 的整数'
    }
  }

  return null
}

function buildPayload(): InboundRequest {
  const payload: InboundRequest = {
    biz_no: form.biz_no.trim(),
    qty: Number(form.qty.trim()),
    unit_cost: form.unit_cost.trim(),
  }

  const productId = form.product_id.trim()
  if (productId) {
    payload.product_id = Number(productId)
  }

  const barcode = form.barcode.trim()
  if (barcode) {
    payload.barcode = barcode
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
    const response = await inboundApi(buildPayload())
    result.value = response.data
    successText.value = `入库成功：${response.data.biz_no}，当前库存 ${response.data.current_stock}`

    form.biz_no = generateBizNo()
    form.qty = '1'
    form.unit_cost = ''
    form.expected_version = ''
    form.remark = ''
  } catch (error) {
    errorText.value = formatApiError(error, '入库失败')
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <section>
    <h2>采购入库</h2>

    <p v-if="!canSubmit" class="warn-text">当前角色无采购入库权限，仅 OWNER/PURCHASER 可操作。</p>

    <div class="card-panel form-grid" style="margin-bottom: 12px">
      <h3>扫码入库（快速累加）</h3>

      <form class="form-inline" @submit.prevent="scanAndAccumulate">
        <label class="form-label inline">
          <span>条码</span>
          <input
            v-model="scanForm.barcode"
            :disabled="scanLoading || loading"
            placeholder="扫码枪回车，例如：690123456789"
          />
        </label>

        <button class="btn" type="submit" :disabled="scanLoading || loading || !canSubmit">
          {{ scanLoading ? '识别中...' : '扫码并累加' }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="scanLoading || loading" @click="resetScanForm">
          清空
        </button>
      </form>

      <p class="table-summary">命中商品后将自动回填商品信息，并在当前入库表单中把数量 +1。</p>

      <p v-if="scanErrorText" class="error-text">{{ scanErrorText }}</p>
      <p v-if="scanSuccessText" class="success-text">{{ scanSuccessText }}</p>
    </div>

    <form class="form-grid" @submit.prevent="submit">
      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>业务单号 *</span>
          <input v-model="form.biz_no" :disabled="loading" placeholder="例如：PO-20260306-0001" />
        </label>

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
          <span>单次进价 *</span>
          <input v-model="form.unit_cost" :disabled="loading" placeholder="例如：2.20" />
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
        <button class="btn" type="submit" :disabled="loading || !canSubmit">
          {{ loading ? '提交中...' : '提交入库' }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="loading" @click="resetForm">
          重置
        </button>
      </div>

      <p v-if="errorText" class="error-text">{{ errorText }}</p>
      <p v-if="successText" class="success-text">{{ successText }}</p>
    </form>

    <div v-if="result" class="result-panel">
      <h3>最近一次入库结果</h3>
      <p>业务单号：{{ result.biz_no }}</p>
      <p>商品ID：{{ result.product_id }}</p>
      <p>当前库存：{{ result.current_stock }}</p>
      <p>当前成本价：{{ result.cost_price }}</p>
      <p>版本：{{ result.version }}</p>
    </div>
  </section>
</template>