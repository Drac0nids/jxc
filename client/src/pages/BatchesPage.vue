<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import {
  createBatchApi,
  deleteBatchApi,
  listBatchesApi,
  markBatchSoldOutApi,
  updateBatchApi,
} from '@/api/catalog'
import { listProductsApi } from '@/api/products'
import { ApiClientError } from '@/api/http'
import type { BatchData, ProductData } from '@/types/api'

const loading = ref(false)
const saving = ref(false)
const errorText = ref('')
const successText = ref('')
const batches = ref<BatchData[]>([])
const products = ref<ProductData[]>([])
const filterProductId = ref('')
const onlyActive = ref(false)
const editingId = ref<number | null>(null)

const form = reactive({
  product_id: '',
  lot_number: '',
  supplier: '',
  inbound_at: '',
  produced_at: '',
  expires_at: '',
  notes: '',
})
const editForm = reactive({
  lot_number: '',
  supplier: '',
  produced_at: '',
  expires_at: '',
  notes: '',
})

function fmtError(e: unknown, fb: string) {
  if (e instanceof ApiClientError) return `${e.message}（code=${e.code}）`
  return e instanceof Error ? e.message : fb
}

function expiryClass(b: BatchData) {
  const map: Record<string, string> = { EXPIRED: 'text-danger', CRITICAL: 'text-danger', WARNING: 'text-warning', NOTICE: 'text-notice' }
  return b.expiry_level ? (map[b.expiry_level] ?? '') : ''
}

async function fetchProducts() {
  try {
    const res = await listProductsApi({ page_size: 500 })
    products.value = res.data.list
  } catch (_) { /* ignore */ }
}

async function fetchBatches() {
  loading.value = true; errorText.value = ''
  try {
    const res = await listBatchesApi({
      product_id: filterProductId.value ? Number(filterProductId.value) : undefined,
      only_active: onlyActive.value || undefined,
    })
    batches.value = res.data
  } catch (e) {
    errorText.value = fmtError(e, '查询批次失败')
  } finally {
    loading.value = false
  }
}

function resetForm() {
  Object.assign(form, { product_id: '', lot_number: '', supplier: '', inbound_at: '', produced_at: '', expires_at: '', notes: '' })
}

async function submitCreate() {
  if (!form.product_id) { errorText.value = '请选择商品'; return }
  saving.value = true; errorText.value = ''; successText.value = ''
  try {
    await createBatchApi({
      product_id: Number(form.product_id),
      lot_number: form.lot_number || undefined,
      supplier: form.supplier || undefined,
      inbound_at: form.inbound_at || undefined,
      produced_at: form.produced_at || undefined,
      expires_at: form.expires_at || undefined,
      notes: form.notes || undefined,
    })
    successText.value = '批次创建成功'
    resetForm()
    await fetchBatches()
  } catch (e) {
    errorText.value = fmtError(e, '创建批次失败')
  } finally {
    saving.value = false
  }
}

function startEdit(b: BatchData) {
  editingId.value = b.id
  editForm.lot_number = b.lot_number
  editForm.supplier = b.supplier ?? ''
  editForm.produced_at = b.produced_at ? b.produced_at.substring(0, 10) : ''
  editForm.expires_at = b.expires_at ? b.expires_at.substring(0, 10) : ''
  editForm.notes = b.notes ?? ''
}

function cancelEdit() { editingId.value = null }

async function submitEdit(b: BatchData) {
  saving.value = true; errorText.value = ''; successText.value = ''
  try {
    await updateBatchApi(b.id, {
      lot_number: editForm.lot_number || undefined,
      supplier: editForm.supplier || null,
      produced_at: editForm.produced_at || null,
      expires_at: editForm.expires_at || null,
      notes: editForm.notes || undefined,
    })
    successText.value = '更新成功'
    editingId.value = null
    await fetchBatches()
  } catch (e) {
    errorText.value = fmtError(e, '更新批次失败')
  } finally {
    saving.value = false
  }
}

async function markSoldOut(b: BatchData) {
  if (!confirm(`确定标记批次「${b.lot_number}」为售罄？`)) return
  saving.value = true; errorText.value = ''; successText.value = ''
  try {
    await markBatchSoldOutApi(b.id)
    successText.value = `批次已标记为售罄`
    await fetchBatches()
  } catch (e) {
    errorText.value = fmtError(e, '标记失败')
  } finally {
    saving.value = false
  }
}

async function deleteBatch(b: BatchData) {
  if (!confirm(`确定删除批次「${b.lot_number}」？`)) return
  saving.value = true; errorText.value = ''; successText.value = ''
  try {
    await deleteBatchApi(b.id)
    successText.value = '批次已删除'
    await fetchBatches()
  } catch (e) {
    errorText.value = fmtError(e, '删除批次失败')
  } finally {
    saving.value = false
  }
}

function productName(id: number) {
  return products.value.find(p => p.id === id)?.name ?? `#${id}`
}

onMounted(async () => {
  await fetchProducts()
  await fetchBatches()
})
</script>

<template>
  <section>
    <h2>批次管理</h2>

    <!-- 新增表单 -->
    <div class="card-panel">
      <h3>新增批次</h3>
      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>商品 *</span>
          <select v-model="form.product_id" :disabled="saving" style="width:180px">
            <option value="">请选择商品</option>
            <option v-for="p in products" :key="p.id" :value="String(p.id)">{{ p.name }}（{{ p.sku }}）</option>
          </select>
        </label>
        <label class="form-label inline">
          <span>批次号</span>
          <input v-model="form.lot_number" :disabled="saving" placeholder="留空自动生成" />
        </label>
        <label class="form-label inline">
          <span>供应商</span>
          <input v-model="form.supplier" :disabled="saving" placeholder="选填" />
        </label>
        <label class="form-label inline">
          <span>入库日期</span>
          <input v-model="form.inbound_at" :disabled="saving" type="date" />
        </label>
        <label class="form-label inline">
          <span>生产日期</span>
          <input v-model="form.produced_at" :disabled="saving" type="date" />
        </label>
        <label class="form-label inline">
          <span>过期日期</span>
          <input v-model="form.expires_at" :disabled="saving" type="date" />
        </label>
        <label class="form-label inline">
          <span>备注</span>
          <input v-model="form.notes" :disabled="saving" placeholder="选填" />
        </label>
      </div>
      <div class="form-actions">
        <button class="btn" :disabled="saving" @click="submitCreate">{{ saving ? '保存中...' : '新增批次' }}</button>
        <button class="btn btn-secondary" @click="resetForm">重置</button>
      </div>
    </div>

    <!-- 筛选 -->
    <div class="form-inline form-inline-compact" style="margin: 12px 0;">
      <select v-model="filterProductId" style="width:180px">
        <option value="">全部商品</option>
        <option v-for="p in products" :key="p.id" :value="String(p.id)">{{ p.name }}</option>
      </select>
      <label class="form-label inline" style="margin:0">
        <input v-model="onlyActive" type="checkbox" />
        <span style="margin-left:4px">仅显示未售罄</span>
      </label>
      <button class="btn btn-secondary" :disabled="loading" @click="fetchBatches">
        {{ loading ? '加载中...' : '查询' }}
      </button>
    </div>

    <p v-if="errorText" class="error-text">{{ errorText }}</p>
    <p v-if="successText" class="success-text">{{ successText }}</p>
    <p class="table-summary">共 {{ batches.length }} 条</p>

    <div class="table-wrapper">
      <table class="data-table">
        <thead>
          <tr>
            <th>ID</th>
            <th>商品</th>
            <th>批次号</th>
            <th>供应商</th>
            <th>入库日期</th>
            <th>过期日期</th>
            <th>剩余天数</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="b in batches" :key="b.id" :class="{ 'row-muted': b.is_sold_out }">
            <td>{{ b.id }}</td>
            <td>{{ productName(b.product_id) }}</td>
            <td>
              <template v-if="editingId === b.id">
                <input v-model="editForm.lot_number" style="width:110px" />
              </template>
              <template v-else>{{ b.lot_number }}</template>
            </td>
            <td>
              <template v-if="editingId === b.id">
                <input v-model="editForm.supplier" style="width:100px" placeholder="供应商" />
              </template>
              <template v-else>{{ b.supplier ?? '—' }}</template>
            </td>
            <td><small>{{ b.inbound_at.substring(0, 10) }}</small></td>
            <td>
              <template v-if="editingId === b.id">
                <input v-model="editForm.expires_at" type="date" style="width:130px" />
              </template>
              <template v-else>
                <span :class="expiryClass(b)">{{ b.expires_at ? b.expires_at.substring(0, 10) : '—' }}</span>
              </template>
            </td>
            <td>
              <span v-if="b.days_until_expiry !== null" :class="expiryClass(b)">{{ b.days_until_expiry }}天</span>
              <span v-else>—</span>
            </td>
            <td>
              <span v-if="b.is_sold_out" class="badge badge-muted">售罄</span>
              <span v-else class="badge badge-ok">在售</span>
            </td>
            <td>
              <div class="form-inline form-inline-compact">
                <template v-if="editingId === b.id">
                  <button class="btn" :disabled="saving" @click="submitEdit(b)">保存</button>
                  <button class="btn btn-secondary" @click="cancelEdit">取消</button>
                </template>
                <template v-else>
                  <button class="btn btn-secondary" :disabled="saving" @click="startEdit(b)">编辑</button>
                  <button v-if="!b.is_sold_out" class="btn btn-secondary" :disabled="saving" @click="markSoldOut(b)">售罄</button>
                  <button class="btn btn-danger" :disabled="saving" @click="deleteBatch(b)">删除</button>
                </template>
              </div>
            </td>
          </tr>
          <tr v-if="!loading && batches.length === 0">
            <td colspan="9" class="empty-cell">暂无批次数据</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
