<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import {
  createSupplierApi,
  deleteSupplierApi,
  listSuppliersApi,
  updateSupplierApi,
} from '@/api/catalog'
import { ApiClientError } from '@/api/http'
import type { SupplierData } from '@/types/api'

const loading = ref(false)
const saving = ref(false)
const keyword = ref('')
const errorText = ref('')
const successText = ref('')
const suppliers = ref<SupplierData[]>([])
const editingId = ref<number | null>(null)

const form = reactive({ name: '', phone: '', notes: '' })
const editForm = reactive({ name: '', phone: '', notes: '' })

function fmtError(e: unknown, fallback: string) {
  if (e instanceof ApiClientError) return `${e.message}（code=${e.code}）`
  return e instanceof Error ? e.message : fallback
}

async function fetchSuppliers() {
  loading.value = true
  errorText.value = ''
  try {
    const res = await listSuppliersApi(keyword.value.trim() || undefined)
    suppliers.value = res.data
  } catch (e) {
    errorText.value = fmtError(e, '查询供应商失败')
  } finally {
    loading.value = false
  }
}

function resetForm() {
  form.name = ''
  form.phone = ''
  form.notes = ''
}

async function submitCreate() {
  if (!form.name.trim()) { errorText.value = '供应商名称不能为空'; return }
  saving.value = true; errorText.value = ''; successText.value = ''
  try {
    const res = await createSupplierApi({
      name: form.name.trim(),
      phone: form.phone.trim() || undefined,
      notes: form.notes.trim() || undefined,
    })
    successText.value = `新增成功：${res.data.name}`
    resetForm()
    await fetchSuppliers()
  } catch (e) {
    errorText.value = fmtError(e, '新增供应商失败')
  } finally {
    saving.value = false
  }
}

function startEdit(s: SupplierData) {
  editingId.value = s.id
  editForm.name = s.name
  editForm.phone = s.phone ?? ''
  editForm.notes = s.notes ?? ''
}

function cancelEdit() {
  editingId.value = null
}

async function submitEdit(s: SupplierData) {
  if (!editForm.name.trim()) { errorText.value = '供应商名称不能为空'; return }
  saving.value = true; errorText.value = ''; successText.value = ''
  try {
    await updateSupplierApi(s.id, {
      name: editForm.name.trim(),
      phone: editForm.phone.trim() || null,
      notes: editForm.notes.trim() || null,
    })
    successText.value = '更新成功'
    editingId.value = null
    await fetchSuppliers()
  } catch (e) {
    errorText.value = fmtError(e, '更新供应商失败')
  } finally {
    saving.value = false
  }
}

async function deleteSupplier(s: SupplierData) {
  if (!confirm(`确定删除供应商「${s.name}」？`)) return
  saving.value = true; errorText.value = ''; successText.value = ''
  try {
    await deleteSupplierApi(s.id)
    successText.value = `已删除：${s.name}`
    await fetchSuppliers()
  } catch (e) {
    errorText.value = fmtError(e, '删除供应商失败')
  } finally {
    saving.value = false
  }
}

onMounted(() => { void fetchSuppliers() })
</script>

<template>
  <section>
    <h2>供应商管理</h2>

    <!-- 新增表单 -->
    <div class="card-panel">
      <h3>新增供应商</h3>
      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>名称 *</span>
          <input v-model="form.name" :disabled="saving" placeholder="供应商名称" />
        </label>
        <label class="form-label inline">
          <span>电话</span>
          <input v-model="form.phone" :disabled="saving" placeholder="联系电话（选填）" />
        </label>
        <label class="form-label inline">
          <span>备注</span>
          <input v-model="form.notes" :disabled="saving" placeholder="备注（选填）" />
        </label>
      </div>
      <div class="form-actions">
        <button class="btn" :disabled="saving" @click="submitCreate">
          {{ saving ? '保存中...' : '新增' }}
        </button>
        <button class="btn btn-secondary" :disabled="saving" @click="resetForm">重置</button>
      </div>
    </div>

    <!-- 搜索 -->
    <div class="form-inline form-inline-compact" style="margin: 12px 0;">
      <input v-model="keyword" placeholder="搜索名称/电话..." style="width:220px" @keyup.enter="fetchSuppliers" />
      <button class="btn btn-secondary" :disabled="loading" @click="fetchSuppliers">
        {{ loading ? '查询中...' : '搜索' }}
      </button>
    </div>

    <p v-if="errorText" class="error-text">{{ errorText }}</p>
    <p v-if="successText" class="success-text">{{ successText }}</p>
    <p class="table-summary">共 {{ suppliers.length }} 条</p>

    <div class="table-wrapper">
      <table class="data-table">
        <thead>
          <tr>
            <th>ID</th>
            <th>名称</th>
            <th>电话</th>
            <th>备注</th>
            <th>创建时间</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="s in suppliers" :key="s.id">
            <td>{{ s.id }}</td>
            <td>
              <template v-if="editingId === s.id">
                <input v-model="editForm.name" style="width:120px" />
              </template>
              <template v-else>{{ s.name }}</template>
            </td>
            <td>
              <template v-if="editingId === s.id">
                <input v-model="editForm.phone" style="width:120px" placeholder="电话" />
              </template>
              <template v-else>{{ s.phone ?? '—' }}</template>
            </td>
            <td>
              <template v-if="editingId === s.id">
                <input v-model="editForm.notes" style="width:140px" placeholder="备注" />
              </template>
              <template v-else>{{ s.notes ?? '—' }}</template>
            </td>
            <td><small>{{ s.created_at.substring(0, 10) }}</small></td>
            <td>
              <div class="form-inline form-inline-compact">
                <template v-if="editingId === s.id">
                  <button class="btn" :disabled="saving" @click="submitEdit(s)">保存</button>
                  <button class="btn btn-secondary" @click="cancelEdit">取消</button>
                </template>
                <template v-else>
                  <button class="btn btn-secondary" :disabled="saving" @click="startEdit(s)">编辑</button>
                  <button class="btn btn-danger" :disabled="saving" @click="deleteSupplier(s)">删除</button>
                </template>
              </div>
            </td>
          </tr>
          <tr v-if="!loading && suppliers.length === 0">
            <td colspan="6" class="empty-cell">暂无供应商数据</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
