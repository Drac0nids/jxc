<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import {
  barcodeLookupProductNameApi,
  createProductApi,
  deleteProductApi,
  listProductsApi,
  scanProductApi,
  updateProductApi,
} from '@/api/products'
import { ApiClientError } from '@/api/http'
import { useAuthStore } from '@/stores/auth'
import type { CreateProductRequest, ProductData, UpdateProductRequest } from '@/types/api'

const authStore = useAuthStore()
const router = useRouter()
const route = useRoute()

const MONEY_PATTERN = /^\d+(\.\d{1,4})?$/

const listLoading = ref(false)
const createLoading = ref(false)
const editLoading = ref(false)
const deleteLoadingId = ref<number | null>(null)
const scanLoading = ref(false)

const queryErrorText = ref('')
const scanErrorText = ref('')
const scanSuccessText = ref('')
const createErrorText = ref('')
const createSuccessText = ref('')
const editErrorText = ref('')
const editSuccessText = ref('')
const deleteErrorText = ref('')
const deleteSuccessText = ref('')

const canCreate = computed(() => {
  const role = authStore.session?.user.role
  return role === 'OWNER' || role === 'PURCHASER'
})

const canViewCostPrice = computed(() => authStore.session?.user.role !== 'SALES')

const createBusy = computed(
  () =>
    createLoading.value ||
    listLoading.value ||
    editLoading.value ||
    deleteLoadingId.value !== null,
)
const editBusy = computed(
  () =>
    editLoading.value ||
    listLoading.value ||
    createLoading.value ||
    deleteLoadingId.value !== null,
)
const rowActionBusy = computed(
  () =>
    listLoading.value ||
    createLoading.value ||
    editLoading.value ||
    deleteLoadingId.value !== null,
)

const query = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  barcode: '',
})

const scanForm = reactive({
  barcode: '',
})

const createForm = reactive({
  sku: '',
  barcode: '',
  name: '',
  unit: '',
  retail_price: '',
  init_stock: '0',
  min_stock_limit: '0',
  cost_price: '',
  track_batches: false,
  track_serials: false,
})

const editForm = reactive({
  sku: '',
  barcode: '',
  name: '',
  unit: '',
  retail_price: '',
  min_stock_limit: '0',
  expected_version: '',
  track_batches: false,
  track_serials: false,
})
const editingProductId = ref<number | null>(null)

const products = ref<ProductData[]>([])
const total = ref(0)

// ── Detail modal ─────────────────────────────────────────────────────────────
const detailProduct = ref<ProductData | null>(null)

function openDetail(p: ProductData) { detailProduct.value = p }
function closeDetail() { detailProduct.value = null }

function stockProgress(p: ProductData): number {
  if (!p.min_stock_limit || p.min_stock_limit === 0) return 1
  return Math.min(1, p.current_stock / (p.min_stock_limit * 2))
}

function stockColor(p: ProductData): string {
  const prog = stockProgress(p)
  if (prog < 0.3) return '#ff6f61'
  if (prog < 0.6) return '#f59e0b'
  return '#10b981'
}

function grossMargin(p: ProductData): string | null {
  const cost = parseFloat(p.cost_price ?? '')
  const retail = parseFloat(p.retail_price)
  if (!cost || !retail || retail <= 0) return null
  return ((retail - cost) / retail * 100).toFixed(1) + '%'
}

function firstQueryValue(value: unknown): string {
  if (typeof value === 'string') {
    return value
  }

  if (Array.isArray(value) && typeof value[0] === 'string') {
    return value[0]
  }

  return ''
}

const fromInbound = computed(() => firstQueryValue(route.query.from).trim() === 'inbound')

function formatApiError(error: unknown, fallback: string): string {
  if (error instanceof ApiClientError) {
    return `${error.message}（code=${error.code}${error.requestId ? `, request_id=${error.requestId}` : ''}）`
  }

  return error instanceof Error ? error.message : fallback
}

function formatApiClientErrorMessage(error: ApiClientError, message: string): string {
  return `${message}（code=${error.code}${error.requestId ? `, request_id=${error.requestId}` : ''}）`
}

function readErrorDataObject(error: ApiClientError): Record<string, unknown> | null {
  if (typeof error.data === 'object' && error.data !== null) {
    return error.data as Record<string, unknown>
  }

  return null
}

function formatProductWriteError(error: unknown, fallback: string): string {
  if (error instanceof ApiClientError) {
    if (error.code === 4091) {
      const data = readErrorDataObject(error)
      const currentVersion =
        data && typeof data.current_version === 'number' ? data.current_version : null
      const message =
        currentVersion === null
          ? '版本冲突，请刷新后重试'
          : `版本冲突，请刷新后重试（当前版本：${currentVersion}）`
      return formatApiClientErrorMessage(error, message)
    }

    if (error.code === 4090) {
      const data = readErrorDataObject(error)
      if (data && typeof data.current_stock === 'number') {
        return formatApiClientErrorMessage(error, `存在库存（${data.current_stock}），无法删除商品`)
      }

      return formatApiClientErrorMessage(error, error.message)
    }

    if (error.code === 4030) {
      return formatApiClientErrorMessage(error, '当前角色无权限执行该操作')
    }
  }

  return formatApiError(error, fallback)
}

async function fetchProducts(): Promise<void> {
  listLoading.value = true
  queryErrorText.value = ''

  try {
    const response = await listProductsApi({
      page: query.page,
      page_size: query.page_size,
      keyword: query.keyword.trim() || undefined,
      barcode: query.barcode.trim() || undefined,
    })
    products.value = response.data.list
    total.value = response.data.total
  } catch (error) {
    queryErrorText.value = formatApiError(error, '商品查询失败')
  } finally {
    listLoading.value = false
  }
}

function resetScanForm(): void {
  scanForm.barcode = ''
  scanErrorText.value = ''
  scanSuccessText.value = ''
}

async function scanBarcode(): Promise<void> {
  if (scanLoading.value) {
    return
  }

  const barcode = scanForm.barcode.trim()
  scanErrorText.value = ''
  scanSuccessText.value = ''

  if (!barcode) {
    scanErrorText.value = '请输入条码后再查询'
    return
  }

  scanLoading.value = true
  try {
    const response = await scanProductApi(barcode)
    scanSuccessText.value = `已找到商品：#${response.data.id} ${response.data.name}`

    query.barcode = barcode
    query.page = 1
    await fetchProducts()

    if (canCreate.value) {
      const matched = products.value.find((item) => item.id === response.data.id)
      if (matched) {
        startEditProduct(matched)
      }
    }
  } catch (error) {
    if (error instanceof ApiClientError && error.code === 4040) {
      createForm.barcode = barcode
      if (canCreate.value) {
        createForm.name = ''

        try {
          const lookupResponse = await barcodeLookupProductNameApi(barcode)
          const suggestedName = lookupResponse.data.suggested_name?.trim() ?? ''

          if (suggestedName) {
            createForm.name = suggestedName
            scanSuccessText.value = `未找到商品，已回填条码并自动带出名称（来源：${lookupResponse.data.source}），请补全其余信息后创建。`
          } else {
            scanSuccessText.value = '未找到商品，已将条码回填到创建表单，暂未获取到建议名称，请补全信息后创建。'
          }
        } catch {
          scanSuccessText.value = '未找到商品，已将条码回填到创建表单，请补全信息后创建。'
        }
      } else {
        scanErrorText.value = '未找到商品，且当前角色无建档权限（仅 OWNER/PURCHASER）。'
      }
      return
    }

    scanErrorText.value = formatApiError(error, '扫码查询失败')
  } finally {
    scanLoading.value = false
  }
}

function resetFilters(): void {
  query.keyword = ''
  query.barcode = ''
  query.page = 1
  void fetchProducts()
}

function prevPage(): void {
  if (query.page <= 1) {
    return
  }
  query.page -= 1
  void fetchProducts()
}

function nextPage(): void {
  if (query.page * query.page_size >= total.value) {
    return
  }
  query.page += 1
  void fetchProducts()
}

function resetCreateFormInternal(shouldClearMessage: boolean): void {
  createForm.sku = ''
  createForm.barcode = ''
  createForm.name = ''
  createForm.unit = ''
  createForm.retail_price = ''
  createForm.init_stock = '0'
  createForm.min_stock_limit = '0'
  createForm.cost_price = ''
  createForm.track_batches = false
  createForm.track_serials = false

  if (shouldClearMessage) {
    createErrorText.value = ''
    createSuccessText.value = ''
  }
}

function resetCreateForm(): void {
  resetCreateFormInternal(true)
}

function validateCreateForm(): string | null {
  if (!canCreate.value) {
    return '当前角色无创建商品权限，仅 OWNER/PURCHASER 可操作'
  }

  if (createForm.sku && !createForm.sku.trim()) {
    return 'SKU 不能为空'
  }

  if (!createForm.barcode.trim()) {
    return '条码不能为空'
  }

  if (!createForm.name.trim()) {
    return '商品名称不能为空'
  }

  if (!createForm.unit.trim()) {
    return '单位不能为空'
  }

  if (!MONEY_PATTERN.test(createForm.retail_price.trim())) {
    return '零售价格式错误（示例：3.50）'
  }

  const initStockRaw = createForm.init_stock.trim()
  if (initStockRaw) {
    const initStock = Number(initStockRaw)
    if (!Number.isInteger(initStock) || initStock < 0) {
      return '初始库存必须为大于等于 0 的整数'
    }
  }

  const minStockLimitRaw = createForm.min_stock_limit.trim()
  if (minStockLimitRaw) {
    const minStockLimit = Number(minStockLimitRaw)
    if (!Number.isInteger(minStockLimit) || minStockLimit < 0) {
      return '预警阈值必须为大于等于 0 的整数'
    }
  }

  const costPriceRaw = createForm.cost_price.trim()
  if (!costPriceRaw) {
    return '进货价格不能为空'
  }
  if (!MONEY_PATTERN.test(costPriceRaw)) {
    return '进货价格式错误（示例：2.10）'
  }

  return null
}

function buildCreatePayload(): CreateProductRequest {
  const payload: CreateProductRequest = {
    barcode: createForm.barcode.trim(),
    name: createForm.name.trim(),
    unit: createForm.unit.trim(),
    retail_price: createForm.retail_price.trim(),
    cost_price: createForm.cost_price.trim(),
    track_batches: createForm.track_batches,
    track_serials: createForm.track_serials,
  }

  const sku = createForm.sku.trim()
  if (sku) { payload.sku = sku }

  const initStockRaw = createForm.init_stock.trim()
  if (initStockRaw) { payload.init_stock = Number(initStockRaw) }

  const minStockLimitRaw = createForm.min_stock_limit.trim()
  if (minStockLimitRaw) { payload.min_stock_limit = Number(minStockLimitRaw) }

  return payload
}

function fillEditFormByProduct(product: ProductData): void {
  editForm.sku = product.sku
  editForm.barcode = product.barcode
  editForm.name = product.name
  editForm.unit = product.unit
  editForm.retail_price = product.retail_price
  editForm.min_stock_limit = String(product.min_stock_limit)
  editForm.expected_version = String(product.version)
  editForm.track_batches = product.track_batches ?? false
  editForm.track_serials = product.track_serials ?? false
}

function resetEditFormInternal(shouldClearMessage: boolean): void {
  editForm.sku = ''
  editForm.barcode = ''
  editForm.name = ''
  editForm.unit = ''
  editForm.retail_price = ''
  editForm.min_stock_limit = '0'
  editForm.expected_version = ''
  editForm.track_batches = false
  editForm.track_serials = false

  if (shouldClearMessage) {
    editErrorText.value = ''
    editSuccessText.value = ''
  }
}

function startEditProduct(product: ProductData): void {
  if (!canCreate.value) {
    editErrorText.value = '当前角色无编辑商品权限，仅 OWNER/PURCHASER 可操作'
    return
  }

  editingProductId.value = product.id
  fillEditFormByProduct(product)
  editErrorText.value = ''
  editSuccessText.value = ''
  deleteErrorText.value = ''
  deleteSuccessText.value = ''
}

function cancelEditProduct(): void {
  editingProductId.value = null
  resetEditFormInternal(true)
}

function resetEditFormToLatest(): void {
  if (editingProductId.value === null) {
    return
  }

  const latest = products.value.find((item) => item.id === editingProductId.value)
  if (!latest) {
    editErrorText.value = '当前页未找到正在编辑的商品，请先重新查询'
    return
  }

  fillEditFormByProduct(latest)
  editErrorText.value = ''
}

function validateEditForm(): string | null {
  if (!canCreate.value) {
    return '当前角色无编辑商品权限，仅 OWNER/PURCHASER 可操作'
  }

  if (editingProductId.value === null) {
    return '请先在列表中选择要编辑的商品'
  }

  if (!editForm.sku.trim()) {
    return 'SKU 不能为空'
  }

  if (!editForm.barcode.trim()) {
    return '条码不能为空'
  }

  if (!editForm.name.trim()) {
    return '商品名称不能为空'
  }

  if (!editForm.unit.trim()) {
    return '单位不能为空'
  }

  if (!MONEY_PATTERN.test(editForm.retail_price.trim())) {
    return '零售价格式错误（示例：3.50）'
  }

  const minStockLimitRaw = editForm.min_stock_limit.trim()
  if (!minStockLimitRaw) {
    return '预警阈值不能为空'
  }
  const minStockLimit = Number(minStockLimitRaw)
  if (!Number.isInteger(minStockLimit) || minStockLimit < 0) {
    return '预警阈值必须为大于等于 0 的整数'
  }

  const expectedVersionRaw = editForm.expected_version.trim()
  if (!expectedVersionRaw) {
    return 'expected_version 不能为空'
  }
  const expectedVersion = Number(expectedVersionRaw)
  if (!Number.isInteger(expectedVersion) || expectedVersion < 0) {
    return 'expected_version 必须为大于等于 0 的整数'
  }

  return null
}

function buildUpdatePayload(): UpdateProductRequest {
  return {
    sku: editForm.sku.trim(),
    barcode: editForm.barcode.trim(),
    name: editForm.name.trim(),
    unit: editForm.unit.trim(),
    retail_price: editForm.retail_price.trim(),
    min_stock_limit: Number(editForm.min_stock_limit.trim()),
    expected_version: Number(editForm.expected_version.trim()),
    track_batches: editForm.track_batches,
    track_serials: editForm.track_serials,
  }
}

async function submitCreateProduct(): Promise<void> {
  if (createBusy.value) {
    return
  }

  createErrorText.value = ''
  createSuccessText.value = ''

  const validationError = validateCreateForm()
  if (validationError) {
    createErrorText.value = validationError
    return
  }

  createLoading.value = true
  try {
    const response = await createProductApi(buildCreatePayload())
    resetCreateFormInternal(false)
    createSuccessText.value = `创建成功：#${response.data.id} ${response.data.name}`

    if (fromInbound.value) {
      await router.push({
        name: 'inbound',
        query: {
          created_product_id: String(response.data.id),
          created_barcode: response.data.barcode,
        },
      })
      return
    }

    query.page = 1
    await fetchProducts()
  } catch (error) {
    createErrorText.value = formatApiError(error, '创建商品失败')
  } finally {
    createLoading.value = false
  }
}

async function submitUpdateProduct(): Promise<void> {
  if (editBusy.value) {
    return
  }

  editErrorText.value = ''
  editSuccessText.value = ''

  const validationError = validateEditForm()
  if (validationError) {
    editErrorText.value = validationError
    return
  }

  const productId = editingProductId.value
  if (productId === null) {
    editErrorText.value = '请先在列表中选择要编辑的商品'
    return
  }

  editLoading.value = true
  try {
    const response = await updateProductApi(productId, buildUpdatePayload())
    fillEditFormByProduct(response.data)
    editSuccessText.value = `更新成功：#${response.data.id} ${response.data.name}`
    await fetchProducts()
  } catch (error) {
    editErrorText.value = formatProductWriteError(error, '更新商品失败')

    if (error instanceof ApiClientError && error.code === 4091) {
      await fetchProducts()
      const latest = products.value.find((item) => item.id === productId)
      if (latest) {
        fillEditFormByProduct(latest)
      } else {
        cancelEditProduct()
      }
    }
  } finally {
    editLoading.value = false
  }
}

async function submitDeleteProduct(item: ProductData): Promise<void> {
  if (rowActionBusy.value) {
    return
  }

  if (!canCreate.value) {
    deleteErrorText.value = '当前角色无删除商品权限，仅 OWNER/PURCHASER 可操作'
    return
  }

  deleteErrorText.value = ''
  deleteSuccessText.value = ''

  const confirmed = window.confirm(`确定删除商品 #${item.id} ${item.name} 吗？`)
  if (!confirmed) {
    return
  }

  deleteLoadingId.value = item.id
  try {
    const response = await deleteProductApi(item.id, {
      expected_version: item.version,
    })
    deleteSuccessText.value = `删除成功：#${response.data.id}`

    if (editingProductId.value === item.id) {
      cancelEditProduct()
    }

    if (products.value.length === 1 && query.page > 1) {
      query.page -= 1
    }

    await fetchProducts()
  } catch (error) {
    deleteErrorText.value = formatProductWriteError(error, '删除商品失败')

    if (error instanceof ApiClientError && (error.code === 4091 || error.code === 4090)) {
      await fetchProducts()
      if (editingProductId.value === item.id) {
        const latest = products.value.find((product) => product.id === item.id)
        if (latest) {
          fillEditFormByProduct(latest)
        } else {
          cancelEditProduct()
        }
      }
    }
  } finally {
    deleteLoadingId.value = null
  }
}

onMounted(() => {
  if (fromInbound.value) {
    const barcodeFromInbound = firstQueryValue(route.query.barcode).trim()
    if (barcodeFromInbound) {
      createForm.barcode = barcodeFromInbound
      scanSuccessText.value = `来自入库页的未命中条码已回填：${barcodeFromInbound}，请补全商品信息后创建。`
    }
  }

  void fetchProducts()
})
</script>

<template>
  <section>
    <h2>商品管理</h2>

    <p v-if="!canCreate" class="warn-text">当前角色无商品写操作权限，仅 OWNER/PURCHASER 可操作。</p>

    <div class="card-panel form-grid" style="margin-bottom: 12px">
      <h3>条码查询 / 快速建档</h3>

      <form class="form-inline" @submit.prevent="scanBarcode">
        <label class="form-label inline">
          <span>条码</span>
          <input
            v-model="scanForm.barcode"
            :disabled="scanLoading"
            placeholder="输入或扫码枪回车，例如：690123456789"
          />
        </label>

        <button class="btn" type="submit" :disabled="scanLoading">
          <span class="btn-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.2-3.2" /></svg>
          </span>
          {{ scanLoading ? '查询中...' : '按条码查询' }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="scanLoading" @click="resetScanForm">
          <span class="btn-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24"><path d="M4 4h16M7 4v16m10-16v16M4 20h16" /></svg>
          </span>
          清空
        </button>
      </form>

      <p class="table-summary">
        已存在商品：自动定位并可直接编辑；不存在商品：自动回填到“创建商品属性”的条码输入框。
      </p>

      <p v-if="scanErrorText" class="error-text">{{ scanErrorText }}</p>
      <p v-if="scanSuccessText" class="success-text">{{ scanSuccessText }}</p>
    </div>

    <div v-if="canCreate" class="card-panel form-grid">
      <h3>创建商品属性</h3>

      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>SKU</span>
          <input v-model="createForm.sku" :disabled="createBusy" placeholder="可选，不填则自动生成" />
        </label>

        <label class="form-label inline">
          <span>条码 *</span>
          <input v-model="createForm.barcode" :disabled="createBusy" placeholder="例如：690123456789" />
        </label>

        <label class="form-label inline">
          <span>名称 *</span>
          <input v-model="createForm.name" :disabled="createBusy" placeholder="例如：百事可乐" />
        </label>
      </div>

      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>单位 *</span>
          <input v-model="createForm.unit" :disabled="createBusy" placeholder="例如：瓶" />
        </label>

        <label class="form-label inline">
          <span>零售价 *</span>
          <input v-model="createForm.retail_price" :disabled="createBusy" placeholder="例如：3.50" />
        </label>

        <label class="form-label inline">
          <span>进货价格 *</span>
          <input v-model="createForm.cost_price" :disabled="createBusy" placeholder="例如：2.10" />
        </label>
      </div>

      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>初始库存</span>
          <input v-model="createForm.init_stock" :disabled="createBusy" placeholder="默认 0" />
        </label>

        <label class="form-label inline">
          <span>预警阈值</span>
          <input v-model="createForm.min_stock_limit" :disabled="createBusy" placeholder="默认 0" />
        </label>
      </div>

      <div class="form-inline form-inline-compact" style="align-items:center;gap:20px">
        <label style="display:flex;align-items:center;gap:8px;cursor:pointer;font-size:13px">
          <input type="checkbox" v-model="createForm.track_batches" :disabled="createBusy" />
          <span>启用批次追踪</span>
        </label>
        <label style="display:flex;align-items:center;gap:8px;cursor:pointer;font-size:13px">
          <input type="checkbox" v-model="createForm.track_serials" :disabled="createBusy" />
          <span>启用序列号追踪</span>
        </label>
      </div>

      <div class="form-actions">
        <button class="btn btn-success" type="button" :disabled="createBusy || !canCreate" @click="submitCreateProduct">
          <span class="btn-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24"><path d="M12 5v14M5 12h14" /></svg>
          </span>
          {{ createLoading ? '创建中...' : '创建商品' }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="createBusy" @click="resetCreateForm">
          <span class="btn-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24"><path d="M4 12a8 8 0 1 0 2-5.3" /><path d="M4 4v4h4" /></svg>
          </span>
          重置创建表单
        </button>
      </div>

      <p v-if="createErrorText" class="error-text">{{ createErrorText }}</p>
      <p v-if="createSuccessText" class="success-text">{{ createSuccessText }}</p>
    </div>

    <div v-if="canCreate" class="card-panel form-grid" style="margin-top: 12px">
      <h3>编辑商品属性</h3>

      <p v-if="editingProductId === null" class="table-summary">请在下方列表中点击“编辑”选择商品。</p>

      <template v-else>
        <div class="form-inline form-inline-compact">
          <label class="form-label inline">
            <span>SKU *</span>
            <input v-model="editForm.sku" :disabled="editBusy" />
          </label>

          <label class="form-label inline">
            <span>条码 *</span>
            <input v-model="editForm.barcode" :disabled="editBusy" />
          </label>

          <label class="form-label inline">
            <span>名称 *</span>
            <input v-model="editForm.name" :disabled="editBusy" />
          </label>
        </div>

        <div class="form-inline form-inline-compact">
          <label class="form-label inline">
            <span>单位 *</span>
            <input v-model="editForm.unit" :disabled="editBusy" />
          </label>

          <label class="form-label inline">
            <span>零售价 *</span>
            <input v-model="editForm.retail_price" :disabled="editBusy" />
          </label>

        </div>

        <div class="form-inline form-inline-compact">
          <label class="form-label inline">
            <span>预警阈值 *</span>
            <input v-model="editForm.min_stock_limit" :disabled="editBusy" />
          </label>

          <label class="form-label inline">
            <span>expected_version *</span>
            <input v-model="editForm.expected_version" :disabled="editBusy" />
          </label>
        </div>

        <div class="form-inline form-inline-compact" style="align-items:center;gap:20px">
          <label style="display:flex;align-items:center;gap:8px;cursor:pointer;font-size:13px">
            <input type="checkbox" v-model="editForm.track_batches" :disabled="editBusy" />
            <span>批次追踪</span>
          </label>
          <label style="display:flex;align-items:center;gap:8px;cursor:pointer;font-size:13px">
            <input type="checkbox" v-model="editForm.track_serials" :disabled="editBusy" />
            <span>序列号追踪</span>
          </label>
        </div>

        <div class="form-actions">
          <button class="btn btn-warning" type="button" :disabled="editBusy || !canCreate" @click="submitUpdateProduct">
            <span class="btn-icon" aria-hidden="true">
              <svg viewBox="0 0 24 24"><path d="M4 20h4l10-10-4-4L4 16v4z" /><path d="m12 6 4 4" /></svg>
            </span>
            {{ editLoading ? '保存中...' : '保存修改' }}
          </button>
          <button class="btn btn-secondary" type="button" :disabled="editBusy" @click="resetEditFormToLatest">
            还原当前行
          </button>
          <button class="btn btn-secondary" type="button" :disabled="editBusy" @click="cancelEditProduct">
            取消编辑
          </button>
        </div>
      </template>

      <p v-if="editErrorText" class="error-text">{{ editErrorText }}</p>
      <p v-if="editSuccessText" class="success-text">{{ editSuccessText }}</p>
    </div>

    <div class="card-panel form-grid" style="margin-top: 12px">
      <h3>商品列表查询</h3>

      <form class="form-inline" @submit.prevent="fetchProducts">
        <label class="form-label inline">
          <span>关键字</span>
          <input v-model="query.keyword" :disabled="listLoading" placeholder="名称 / SKU / 条码" />
        </label>

        <label class="form-label inline">
          <span>条码</span>
          <input v-model="query.barcode" :disabled="listLoading" placeholder="精确条码" />
        </label>

        <button class="btn" type="submit" :disabled="listLoading">查询</button>
        <button class="btn btn-secondary" type="button" :disabled="listLoading" @click="resetFilters">
          <span class="btn-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24"><path d="M4 6h16M7 12h10M10 18h4" /></svg>
          </span>
          重置
        </button>
      </form>

      <p v-if="queryErrorText" class="error-text">{{ queryErrorText }}</p>
      <p v-if="deleteErrorText" class="error-text">{{ deleteErrorText }}</p>
      <p v-if="deleteSuccessText" class="success-text">{{ deleteSuccessText }}</p>

      <p class="table-summary">共 {{ total }} 条，当前第 {{ query.page }} 页</p>

      <div class="table-wrapper">
        <table class="data-table">
          <thead>
            <tr>
              <th>ID</th>
              <th>SKU</th>
              <th>条码</th>
              <th>名称</th>
              <th>库存</th>
              <th>零售价</th>
              <th v-if="canViewCostPrice">进货价格</th>
              <th>预警阈值</th>
              <th>版本</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in products" :key="item.id">
              <td>{{ item.id }}</td>
              <td>{{ item.sku }}</td>
              <td>{{ item.barcode }}</td>
              <td>{{ item.name }}</td>
              <td>{{ item.current_stock }}</td>
              <td>{{ item.retail_price }}</td>
              <td v-if="canViewCostPrice">{{ item.cost_price ?? '-' }}</td>
              <td>{{ item.min_stock_limit }}</td>
              <td>{{ item.version }}</td>
              <td>
                <div class="form-inline form-inline-compact">
                  <button class="btn btn-secondary" type="button" @click="openDetail(item)">详情</button>
                  <template v-if="canCreate">
                    <button
                      class="btn btn-secondary"
                      type="button"
                      :disabled="rowActionBusy"
                      @click="startEditProduct(item)"
                    >
                      {{ editingProductId === item.id ? '编辑中' : '编辑' }}
                    </button>
                    <button
                      class="btn btn-danger"
                      type="button"
                      :disabled="rowActionBusy"
                      @click="submitDeleteProduct(item)"
                    >
                      {{ deleteLoadingId === item.id ? '删除中...' : '删除' }}
                    </button>
                  </template>
                </div>
              </td>
            </tr>
            <tr v-if="!listLoading && products.length === 0">
              <td :colspan="canViewCostPrice ? 10 : 9" class="empty-cell">暂无数据</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="pager">
        <button class="btn btn-secondary" :disabled="listLoading || query.page <= 1" @click="prevPage">
          上一页
        </button>
        <button
          class="btn btn-secondary"
          :disabled="listLoading || query.page * query.page_size >= total"
          @click="nextPage"
        >
          下一页
        </button>
      </div>
    </div>
  </section>

  <!-- ── 商品详情 Modal ─────────────────────────────────────────────────────── -->
  <Teleport to="body">
    <div v-if="detailProduct" class="modal-backdrop" @click.self="closeDetail">
      <div class="modal-card detail-modal">
        <!-- Header -->
        <div class="detail-modal-header">
          <div>
            <h3 style="font-size:18px;font-weight:800;margin:0">{{ detailProduct.name }}</h3>
            <p style="font-size:12px;color:#9aa5ba;margin:4px 0 0">SKU: {{ detailProduct.sku || '—' }} &nbsp;·&nbsp; ID #{{ detailProduct.id }}</p>
          </div>
          <button class="btn btn-secondary" style="padding:6px 12px" @click="closeDetail">关闭</button>
        </div>

        <!-- 库存进度 -->
        <div class="detail-section">
          <p class="detail-section-title">📦 库存状态</p>
          <div class="stock-progress-bar">
            <div class="stock-progress-fill"
              :style="{ width: (stockProgress(detailProduct) * 100).toFixed(0) + '%', background: stockColor(detailProduct) }" />
          </div>
          <div class="detail-stat-row">
            <div class="detail-stat">
              <span class="detail-stat-label">当前库存</span>
              <span class="detail-stat-value" :style="{ color: stockColor(detailProduct) }">
                {{ detailProduct.current_stock }} <small>{{ detailProduct.unit }}</small>
              </span>
            </div>
            <div class="detail-stat">
              <span class="detail-stat-label">预警阈值</span>
              <span class="detail-stat-value">{{ detailProduct.min_stock_limit }} <small>{{ detailProduct.unit }}</small></span>
            </div>
            <div v-if="detailProduct.current_stock < detailProduct.min_stock_limit" class="detail-stat">
              <span class="detail-stat-label">缺货数量</span>
              <span class="detail-stat-value" style="color:#ff6f61">
                {{ detailProduct.min_stock_limit - detailProduct.current_stock }} <small>{{ detailProduct.unit }}</small>
              </span>
            </div>
          </div>
        </div>

        <!-- 价格信息 (按角色隐藏成本) -->
        <div v-if="canViewCostPrice" class="detail-section">
          <p class="detail-section-title">💰 价格信息</p>
          <div class="detail-stat-row">
            <div class="detail-stat">
              <span class="detail-stat-label">零售价</span>
              <span class="detail-stat-value" style="color:#10b981">¥{{ detailProduct.retail_price }}</span>
            </div>
            <div class="detail-stat">
              <span class="detail-stat-label">成本价</span>
              <span class="detail-stat-value">{{ detailProduct.cost_price ? '¥' + detailProduct.cost_price : '—' }}</span>
            </div>
            <div class="detail-stat">
              <span class="detail-stat-label">毛利率</span>
              <span class="detail-stat-value" style="color:#8b5cf6">{{ grossMargin(detailProduct) ?? '—' }}</span>
            </div>
          </div>
          <p v-if="detailProduct.last_inbound_unit_cost" style="font-size:12px;color:#9aa5ba;margin-top:8px">
            最近入库单价：¥{{ detailProduct.last_inbound_unit_cost }}
          </p>
        </div>

        <!-- 基本信息 -->
        <div class="detail-section">
          <p class="detail-section-title">ℹ️ 基本信息</p>
          <table class="detail-info-table">
            <tbody>
              <tr><td>条码</td><td><code>{{ detailProduct.barcode || '—' }}</code></td></tr>
              <tr><td>SKU</td><td>{{ detailProduct.sku || '—' }}</td></tr>
              <tr><td>单位</td><td>{{ detailProduct.unit }}</td></tr>
              <tr><td>版本</td><td>{{ detailProduct.version }}</td></tr>
            </tbody>
          </table>
          <!-- 功能标记 -->
          <div style="display:flex;gap:8px;margin-top:10px;flex-wrap:wrap">
            <span class="feature-chip" :class="detailProduct.track_batches ? 'chip-on' : 'chip-off'">
              批次追踪 {{ detailProduct.track_batches ? '✓' : '✗' }}
            </span>
            <span class="feature-chip" :class="detailProduct.track_serials ? 'chip-on' : 'chip-off'">
              序列号追踪 {{ detailProduct.track_serials ? '✓' : '✗' }}
            </span>
          </div>
        </div>

        <!-- 快捷操作 -->
        <div v-if="canCreate" class="detail-section">
          <p class="detail-section-title">⚡ 快捷操作</p>
          <div class="form-inline form-inline-compact">
            <button class="btn btn-secondary" @click="startEditProduct(detailProduct); closeDetail()">编辑商品信息</button>
            <button class="btn btn-danger" @click="submitDeleteProduct(detailProduct); closeDetail()">删除商品</button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
