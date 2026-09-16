<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import {
  createCategoryApi,
  deleteCategoryApi,
  listCategoriesApi,
  updateCategoryApi,
} from '@/api/catalog'
import { ApiClientError } from '@/api/http'
import type { CategoryTreeData } from '@/types/api'

const loading = ref(false)
const saving = ref(false)
const errorText = ref('')
const successText = ref('')
const tree = ref<CategoryTreeData[]>([])
const flatList = ref<(CategoryTreeData & { indent: string })[]>([])
const editingId = ref<number | null>(null)

const form = reactive({ name: '', parent_id: '' as string | '', sort_order: '' })
const editForm = reactive({ name: '', sort_order: '' })

function fmtError(e: unknown, fallback: string) {
  if (e instanceof ApiClientError) return `${e.message}（code=${e.code}）`
  return e instanceof Error ? e.message : fallback
}

function flattenTree(nodes: CategoryTreeData[], depth = 0): (CategoryTreeData & { indent: string })[] {
  const result: (CategoryTreeData & { indent: string })[] = []
  for (const node of nodes) {
    result.push({ ...node, indent: '　'.repeat(depth) })
    if (node.children?.length) {
      result.push(...flattenTree(node.children, depth + 1))
    }
  }
  return result
}

async function fetchCategories() {
  loading.value = true
  errorText.value = ''
  try {
    const res = await listCategoriesApi()
    tree.value = res.data
    flatList.value = flattenTree(res.data)
  } catch (e) {
    errorText.value = fmtError(e, '查询分类失败')
  } finally {
    loading.value = false
  }
}

function resetForm() {
  form.name = ''
  form.parent_id = ''
  form.sort_order = ''
}

async function submitCreate() {
  if (!form.name.trim()) { errorText.value = '分类名不能为空'; return }
  saving.value = true; errorText.value = ''; successText.value = ''
  try {
    const payload: { name: string; parent_id?: number; sort_order?: number } = {
      name: form.name.trim(),
    }
    if (form.parent_id) payload.parent_id = Number(form.parent_id)
    if (form.sort_order) payload.sort_order = Number(form.sort_order)
    const res = await createCategoryApi(payload)
    successText.value = `新增成功：${res.data.name}`
    resetForm()
    await fetchCategories()
  } catch (e) {
    errorText.value = fmtError(e, '新增分类失败')
  } finally {
    saving.value = false
  }
}

function startEdit(c: CategoryTreeData) {
  editingId.value = c.id
  editForm.name = c.name
  editForm.sort_order = String(c.sort_order)
}

function cancelEdit() { editingId.value = null }

async function submitEdit(c: CategoryTreeData) {
  if (!editForm.name.trim()) { errorText.value = '分类名不能为空'; return }
  saving.value = true; errorText.value = ''; successText.value = ''
  try {
    await updateCategoryApi(c.id, {
      name: editForm.name.trim(),
      sort_order: editForm.sort_order ? Number(editForm.sort_order) : undefined,
    })
    successText.value = '更新成功'
    editingId.value = null
    await fetchCategories()
  } catch (e) {
    errorText.value = fmtError(e, '更新分类失败')
  } finally {
    saving.value = false
  }
}

async function deleteCategory(c: CategoryTreeData) {
  if (!confirm(`确定删除分类「${c.name}」？子分类也将被删除。`)) return
  saving.value = true; errorText.value = ''; successText.value = ''
  try {
    await deleteCategoryApi(c.id)
    successText.value = `已删除：${c.name}`
    await fetchCategories()
  } catch (e) {
    errorText.value = fmtError(e, '删除分类失败')
  } finally {
    saving.value = false
  }
}

onMounted(() => { void fetchCategories() })
</script>

<template>
  <section>
    <h2>商品分类管理</h2>

    <!-- 新增表单 -->
    <div class="card-panel">
      <h3>新增分类</h3>
      <div class="form-inline form-inline-compact">
        <label class="form-label inline">
          <span>名称 *</span>
          <input v-model="form.name" :disabled="saving" placeholder="分类名称" />
        </label>
        <label class="form-label inline">
          <span>上级分类</span>
          <select v-model="form.parent_id" :disabled="saving">
            <option value="">— 顶级分类 —</option>
            <option v-for="c in flatList" :key="c.id" :value="String(c.id)">
              {{ c.indent }}{{ c.name }}
            </option>
          </select>
        </label>
        <label class="form-label inline">
          <span>排序</span>
          <input v-model="form.sort_order" :disabled="saving" placeholder="数字越小越靠前" style="width:100px" />
        </label>
      </div>
      <div class="form-actions">
        <button class="btn" :disabled="saving" @click="submitCreate">
          {{ saving ? '保存中...' : '新增' }}
        </button>
        <button class="btn btn-secondary" :disabled="saving" @click="resetForm">重置</button>
        <button class="btn btn-secondary" :disabled="loading" @click="fetchCategories">刷新</button>
      </div>
    </div>

    <p v-if="errorText" class="error-text">{{ errorText }}</p>
    <p v-if="successText" class="success-text">{{ successText }}</p>

    <div class="table-wrapper">
      <table class="data-table">
        <thead>
          <tr>
            <th>ID</th>
            <th>名称</th>
            <th>层级</th>
            <th>排序</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="c in flatList" :key="c.id">
            <td>{{ c.id }}</td>
            <td>
              <template v-if="editingId === c.id">
                <input v-model="editForm.name" style="width:140px" />
              </template>
              <template v-else>{{ c.indent }}{{ c.name }}</template>
            </td>
            <td>{{ c.level }}</td>
            <td>
              <template v-if="editingId === c.id">
                <input v-model="editForm.sort_order" style="width:60px" />
              </template>
              <template v-else>{{ c.sort_order }}</template>
            </td>
            <td>
              <div class="form-inline form-inline-compact">
                <template v-if="editingId === c.id">
                  <button class="btn" :disabled="saving" @click="submitEdit(c)">保存</button>
                  <button class="btn btn-secondary" @click="cancelEdit">取消</button>
                </template>
                <template v-else>
                  <button class="btn btn-secondary" :disabled="saving" @click="startEdit(c)">编辑</button>
                  <button class="btn btn-danger" :disabled="saving" @click="deleteCategory(c)">删除</button>
                </template>
              </div>
            </td>
          </tr>
          <tr v-if="!loading && flatList.length === 0">
            <td colspan="5" class="empty-cell">暂无分类数据</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
