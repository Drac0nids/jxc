import axios, { type AxiosError } from 'axios'

import { ApiClientError, http, requestApi } from '@/api/http'
import type {
  ApiResponse,
  DashboardOrdersDrilldownData,
  DashboardOrdersDrilldownQuery,
  DashboardData,
  DashboardQuery,
  InboundBatchRequest,
  InboundBatchResponseData,
  InboundRequest,
  InboundResponseData,
  ListLowStockAlertsQuery,
  ListLowStockAlertsResponseData,
  OrderActionRequest,
  OutboundRequest,
  OutboundResponseData,
  PurchaseOrderCreateRequest,
  PurchaseOrderData,
  SalesReportData,
  SalesReportExportQuery,
  SalesReportExportResult,
  SalesReportQuery,
  SalesOrderCreateRequest,
  SalesOrderData,
  SalesOrderReturnRequest,
  StockCheckConfirmRequest,
  StockCheckCreateRequest,
  StockCheckData,
  AuditLogData,
  AuditLogQuery,
  StockLogData,
  StockLogQuery,
  PagedData,
} from '@/types/api'

export async function inboundApi(payload: InboundRequest) {
  return requestApi<InboundResponseData>({
    method: 'post',
    url: '/inventory/inbound',
    data: payload,
  })
}

export async function inboundBatchApi(payload: InboundBatchRequest) {
  return requestApi<InboundBatchResponseData>({
    method: 'post',
    url: '/inventory/inbound/batch',
    data: payload,
  })
}

export async function listLowStockAlertsApi(query: ListLowStockAlertsQuery) {
  return requestApi<ListLowStockAlertsResponseData>({
    method: 'get',
    url: '/inventory/alerts/low-stock',
    params: query,
  })
}

export async function getDashboardApi(query: DashboardQuery) {
  return requestApi<DashboardData>({
    method: 'get',
    url: '/reports/dashboard',
    params: query,
  })
}

export async function getDashboardOrdersDrilldownApi(query: DashboardOrdersDrilldownQuery) {
  return requestApi<DashboardOrdersDrilldownData>({
    method: 'get',
    url: '/reports/dashboard/orders',
    params: query,
  })
}

export async function getSalesReportApi(query: SalesReportQuery) {
  return requestApi<SalesReportData>({
    method: 'get',
    url: '/reports/sales',
    params: query,
  })
}

function parseFilenameFromContentDisposition(contentDisposition?: string): string | null {
  if (!contentDisposition) {
    return null
  }

  const utf8Match = contentDisposition.match(/filename\*=UTF-8''([^;]+)/i)
  if (utf8Match?.[1]) {
    try {
      return decodeURIComponent(utf8Match[1])
    } catch {
      return utf8Match[1]
    }
  }

  const plainMatch = contentDisposition.match(/filename="?([^";]+)"?/i)
  if (plainMatch?.[1]) {
    return plainMatch[1]
  }

  return null
}

export async function exportSalesReportApi(
  query: SalesReportExportQuery,
): Promise<SalesReportExportResult> {
  try {
    const response = await http.request<Blob>({
      method: 'get',
      url: '/reports/sales/export',
      params: query,
      responseType: 'blob',
    })

    const contentDisposition = response.headers['content-disposition'] as string | undefined
    const contentType =
      (response.headers['content-type'] as string | undefined) ??
      'application/octet-stream'

    const format = query.format ?? 'csv'
    const fallbackFilename = `sales_report_export.${format}`
    const filename = parseFilenameFromContentDisposition(contentDisposition) ?? fallbackFilename

    return {
      blob: response.data,
      filename,
      content_type: contentType,
    }
  } catch (error) {
    if (axios.isAxiosError(error)) {
      const axiosError = error as AxiosError<Blob>

      const requestId = axiosError.response?.headers?.['x-request-id'] as string | undefined
      const responseBlob = axiosError.response?.data
      if (responseBlob instanceof Blob) {
        let payload: Partial<ApiResponse<unknown>> | null = null
        try {
          payload = JSON.parse(await responseBlob.text()) as Partial<ApiResponse<unknown>>
        } catch {
          // ignore json parse failure and fallback to generic network error
        }

        if (payload && typeof payload.code === 'number' && typeof payload.message === 'string') {
          throw new ApiClientError({
            code: payload.code,
            message: payload.message,
            data: payload.data,
            request_id: typeof payload.request_id === 'string' ? payload.request_id : requestId,
          })
        }
      }

      throw new ApiClientError({
        code: 5000,
        message: axiosError.message || '网络请求失败',
        request_id: requestId,
      })
    }

    throw new ApiClientError({
      code: 5000,
      message: error instanceof Error ? error.message : '未知异常',
    })
  }
}

export async function outboundApi(payload: OutboundRequest) {
  return requestApi<OutboundResponseData>({
    method: 'post',
    url: '/inventory/outbound',
    data: payload,
  })
}

export async function createPurchaseOrderApi(payload: PurchaseOrderCreateRequest) {
  return requestApi<PurchaseOrderData>({
    method: 'post',
    url: '/purchase-orders',
    data: payload,
  })
}

export async function getPurchaseOrderApi(id: number) {
  return requestApi<PurchaseOrderData>({
    method: 'get',
    url: `/purchase-orders/${id}`,
  })
}

export async function confirmPurchaseOrderApi(id: number, payload: OrderActionRequest) {
  return requestApi<PurchaseOrderData>({
    method: 'post',
    url: `/purchase-orders/${id}/confirm`,
    data: payload,
  })
}

export async function voidPurchaseOrderApi(id: number, payload: OrderActionRequest) {
  return requestApi<PurchaseOrderData>({
    method: 'post',
    url: `/purchase-orders/${id}/void`,
    data: payload,
  })
}

export async function createStockCheckApi(payload: StockCheckCreateRequest) {
  return requestApi<StockCheckData>({
    method: 'post',
    url: '/inventory/stock-checks',
    data: payload,
  })
}

export async function getStockCheckApi(id: number) {
  return requestApi<StockCheckData>({
    method: 'get',
    url: `/inventory/stock-checks/${id}`,
  })
}

export async function startStockCheckApi(id: number, payload: OrderActionRequest) {
  return requestApi<StockCheckData>({
    method: 'post',
    url: `/inventory/stock-checks/${id}/start`,
    data: payload,
  })
}

export async function confirmStockCheckApi(id: number, payload: StockCheckConfirmRequest) {
  return requestApi<StockCheckData>({
    method: 'post',
    url: `/inventory/stock-checks/${id}/confirm`,
    data: payload,
  })
}

export async function createSalesOrderApi(payload: SalesOrderCreateRequest) {
  return requestApi<SalesOrderData>({
    method: 'post',
    url: '/sales-orders',
    data: payload,
  })
}

export async function getSalesOrderApi(id: number) {
  return requestApi<SalesOrderData>({
    method: 'get',
    url: `/sales-orders/${id}`,
  })
}

export async function confirmSalesOrderApi(id: number, payload: OrderActionRequest) {
  return requestApi<SalesOrderData>({
    method: 'post',
    url: `/sales-orders/${id}/confirm`,
    data: payload,
  })
}

export async function voidSalesOrderApi(id: number, payload: OrderActionRequest) {
  return requestApi<SalesOrderData>({
    method: 'post',
    url: `/sales-orders/${id}/void`,
    data: payload,
  })
}

export async function returnSalesOrderApi(id: number, payload: SalesOrderReturnRequest) {
  return requestApi<SalesOrderData>({
    method: 'post',
    url: `/sales-orders/${id}/return`,
    data: payload,
  })
}

export async function listAuditLogsApi(query: AuditLogQuery) {
  return requestApi<PagedData<AuditLogData>>({
    method: 'get',
    url: '/audit/logs',
    params: query,
  })
}

export async function listStockLogsApi(query: StockLogQuery) {
  return requestApi<PagedData<StockLogData>>({
    method: 'get',
    url: '/inventory/logs',
    params: query,
  })
}
