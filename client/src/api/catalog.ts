import { requestApi } from '@/api/http'
import type {
  CategoryTreeData,
  CreateCategoryRequest,
  UpdateCategoryRequest,
  SupplierData,
  CreateSupplierRequest,
  UpdateSupplierRequest,
  BatchData,
  ExpiringBatchData,
  CreateBatchRequest,
  UpdateBatchRequest,
} from '@/types/api'

// ── Suppliers ─────────────────────────────────────────────────────────────────

export async function listSuppliersApi(keyword?: string) {
  return requestApi<SupplierData[]>({
    method: 'get',
    url: '/suppliers',
    params: keyword ? { keyword } : undefined,
  })
}

export async function createSupplierApi(payload: CreateSupplierRequest) {
  return requestApi<SupplierData>({
    method: 'post',
    url: '/suppliers',
    data: payload,
  })
}

export async function updateSupplierApi(id: number, payload: UpdateSupplierRequest) {
  return requestApi<SupplierData>({
    method: 'put',
    url: `/suppliers/${id}`,
    data: payload,
  })
}

export async function deleteSupplierApi(id: number) {
  return requestApi<void>({
    method: 'delete',
    url: `/suppliers/${id}`,
  })
}

// ── Categories ────────────────────────────────────────────────────────────────

export async function listCategoriesApi() {
  return requestApi<CategoryTreeData[]>({
    method: 'get',
    url: '/categories/tree',
  })
}

export async function createCategoryApi(payload: CreateCategoryRequest) {
  return requestApi<CategoryTreeData>({
    method: 'post',
    url: '/categories',
    data: payload,
  })
}

export async function updateCategoryApi(id: number, payload: UpdateCategoryRequest) {
  return requestApi<CategoryTreeData>({
    method: 'put',
    url: `/categories/${id}`,
    data: payload,
  })
}

export async function deleteCategoryApi(id: number) {
  return requestApi<void>({
    method: 'delete',
    url: `/categories/${id}`,
  })
}

// ── Batches ───────────────────────────────────────────────────────────────────

export async function listBatchesApi(params?: { product_id?: number; only_active?: boolean }) {
  return requestApi<BatchData[]>({
    method: 'get',
    url: '/batches',
    params,
  })
}

export async function listExpiringBatchesApi(withinDays = 30) {
  return requestApi<ExpiringBatchData[]>({
    method: 'get',
    url: '/batches/expiring',
    params: { within_days: withinDays },
  })
}

export async function createBatchApi(payload: CreateBatchRequest) {
  return requestApi<BatchData>({
    method: 'post',
    url: '/batches',
    data: payload,
  })
}

export async function updateBatchApi(id: number, payload: UpdateBatchRequest) {
  return requestApi<BatchData>({
    method: 'put',
    url: `/batches/${id}`,
    data: payload,
  })
}

export async function markBatchSoldOutApi(id: number) {
  return requestApi<BatchData>({
    method: 'post',
    url: `/batches/${id}/sold-out`,
  })
}

export async function deleteBatchApi(id: number) {
  return requestApi<void>({
    method: 'delete',
    url: `/batches/${id}`,
  })
}

// Per-product batch APIs (using /products/{product_id}/batches path)
export interface ProductBatchCreateRequest {
  inbound_at: string
  lot_number?: string
  produced_at?: string
  expires_at?: string
  notes?: string
}

export async function listProductBatchesApi(
  productId: number,
  params?: { only_active?: boolean },
) {
  return requestApi<{ product_id: number; batches: import('@/types/api').BatchData[] }>({
    method: 'get',
    url: `/products/${productId}/batches`,
    params,
  })
}

export async function createProductBatchApi(
  productId: number,
  payload: ProductBatchCreateRequest,
) {
  return requestApi<import('@/types/api').BatchData>({
    method: 'post',
    url: `/products/${productId}/batches`,
    data: payload,
  })
}

// ── Inbound Logs ──────────────────────────────────────────────────────────────

export async function listInboundLogsApi(params?: {
  page?: number
  page_size?: number
  keyword?: string
  start_date?: string
  end_date?: string
}) {
  return requestApi<{ list: InboundLogData[]; total: number }>({
    method: 'get',
    url: '/inventory/inbound-logs',
    params,
  })
}

export interface InboundLogData {
  id: number
  product_id: number
  product_name: string
  product_sku: string
  quantity: number
  unit_cost: string
  total_cost: string
  operator_name: string
  notes: string | null
  created_at: string
}
