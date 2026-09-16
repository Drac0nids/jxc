export interface ApiResponse<T> {
  code: number
  message: string
  data: T
  request_id: string
}

export interface ApiErrorPayload {
  code: number
  message: string
  data?: unknown
  request_id?: string
}

export interface StockInsufficientErrorData {
  failed_product_id?: number
  available_stock?: number
  required_qty?: number
}

export interface LoginRequest {
  username: string
  password: string
}

export interface RegisterRequest {
  tenant_name?: string
  username: string
  name: string
  password: string
}

export interface LoginUserInfo {
  id: string
  name: string
  role: 'OWNER' | 'PURCHASER' | 'SALES' | string
}

export interface LoginResponseData {
  access_token: string
  refresh_token: string
  expires_in: number
  tenant_id: string
  user_info: LoginUserInfo
}

export interface RegisterResponseData {
  registered: boolean
  tenant_name?: string | null
  access_token: string
  refresh_token: string
  expires_in: number
  tenant_id: string
  user_info: LoginUserInfo
}

export interface RefreshTokenRequest {
  refresh_token: string
}

export interface RefreshTokenResponseData {
  access_token: string
  refresh_token?: string
  expires_in: number
  tenant_id: string
  user_info: LoginUserInfo
}

export interface LogoutResponseData {
  logged_out: boolean
  user_id: string
}

export interface UserData {
  id: string
  username: string
  name: string
  role: 'OWNER' | 'PURCHASER' | 'SALES' | string
}

export interface ListUsersResponseData {
  list: UserData[]
  total: number
}

export interface CreateUserRequest {
  username: string
  name: string
  role: 'OWNER' | 'PURCHASER' | 'SALES' | string
  password: string
}

export interface UpdateUserRoleRequest {
  role: 'OWNER' | 'PURCHASER' | 'SALES' | string
}

export interface ResetUserPasswordRequest {
  new_password: string
}

export interface ResetUserPasswordResponseData {
  id: string
  reset: boolean
}

export interface ProductData {
  id: number
  sku: string
  barcode: string
  name: string
  unit: string
  current_stock: number
  retail_price: string
  last_inbound_unit_cost: string | null
  cost_price: string | null
  min_stock_limit: number
  version: number
  track_batches?: boolean
  track_serials?: boolean
}

export interface ScanProductData {
  id: number
  sku: string
  barcode: string
  name: string
  unit: string
  current_stock: number
  retail_price: string
  last_inbound_unit_cost: string | null
  cost_price: string | null
  version: number
  min_stock_limit: number
}

export interface BarcodeLookupData {
  barcode: string
  status: 'FOUND' | 'NOT_FOUND' | string
  suggested_name: string | null
  cache_hit: boolean
  source: 'CACHE' | 'THIRD_PARTY' | 'CACHE_STALE' | 'DEGRADED' | string
}

export interface PagedData<T> {
  list: T[]
  total: number
  page: number
  page_size: number
}

export interface ListProductsQuery {
  page?: number
  page_size?: number
  keyword?: string
  barcode?: string
}

export interface CreateProductRequest {
  sku?: string
  barcode: string
  name: string
  unit: string
  retail_price: string
  init_stock?: number
  min_stock_limit?: number
  cost_price: string
  track_batches?: boolean
  track_serials?: boolean
}

export interface UpdateProductRequest {
  sku?: string
  barcode?: string
  name?: string
  unit?: string
  retail_price?: string
  min_stock_limit?: number
  expected_version?: number
  track_batches?: boolean
  track_serials?: boolean
}

export interface DeleteProductQuery {
  expected_version?: number
}

export interface DeleteProductResponseData {
  id: number
  deleted: boolean
  version: number
}

export interface LowStockAlertData {
  product_id: number
  sku: string
  barcode: string
  name: string
  unit: string
  current_stock: number
  min_stock_limit: number
  shortage_qty: number
}

export interface ListLowStockAlertsQuery {
  page?: number
  page_size?: number
  keyword?: string
  only_active?: boolean
}

export interface ListLowStockAlertsResponseData extends PagedData<LowStockAlertData> {
  active_low_stock_count: number
}

export interface DashboardQuery {
  date?: string
}

export interface DashboardData {
  date: string
  total_sales: string
  total_gross_profit: string
  total_orders: number
  low_stock_count: number
  top_selling_item: string
}

export interface DashboardOrdersDrilldownQuery {
  start_date?: string
  end_date?: string
  page?: number
  page_size?: number
}

export type SalesReportGroupBy = 'product'

export type SalesReportExportFormat = 'csv' | 'xlsx'

export interface SalesReportQuery {
  start_date?: string
  end_date?: string
  group_by?: SalesReportGroupBy
  page?: number
  page_size?: number
}

export interface SalesReportItemData {
  product_id: number
  product_name: string
  total_qty: number
  total_sales: string
  total_cost: string
  gross_profit: string
}

export interface SalesReportSummaryData {
  total_sales: string
  total_cost: string
  total_gross_profit: string
  total_qty: number
}

export interface SalesReportData extends PagedData<SalesReportItemData> {
  group_by: SalesReportGroupBy | string
  start_date: string
  end_date: string
  summary: SalesReportSummaryData
}

export interface SalesReportExportQuery {
  start_date?: string
  end_date?: string
  group_by?: SalesReportGroupBy
  format?: SalesReportExportFormat
}

export interface SalesReportExportResult {
  blob: Blob
  filename: string
  content_type: string
}

export interface OutboundItemRequest {
  product_id: number
  qty: number
  sell_price: string
  expected_version?: number
}

export interface InboundRequest {
  product_id?: number
  barcode?: string
  qty: number
  unit_cost?: string
  expected_version?: number
  remark?: string
}

export interface InboundBatchItemRequest {
  product_id?: number
  barcode?: string
  qty: number
  unit_cost?: string
  expected_version?: number
  remark?: string
}

export interface InboundBatchRequest {
  items: InboundBatchItemRequest[]
}

export interface InboundResponseData {
  biz_no: string
  product_id: number
  current_stock: number
  cost_price: string
  version: number
  track_batches?: boolean
}

export interface InboundBatchItemResponseData {
  product_id: number
  current_stock: number
  cost_price: string
  version: number
  track_batches?: boolean
}

export interface InboundBatchResponseData {
  biz_no: string
  items: InboundBatchItemResponseData[]
}

export interface PurchaseOrderCreateItemRequest {
  product_id: number
  qty: number
  unit_cost: string
}

export interface PurchaseOrderCreateRequest {
  supplier_id?: number
  items: PurchaseOrderCreateItemRequest[]
  remark?: string
}

export interface OrderActionRequest {
  expected_version?: number
}

export interface StockCheckCreateItemRequest {
  product_id: number
}

export interface StockCheckCreateRequest {
  items: StockCheckCreateItemRequest[]
  remark?: string
}

export interface StockCheckConfirmItemRequest {
  product_id: number
  actual_stock: number
}

export interface StockCheckConfirmRequest {
  expected_version?: number
  items: StockCheckConfirmItemRequest[]
  remark?: string
}

export interface StockCheckItemData {
  product_id: number
  product_name: string
  book_stock: number
  actual_stock: number | null
  delta_qty: number | null
}

export interface StockCheckData {
  id: number
  biz_no: string
  status: 'DRAFT' | 'COUNTING' | 'CONFIRMED' | string
  items: StockCheckItemData[]
  remark: string | null
  version: number
  counting_at: string | null
  confirmed_at: string | null
  created_at: string
  updated_at: string
}

export interface PurchaseOrderItemData {
  product_id: number
  product_name: string
  qty: number
  unit_cost: string
  line_amount: string
}

export interface PurchaseOrderData {
  id: number
  biz_no: string
  supplier_id: number | null
  status: 'DRAFT' | 'CONFIRMED' | 'VOIDED' | string
  items: PurchaseOrderItemData[]
  total_amount: string
  remark: string | null
  version: number
  confirmed_at: string | null
  voided_at: string | null
  created_at: string
  updated_at: string
}

export interface SalesOrderCreateItemRequest {
  product_id: number
  qty: number
  sell_price: string
}

export interface SalesOrderCreateRequest {
  customer_id?: number
  items: SalesOrderCreateItemRequest[]
  remark?: string
}

export interface SalesOrderReturnItemRequest {
  product_id: number
  qty: number
}

export interface SalesOrderReturnRequest {
  expected_version?: number
  items: SalesOrderReturnItemRequest[]
  remark?: string
}

export interface SalesOrderItemData {
  product_id: number
  product_name: string
  qty: number
  sell_price: string
  line_amount: string
  returned_qty: number
}

export interface SalesOrderData {
  id: number
  biz_no: string
  customer_id: number | null
  status:
    | 'DRAFT'
    | 'CONFIRMED'
    | 'RETURNED_PARTIAL'
    | 'RETURNED_FULL'
    | 'VOIDED'
    | string
  items: SalesOrderItemData[]
  total_amount: string
  remark: string | null
  version: number
  confirmed_at: string | null
  returned_at: string | null
  voided_at: string | null
  created_at: string
  updated_at: string
}

export interface DashboardOrdersDrilldownData extends PagedData<SalesOrderData> {
  start_date: string
  end_date: string
}

export interface OutboundRequest {
  customer_id?: number
  expected_version?: number
  items: OutboundItemRequest[]
  remark?: string
}

export interface OutboundItemResult {
  product_id: number
  qty: number
  current_stock: number
  version: number
}

export interface OutboundResponseData {
  biz_no: string
  items: OutboundItemResult[]
  total_amount: string
}

export interface AuditLogData {
  id: number
  operator_id: string
  action: string
  target_type: string
  target_id: string
  before_data: unknown
  after_data: unknown
  request_id: string
  created_at: string
}

export interface AuditLogQuery {
  page?: number
  page_size?: number
  action?: string
  target_type?: string
  operator_id?: string
  request_id?: string
  start_date?: string
  end_date?: string
}

export interface StockLogData {
  id: number
  product_id: number
  biz_type: string
  biz_no: string
  delta_qty: number
  snapshot_stock: number
  snapshot_cost: string
  operator_id: string
  created_at: string
}

export interface StockLogQuery {
  page?: number
  page_size?: number
  biz_type?: string
  biz_no?: string
  product_id?: number
  operator_id?: string
  start_date?: string
  end_date?: string
}

// ── Suppliers ─────────────────────────────────────────────────────────────────

export interface SupplierData {
  id: number
  name: string
  phone: string | null
  notes: string | null
  created_at: string
  updated_at: string
}

export interface CreateSupplierRequest {
  name: string
  phone?: string
  notes?: string
}

export interface UpdateSupplierRequest {
  name?: string
  phone?: string | null
  notes?: string | null
}

// ── Categories ────────────────────────────────────────────────────────────────

export interface CategoryTreeData {
  id: number
  name: string
  level: number
  sort_order: number
  parent_id: number | null
  children: CategoryTreeData[]
}

export interface CreateCategoryRequest {
  name: string
  parent_id?: number
  sort_order?: number
}

export interface UpdateCategoryRequest {
  name?: string
  sort_order?: number
}

// ── Batches ───────────────────────────────────────────────────────────────────

export type BatchExpiryLevel = 'EXPIRED' | 'CRITICAL' | 'WARNING' | 'NOTICE' | 'OK'

export interface BatchData {
  id: number
  product_id: number
  lot_number: string
  supplier: string | null
  inbound_at: string
  produced_at: string | null
  expires_at: string | null
  notes: string | null
  is_sold_out: boolean
  sold_out_at: string | null
  days_until_expiry: number | null
  expiry_level: BatchExpiryLevel | null
  created_at: string
}

export interface ExpiringBatchData {
  batch: BatchData
  product_name: string
  product_sku: string
}

export interface CreateBatchRequest {
  product_id: number
  lot_number?: string
  supplier?: string
  inbound_at?: string
  produced_at?: string
  expires_at?: string
  notes?: string
}

export interface UpdateBatchRequest {
  lot_number?: string
  supplier?: string | null
  produced_at?: string | null
  expires_at?: string | null
  notes?: string
}
