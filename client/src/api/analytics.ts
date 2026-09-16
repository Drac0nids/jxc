import { requestApi } from '@/api/http'
import type { PagedData } from '@/types/api'

// ── Trend ─────────────────────────────────────────────────────────────────────

export interface TrendDayData {
  date: string
  total_sales: string
  total_gross_profit: string
  total_orders: number
  low_stock_count: number
  top_selling_item: string
}

export interface TrendData {
  start_date: string
  end_date: string
  days: TrendDayData[]
}

export async function getTrendApi(params: { start_date: string; end_date: string }) {
  return requestApi<TrendData>({
    method: 'get',
    url: '/reports/trend',
    params,
  })
}

// ── Stock Check Logs ──────────────────────────────────────────────────────────

export interface StockCheckLogData {
  id: number
  biz_no: string
  status: string
  item_count: number
  operator_name: string
  remark: string | null
  confirmed_at: string | null
  created_at: string
}

export async function listStockCheckLogsApi(params?: {
  page?: number
  page_size?: number
  start_date?: string
  end_date?: string
}) {
  return requestApi<PagedData<StockCheckLogData>>({
    method: 'get',
    url: '/inventory/stock-check-logs',
    params,
  })
}

// ── Serials ────────────────────────────────────────────────────────────────────

export interface SerialNumber {
  id: string
  sn: string
  product_id: number
  batch_id: string | null
  status: 'IN_STOCK' | 'SOLD' | 'RETURNED' | string
  unit_cost: string | null
  sell_price: string | null
  inbound_biz_no: string | null
  outbound_biz_no: string | null
  created_at: string
  updated_at: string
}

export interface SerialInboundResult {
  biz_no: string
  count: number
}

export interface SerialOutboundResult {
  biz_no: string
  count: number
  list: SerialNumber[]
}

export interface SerialHistoryResult {
  list: SerialNumber[]
  total: number
  page: number
  page_size: number
}

export async function serialInboundApi(payload: {
  product_id: number
  batch_id?: string
  unit_cost?: string
  sns: string[]
}) {
  return requestApi<SerialInboundResult>({
    method: 'post',
    url: '/serials/inbound',
    data: payload,
  })
}

export async function serialOutboundApi(payload: {
  sell_price?: string
  sns: string[]
}) {
  return requestApi<SerialOutboundResult>({
    method: 'post',
    url: '/serials/outbound',
    data: payload,
  })
}

export async function findSerialBySnApi(sn: string) {
  return requestApi<SerialNumber | null>({
    method: 'get',
    url: '/serials',
    params: { sn },
  })
}

export async function listSerialsByProductApi(productId: number, status?: string) {
  return requestApi<SerialNumber[]>({
    method: 'get',
    url: '/serials',
    params: { product_id: productId, status },
  })
}

export async function getSerialHistoryApi(params: {
  page?: number
  page_size?: number
  product_id?: number
  status?: string
  sn?: string
  start_date?: string
  end_date?: string
}) {
  return requestApi<SerialHistoryResult>({
    method: 'get',
    url: '/serials/history',
    params,
  })
}
