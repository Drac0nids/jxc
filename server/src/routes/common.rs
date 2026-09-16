use axum::{
    Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::{FixedOffset, NaiveDate, Utc};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, Workbook};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{AuditLog, Product, PurchaseOrder, SalesOrder, StockCheck, User, UserRole},
    response::build_response_headers,
    state::AppState,
};

pub const X_IDEMPOTENCY_KEY_HEADER: &str = "x-idempotency-key";

// --- Users ---

#[derive(Debug, Serialize)]
pub struct UserData {
    pub id: String,
    pub username: String,
    pub name: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub name: String,
    pub role: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRoleRequest {
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetUserPasswordRequest {
    pub new_password: String,
}

// --- Products ---

#[derive(Debug, Deserialize)]
pub struct ScanQuery {
    pub barcode: String,
}

#[derive(Debug, Deserialize)]
pub struct BarcodeLookupQuery {
    pub barcode: String,
}

#[derive(Debug, Serialize)]
pub struct BarcodeLookupData {
    pub barcode: String,
    pub status: String,
    pub suggested_name: Option<String>,
    pub cache_hit: bool,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct ScanProductData {
    pub id: i64,
    pub sku: String,
    pub barcode: String,
    pub name: String,
    pub unit: String,
    pub current_stock: i32,
    pub retail_price: String,
    pub cost_price: Option<String>,
    pub min_stock_limit: i32,
}

#[derive(Debug, Deserialize)]
pub struct ListProductsQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub keyword: Option<String>,
    pub barcode: Option<String>,
    pub low_stock: Option<String>,
    pub category_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ProductData {
    pub id: i64,
    pub sku: String,
    pub barcode: String,
    pub name: String,
    pub unit: String,
    pub current_stock: i32,
    pub retail_price: String,
    pub last_inbound_unit_cost: Option<String>,
    pub cost_price: Option<String>,
    pub min_stock_limit: i32,
    pub category_id: Option<i64>,
    pub track_batches: bool,
    pub track_serials: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductRequest {
    pub sku: Option<String>,
    pub barcode: String,
    pub name: String,
    pub unit: String,
    pub retail_price: String,
    pub init_stock: Option<i32>,
    pub min_stock_limit: Option<i32>,
    pub cost_price: String,
    pub category_id: Option<i64>,
    pub track_batches: Option<bool>,
    pub track_serials: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProductRequest {
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub name: Option<String>,
    pub unit: Option<String>,
    pub retail_price: Option<String>,
    pub min_stock_limit: Option<i32>,
    pub expected_version: Option<i32>,
    pub category_id: Option<i64>,
    pub track_batches: Option<bool>,
    pub track_serials: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteProductQuery {
    pub expected_version: Option<i32>,
}

// --- Inventory ---

#[derive(Debug, Deserialize)]
pub struct ListLowStockAlertsQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub keyword: Option<String>,
    pub only_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LowStockAlertData {
    pub product_id: i64,
    pub sku: String,
    pub barcode: String,
    pub name: String,
    pub unit: String,
    pub current_stock: i32,
    pub min_stock_limit: i32,
    pub shortage_qty: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundRequest {
    pub product_id: Option<i64>,
    pub barcode: Option<String>,
    pub qty: i32,
    pub unit_cost: Option<String>,
    pub expected_version: Option<i32>,
    pub biz_no: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundBatchItemRequest {
    pub product_id: Option<i64>,
    pub barcode: Option<String>,
    pub qty: i32,
    pub unit_cost: Option<String>,
    pub expected_version: Option<i32>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundBatchRequest {
    pub items: Vec<InboundBatchItemRequest>,
}

#[derive(Debug, Serialize)]
pub struct InventoryResultData {
    pub biz_no: String,
    pub product_id: i64,
    pub current_stock: i32,
    pub cost_price: String,
    pub track_batches: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct InventoryBatchItemResultData {
    pub product_id: i64,
    pub current_stock: i32,
    pub cost_price: String,
    pub track_batches: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct InventoryBatchResultData {
    pub biz_no: String,
    pub items: Vec<InventoryBatchItemResultData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundRequest {
    pub biz_no: Option<String>,
    pub customer_id: Option<i64>,
    pub expected_version: Option<i32>,
    pub items: Vec<OutboundItemRequest>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundItemRequest {
    pub product_id: i64,
    pub qty: i32,
    pub sell_price: String,
    pub expected_version: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct OutboundItemResult {
    pub product_id: i64,
    pub qty: i32,
    pub current_stock: i32,
}

// --- Orders Common ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderActionRequest {
    pub expected_version: Option<i32>,
}

// --- Purchase Orders ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderCreateItemRequest {
    pub product_id: i64,
    pub qty: i32,
    pub unit_cost: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderCreateRequest {
    pub biz_no: Option<String>,
    pub supplier_id: Option<i64>,
    pub items: Vec<PurchaseOrderCreateItemRequest>,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PurchaseOrderItemData {
    pub product_id: i64,
    pub product_name: String,
    pub qty: i32,
    pub unit_cost: String,
    pub line_amount: String,
}

#[derive(Debug, Serialize)]
pub struct PurchaseOrderData {
    pub id: i64,
    pub biz_no: String,
    pub supplier_id: Option<i64>,
    pub status: String,
    pub items: Vec<PurchaseOrderItemData>,
    pub total_amount: String,
    pub remark: Option<String>,
    pub confirmed_at: Option<String>,
    pub voided_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// --- Sales Orders ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrderCreateItemRequest {
    pub product_id: i64,
    pub qty: i32,
    pub sell_price: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrderCreateRequest {
    pub biz_no: Option<String>,
    pub customer_id: Option<i64>,
    pub items: Vec<SalesOrderCreateItemRequest>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrderReturnItemRequest {
    pub product_id: i64,
    pub qty: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrderReturnRequest {
    pub expected_version: Option<i32>,
    pub items: Vec<SalesOrderReturnItemRequest>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SalesOrderItemData {
    pub product_id: i64,
    pub product_name: String,
    pub qty: i32,
    pub sell_price: String,
    pub line_amount: String,
    pub returned_qty: i32,
    /// 该明细行出库的 SN 码列表（仅 track_serials=true 的商品才有值）
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sns: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SalesOrderData {
    pub id: i64,
    pub biz_no: String,
    pub customer_id: Option<i64>,
    pub status: String,
    pub items: Vec<SalesOrderItemData>,
    pub total_amount: String,
    pub remark: Option<String>,
    pub confirmed_at: Option<String>,
    pub returned_at: Option<String>,
    pub voided_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// --- Stock Checks ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockCheckCreateItemRequest {
    pub product_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockCheckCreateRequest {
    pub biz_no: Option<String>,
    pub items: Vec<StockCheckCreateItemRequest>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockCheckConfirmItemRequest {
    pub product_id: i64,
    pub actual_stock: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockCheckConfirmRequest {
    pub expected_version: Option<i32>,
    pub items: Vec<StockCheckConfirmItemRequest>,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StockCheckItemData {
    pub product_id: i64,
    pub product_name: String,
    pub book_stock: i32,
    pub actual_stock: Option<i32>,
    pub delta_qty: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct StockCheckData {
    pub id: i64,
    pub biz_no: String,
    pub status: String,
    pub items: Vec<StockCheckItemData>,
    pub remark: Option<String>,
    pub counting_at: Option<String>,
    pub confirmed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// --- Reports ---

#[derive(Debug, Deserialize)]
pub struct DashboardQuery {
    pub date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DashboardOrdersQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct SalesReportQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub group_by: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct SalesReportExportQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub group_by: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TrendQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SalesReportProductRow {
    pub product_id: i64,
    pub product_name: String,
    pub total_qty: i32,
    pub total_sales: Decimal,
    pub total_cost: Decimal,
}

#[derive(Debug, Serialize)]
pub struct SalesReportProductData {
    pub product_id: i64,
    pub product_name: String,
    pub total_qty: i32,
    pub total_sales: String,
    pub total_cost: String,
    pub gross_profit: String,
}

// --- Audit & Logs ---

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub action: Option<String>,
    pub target_type: Option<String>,
    pub operator_id: Option<String>,
    pub request_id: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StockLogQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub biz_type: Option<String>,
    pub biz_no: Option<String>,
    pub product_id: Option<i64>,
    pub operator_id: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditLogData {
    pub id: i64,
    pub operator_id: String,
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub before_data: Value,
    pub after_data: Value,
    pub request_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StockLogData {
    pub id: i64,
    pub product_id: i64,
    pub product_name: Option<String>,
    pub biz_type: String,
    pub biz_no: String,
    pub delta_qty: i32,
    pub snapshot_stock: i32,
    pub snapshot_cost: String,
    pub snapshot_sell_price: Option<String>,
    pub snapshot_inbound_unit_cost: Option<String>,
    pub operator_id: String,
    pub operator_name: Option<String>,
    pub created_at: String,
}

// --- Helpers ---

pub fn postgres_pool_or_none(state: &AppState) -> Option<&sqlx::PgPool> {
    state.persistence.postgres.as_ref()
}

pub async fn load_product_name_map(
    state: &AppState,
    tenant_id: Uuid,
    request_id: &str,
) -> Result<HashMap<i64, String>, AppError> {
    if state.repository.is_postgres() {
        let products = state
            .repository
            .list_products_by_tenant_all(&state.persistence, tenant_id)
            .await
            .map_err(|err| err.with_request_id(request_id.to_string()))?;

        return Ok(products.into_iter().map(|p| (p.id, p.name)).collect());
    }

    let products = state
        .products
        .lock()
        .map_err(|_| AppError::internal("商品状态锁异常").with_request_id(request_id.to_string()))?;

    Ok(products
        .values()
        .filter(|product| product.tenant_id == tenant_id)
        .map(|product| (product.id, product.name.clone()))
        .collect())
}

pub fn to_purchase_order_data(
    order: &PurchaseOrder,
    product_name_map: &HashMap<i64, String>,
) -> PurchaseOrderData {
    let mut total_amount = Decimal::ZERO;
    let mut items = Vec::with_capacity(order.items.len());

    for item in &order.items {
        let rounded_unit_cost = item.unit_cost.round_dp(4);
        let line_amount = (rounded_unit_cost * Decimal::from(item.qty)).round_dp(4);
        total_amount += line_amount;

        items.push(PurchaseOrderItemData {
            product_id: item.product_id,
            product_name: item
                .product_name_snapshot
                .clone()
                .or_else(|| product_name_map.get(&item.product_id).cloned())
                .unwrap_or_else(|| format!("商品#{}", item.product_id)),
            qty: item.qty,
            unit_cost: rounded_unit_cost.to_string(),
            line_amount: line_amount.to_string(),
        });
    }

    PurchaseOrderData {
        id: order.id,
        biz_no: order.biz_no.clone(),
        supplier_id: order.supplier_id,
        status: order.status.as_str().to_string(),
        items,
        total_amount: total_amount.round_dp(4).to_string(),
        remark: order.remark.clone(),
        confirmed_at: order.confirmed_at.clone(),
        voided_at: order.voided_at.clone(),
        created_at: order.created_at.clone(),
        updated_at: order.updated_at.clone(),
    }
}

#[allow(dead_code)]
pub fn purchase_order_snapshot(order: &PurchaseOrder) -> Value {
    json!({
        "id": order.id,
        "biz_no": order.biz_no,
        "status": order.status.as_str(),
        "updated_at": order.updated_at
    })
}

pub async fn to_purchase_order_data_with_product_names(
    state: &AppState,
    tenant_id: Uuid,
    request_id: &str,
    order: &PurchaseOrder,
) -> Result<PurchaseOrderData, AppError> {
    let product_name_map = load_product_name_map(state, tenant_id, request_id).await?;
    Ok(to_purchase_order_data(order, &product_name_map))
}

pub fn to_sales_order_data(
    order: &SalesOrder,
    product_name_map: &HashMap<i64, String>,
) -> SalesOrderData {
    let mut total_amount = Decimal::ZERO;
    let mut items = Vec::with_capacity(order.items.len());

    for item in &order.items {
        let rounded_sell_price = item.sell_price.round_dp(4);
        let line_amount = (rounded_sell_price * Decimal::from(item.qty)).round_dp(4);
        total_amount += line_amount;

        items.push(SalesOrderItemData {
            product_id: item.product_id,
            product_name: item
                .product_name_snapshot
                .clone()
                .or_else(|| product_name_map.get(&item.product_id).cloned())
                .unwrap_or_else(|| format!("商品#{}", item.product_id)),
            qty: item.qty,
            sell_price: rounded_sell_price.to_string(),
            line_amount: line_amount.to_string(),
            returned_qty: item.returned_qty,
            sns: Vec::new(),
        });
    }

    SalesOrderData {
        id: order.id,
        biz_no: order.biz_no.clone(),
        customer_id: order.customer_id,
        status: order.status.as_str().to_string(),
        items,
        total_amount: total_amount.round_dp(4).to_string(),
        remark: order.remark.clone(),
        confirmed_at: order.confirmed_at.clone(),
        returned_at: order.returned_at.clone(),
        voided_at: order.voided_at.clone(),
        created_at: order.created_at.clone(),
        updated_at: order.updated_at.clone(),
    }
}

pub async fn to_sales_order_data_with_product_names(
    state: &AppState,
    tenant_id: Uuid,
    request_id: &str,
    order: &SalesOrder,
) -> Result<SalesOrderData, AppError> {
    let product_name_map = load_product_name_map(state, tenant_id, request_id).await?;
    Ok(to_sales_order_data(order, &product_name_map))
}

#[allow(dead_code)]
pub fn sales_order_snapshot(order: &SalesOrder) -> Value {
    json!({
        "id": order.id,
        "biz_no": order.biz_no,
        "status": order.status.as_str(),
        "updated_at": order.updated_at
    })
}

pub fn to_stock_check_data(
    check: &StockCheck,
    product_name_map: &HashMap<i64, String>,
) -> StockCheckData {
    StockCheckData {
        id: check.id,
        biz_no: check.biz_no.clone(),
        status: check.status.as_str().to_string(),
        items: check
            .items
            .iter()
            .map(|i| StockCheckItemData {
                product_id: i.product_id,
                product_name: product_name_map
                    .get(&i.product_id)
                    .cloned()
                    .unwrap_or_else(|| format!("商品#{}", i.product_id)),
                book_stock: i.book_stock,
                actual_stock: i.actual_stock,
                delta_qty: i.delta_qty,
            })
            .collect(),
        remark: check.remark.clone(),
        counting_at: check.counting_at.clone(),
        confirmed_at: check.confirmed_at.clone(),
        created_at: check.created_at.clone(),
        updated_at: check.updated_at.clone(),
    }
}

pub async fn to_stock_check_data_with_names(
    state: &AppState,
    tenant_id: Uuid,
    request_id: &str,
    check: &StockCheck,
) -> Result<StockCheckData, AppError> {
    let product_name_map = load_product_name_map(state, tenant_id, request_id).await?;
    Ok(to_stock_check_data(check, &product_name_map))
}

#[allow(dead_code)]
pub fn stock_check_snapshot(check: &StockCheck) -> Value {
    json!({
        "id": check.id,
        "biz_no": check.biz_no,
        "status": check.status.as_str(),
        "updated_at": check.updated_at
    })
}

pub fn to_user_data(user: &User) -> UserData {
    UserData {
        id: user.id.to_string(),
        username: user.username.clone(),
        name: user.name.clone(),
        role: user.role.as_str().to_string(),
    }
}

pub fn parse_user_role_input(raw: &str, request_id: &str) -> Result<UserRole, AppError> {
    match raw.trim().to_uppercase().as_str() {
        "OWNER" => Ok(UserRole::Owner),
        "ADMIN" => Ok(UserRole::Admin),
        "PURCHASER" => Ok(UserRole::Purchaser),
        "SALES" => Ok(UserRole::Sales),
        _ => Err(AppError::bad_request(format!("无效的角色类型: {raw}"))
            .with_request_id(request_id.to_string())),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn append_audit_log(
    state: &AppState,
    tenant_id: Uuid,
    operator_id: Uuid,
    action: &str,
    target_type: &str,
    target_id: String,
    before_data: Value,
    after_data: Value,
    request_id: &str,
) -> Result<(), AppError> {
    let mut audit_logs = state.audit_logs.lock().map_err(|_| {
        AppError::internal("审计日志锁异常").with_request_id(request_id.to_string())
    })?;
    let log_id = audit_logs.len() as i64 + 1;
    audit_logs.push(AuditLog::now(
        log_id,
        tenant_id,
        operator_id,
        action,
        target_type,
        target_id,
        before_data,
        after_data,
        request_id,
    ));
    Ok(())
}

pub fn to_product_data(product: &Product, hide_cost_price: bool) -> ProductData {
    ProductData {
        id: product.id,
        sku: product.sku.clone(),
        barcode: product.barcode.clone(),
        name: product.name.clone(),
        unit: product.unit.clone(),
        current_stock: product.current_stock,
        retail_price: product.retail_price.round_dp(4).to_string(),
        last_inbound_unit_cost: product
            .last_inbound_unit_cost
            .map(|v| v.round_dp(4).to_string()),
        cost_price: if hide_cost_price {
            None
        } else {
            Some(product.cost_price.round_dp(4).to_string())
        },
        min_stock_limit: product.min_stock_limit,
        category_id: product.category_id,
        track_batches: product.track_batches,
        track_serials: product.track_serials,
    }
}

#[allow(dead_code)]
pub fn product_snapshot(product: &Product) -> Value {
    json!({
        "id": product.id,
        "current_stock": product.current_stock,
        "cost_price": product.cost_price.round_dp(4).to_string(),
    })
}

pub fn barcode_scope_key(tenant_id: &Uuid, barcode: &str) -> String {
    format!("{}:{}", tenant_id, barcode.trim())
}

pub fn generate_server_biz_no(prefix: &str) -> String {
    let normalized_prefix = {
        let trimmed = prefix.trim();
        if trimmed.is_empty() {
            "BIZ".to_string()
        } else {
            trimmed.to_ascii_uppercase()
        }
    };

    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let random = Uuid::new_v4().simple().to_string().to_ascii_uppercase();

    format!("{}-{}-{}", normalized_prefix, timestamp, &random[..8])
}

pub fn parse_required_text(
    raw: &str,
    field_name: &str,
    request_id: &str,
) -> Result<String, AppError> {
    let v = raw.trim();
    if v.is_empty() {
        return Err(AppError::bad_request(format!("{field_name} 不能为空"))
            .with_request_id(request_id.to_string()));
    }
    Ok(v.to_string())
}

pub fn parse_decimal(raw: &str, field_name: &str, request_id: &str) -> Result<Decimal, AppError> {
    Decimal::from_str(raw.trim()).map_err(|_| {
        AppError::bad_request(format!("{field_name} 格式错误，必须是数字字符串"))
            .with_request_id(request_id.to_string())
    })
}

pub fn resolve_report_date(
    date: Option<&str>,
    timezone: &FixedOffset,
    request_id: &str,
) -> Result<NaiveDate, AppError> {
    match date.map(str::trim).filter(|v| !v.is_empty()) {
        Some(raw_date) => NaiveDate::parse_from_str(raw_date, "%Y-%m-%d").map_err(|_| {
            AppError::bad_request("date 格式错误，必须为 YYYY-MM-DD")
                .with_request_id(request_id.to_string())
        }),
        None => Ok(Utc::now().with_timezone(timezone).date_naive()),
    }
}

pub fn parse_report_date(
    raw: &str,
    field_name: &str,
    request_id: &str,
) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d").map_err(|_| {
        AppError::bad_request(format!("{field_name} 格式错误，必须为 YYYY-MM-DD"))
            .with_request_id(request_id.to_string())
    })
}

pub fn escape_csv_field(value: &str) -> String {
    let need_quote =
        value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r');
    if !need_quote {
        return value.to_string();
    }

    let escaped = value.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

pub fn decimal_to_xlsx_number(
    value: Decimal,
    field_name: &str,
    request_id: &str,
) -> Result<f64, AppError> {
    value.round_dp(2).to_f64().ok_or_else(|| {
        AppError::internal(format!("xlsx 数值转换失败: {field_name}"))
            .with_request_id(request_id.to_string())
    })
}

pub fn build_sales_report_xlsx_content(
    rows: &[SalesReportProductRow],
    summary_total_qty: i64,
    summary_total_sales: Decimal,
    summary_total_cost: Decimal,
    summary_total_gross_profit: Decimal,
    request_id: &str,
) -> Result<Vec<u8>, AppError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let header_format = Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_background_color("#D9E1F2")
        .set_border(FormatBorder::Thin);
    let text_center_format = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);
    let text_left_format = Format::new()
        .set_align(FormatAlign::Left)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);
    let qty_format = Format::new()
        .set_num_format("#,##0")
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);
    let money_format = Format::new()
        .set_num_format("#,##0.00")
        .set_align(FormatAlign::Right)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);
    let summary_label_format = Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_background_color("#FCE4D6")
        .set_border(FormatBorder::Thin);
    let summary_text_format = Format::new()
        .set_bold()
        .set_align(FormatAlign::Left)
        .set_align(FormatAlign::VerticalCenter)
        .set_background_color("#FCE4D6")
        .set_border(FormatBorder::Thin);
    let summary_qty_format = Format::new()
        .set_bold()
        .set_num_format("#,##0")
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_background_color("#FCE4D6")
        .set_border(FormatBorder::Thin);
    let summary_money_format = Format::new()
        .set_bold()
        .set_num_format("#,##0.00")
        .set_align(FormatAlign::Right)
        .set_align(FormatAlign::VerticalCenter)
        .set_background_color("#FCE4D6")
        .set_border(FormatBorder::Thin);

    worksheet.set_column_width(0, 14).map_err(|_| {
        AppError::internal("xlsx 列宽设置失败").with_request_id(request_id.to_string())
    })?;
    worksheet.set_column_width(1, 32).map_err(|_| {
        AppError::internal("xlsx 列宽设置失败").with_request_id(request_id.to_string())
    })?;
    worksheet.set_column_width(2, 12).map_err(|_| {
        AppError::internal("xlsx 列宽设置失败").with_request_id(request_id.to_string())
    })?;
    worksheet.set_column_width(3, 16).map_err(|_| {
        AppError::internal("xlsx 列宽设置失败").with_request_id(request_id.to_string())
    })?;
    worksheet.set_column_width(4, 16).map_err(|_| {
        AppError::internal("xlsx 列宽设置失败").with_request_id(request_id.to_string())
    })?;
    worksheet.set_column_width(5, 16).map_err(|_| {
        AppError::internal("xlsx 列宽设置失败").with_request_id(request_id.to_string())
    })?;
    worksheet.set_freeze_panes(1, 0).map_err(|_| {
        AppError::internal("xlsx 冻结窗格设置失败").with_request_id(request_id.to_string())
    })?;

    if !rows.is_empty() {
        worksheet
            .autofilter(0, 0, rows.len() as u32, 5)
            .map_err(|_| {
                AppError::internal("xlsx 自动筛选设置失败").with_request_id(request_id.to_string())
            })?;
    }

    for (col, header) in [
        "product_id",
        "product_name",
        "total_qty",
        "total_sales",
        "total_cost",
        "gross_profit",
    ]
    .iter()
    .enumerate()
    {
        worksheet
            .write_string_with_format(0, col as u16, *header, &header_format)
            .map_err(|_| {
                AppError::internal("xlsx 表头写入失败").with_request_id(request_id.to_string())
            })?;
    }

    for (idx, row) in rows.iter().enumerate() {
        let line_no = (idx + 1) as u32;
        let gross_profit = row.total_sales - row.total_cost;

        worksheet
            .write_string_with_format(line_no, 0, row.product_id.to_string(), &text_center_format)
            .map_err(|_| {
                AppError::internal("xlsx 明细写入失败").with_request_id(request_id.to_string())
            })?;
        worksheet
            .write_string_with_format(line_no, 1, row.product_name.clone(), &text_left_format)
            .map_err(|_| {
                AppError::internal("xlsx 明细写入失败").with_request_id(request_id.to_string())
            })?;
        worksheet
            .write_number_with_format(line_no, 2, row.total_qty as f64, &qty_format)
            .map_err(|_| {
                AppError::internal("xlsx 明细写入失败").with_request_id(request_id.to_string())
            })?;
        worksheet
            .write_number_with_format(
                line_no,
                3,
                decimal_to_xlsx_number(row.total_sales, "total_sales", request_id)?,
                &money_format,
            )
            .map_err(|_| {
                AppError::internal("xlsx 明细写入失败").with_request_id(request_id.to_string())
            })?;
        worksheet
            .write_number_with_format(
                line_no,
                4,
                decimal_to_xlsx_number(row.total_cost, "total_cost", request_id)?,
                &money_format,
            )
            .map_err(|_| {
                AppError::internal("xlsx 明细写入失败").with_request_id(request_id.to_string())
            })?;
        worksheet
            .write_number_with_format(
                line_no,
                5,
                decimal_to_xlsx_number(gross_profit, "gross_profit", request_id)?,
                &money_format,
            )
            .map_err(|_| {
                AppError::internal("xlsx 明细写入失败").with_request_id(request_id.to_string())
            })?;
    }

    let summary_line = (rows.len() + 1) as u32;
    worksheet
        .write_string_with_format(summary_line, 0, "SUMMARY", &summary_label_format)
        .map_err(|_| {
            AppError::internal("xlsx 汇总写入失败").with_request_id(request_id.to_string())
        })?;
    worksheet
        .write_string_with_format(summary_line, 1, "", &summary_text_format)
        .map_err(|_| {
            AppError::internal("xlsx 汇总写入失败").with_request_id(request_id.to_string())
        })?;
    worksheet
        .write_number_with_format(
            summary_line,
            2,
            summary_total_qty as f64,
            &summary_qty_format,
        )
        .map_err(|_| {
            AppError::internal("xlsx 汇总写入失败").with_request_id(request_id.to_string())
        })?;
    worksheet
        .write_number_with_format(
            summary_line,
            3,
            decimal_to_xlsx_number(summary_total_sales, "summary_total_sales", request_id)?,
            &summary_money_format,
        )
        .map_err(|_| {
            AppError::internal("xlsx 汇总写入失败").with_request_id(request_id.to_string())
        })?;
    worksheet
        .write_number_with_format(
            summary_line,
            4,
            decimal_to_xlsx_number(summary_total_cost, "summary_total_cost", request_id)?,
            &summary_money_format,
        )
        .map_err(|_| {
            AppError::internal("xlsx 汇总写入失败").with_request_id(request_id.to_string())
        })?;
    worksheet
        .write_number_with_format(
            summary_line,
            5,
            decimal_to_xlsx_number(
                summary_total_gross_profit,
                "summary_total_gross_profit",
                request_id,
            )?,
            &summary_money_format,
        )
        .map_err(|_| {
            AppError::internal("xlsx 汇总写入失败").with_request_id(request_id.to_string())
        })?;

    workbook
        .save_to_buffer()
        .map_err(|_| AppError::internal("xlsx 生成失败").with_request_id(request_id.to_string()))
}

pub fn is_report_date_in_range(
    rfc3339: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
    timezone: &FixedOffset,
    request_id: &str,
) -> Result<bool, AppError> {
    let dt = chrono::DateTime::parse_from_rfc3339(rfc3339)
        .map_err(|_| AppError::internal("时间格式异常").with_request_id(request_id.to_string()))?;
    let report_date = dt.with_timezone(timezone).date_naive();
    Ok(report_date >= start_date && report_date <= end_date)
}

pub fn is_same_report_date(
    rfc3339: &str,
    report_date: NaiveDate,
    timezone: &FixedOffset,
    request_id: &str,
) -> Result<bool, AppError> {
    let dt = chrono::DateTime::parse_from_rfc3339(rfc3339)
        .map_err(|_| AppError::internal("时间格式异常").with_request_id(request_id.to_string()))?;
    Ok(dt.with_timezone(timezone).date_naive() == report_date)
}

pub fn ensure_role(role: &str, allowed_roles: &[&str], request_id: &str) -> Result<(), AppError> {
    if allowed_roles
        .iter()
        .any(|allowed| role.eq_ignore_ascii_case(allowed))
    {
        Ok(())
    } else {
        Err(AppError::forbidden("角色无权限执行该操作").with_request_id(request_id.to_string()))
    }
}

pub fn require_idempotency_key(headers: &HeaderMap, request_id: &str) -> Result<String, AppError> {
    let key = headers
        .get(X_IDEMPOTENCY_KEY_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            AppError::bad_request("缺少 X-Idempotency-Key").with_request_id(request_id.to_string())
        })?;

    Ok(key)
}

pub fn resolve_product_id(
    state: &AppState,
    tenant_id: Uuid,
    product_id: Option<i64>,
    barcode: Option<&str>,
    request_id: &str,
) -> Result<i64, AppError> {
    if let Some(pid) = product_id {
        return Ok(pid);
    }

    let barcode = barcode
        .map(str::trim)
        .filter(|b| !b.is_empty())
        .ok_or_else(|| {
            AppError::bad_request("product_id 或 barcode 至少提供一个")
                .with_request_id(request_id.to_string())
        })?;

    let barcode_key = barcode_scope_key(&tenant_id, barcode);
    let barcode_index = state.barcode_index.lock().map_err(|_| {
        AppError::internal("条码索引锁异常").with_request_id(request_id.to_string())
    })?;

    barcode_index
        .get(&barcode_key)
        .copied()
        .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.to_string()))
}

pub async fn try_idempotent_replay(
    state: &AppState,
    scope_key: &str,
    request_payload: &Value,
    request_id: &str,
) -> Result<Option<Response>, AppError> {
    if state.repository.is_postgres() {
        if let Some(record) = state
            .repository
            .load_idempotency_record(&state.persistence, scope_key)
            .await?
        {
            if &record.request_payload != request_payload {
                return Err(AppError::conflict(4092, "幂等键请求体冲突")
                    .with_request_id(request_id.to_string()));
            }

            let replay_request_id = record
                .response_body
                .get("request_id")
                .and_then(Value::as_str)
                .unwrap_or(request_id);

            let response = (
                StatusCode::OK,
                build_response_headers(replay_request_id, true),
                Json(record.response_body),
            )
                .into_response();

            return Ok(Some(response));
        }

        return Ok(None);
    }

    let records = state.idempotency_records.lock().map_err(|_| {
        AppError::internal("幂等记录锁异常").with_request_id(request_id.to_string())
    })?;

    if let Some(record) = records.get(scope_key) {
        let stored_request_payload = record
            .get("request_payload")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if &stored_request_payload != request_payload {
            return Err(AppError::conflict(4092, "幂等键请求体冲突")
                .with_request_id(request_id.to_string()));
        }

        let response_body = record.get("response_body").cloned().ok_or_else(|| {
            AppError::internal("幂等记录缺少 response_body").with_request_id(request_id.to_string())
        })?;

        let replay_request_id = response_body
            .get("request_id")
            .and_then(Value::as_str)
            .unwrap_or(request_id);

        let response = (
            StatusCode::OK,
            build_response_headers(replay_request_id, true),
            Json(response_body),
        )
            .into_response();

        return Ok(Some(response));
    }

    Ok(None)
}

pub async fn save_idempotency_record(
    state: &AppState,
    scope_key: &str,
    request_payload: Value,
    response_body: Value,
) -> Result<(), AppError> {
    if state.repository.is_postgres() {
        return state
            .repository
            .save_idempotency_record(
                &state.persistence,
                scope_key,
                &request_payload,
                &response_body,
            )
            .await;
    }

    save_idempotency_record_sync(state, scope_key, request_payload, response_body)
}

pub fn save_idempotency_record_sync(
    state: &AppState,
    scope_key: &str,
    request_payload: Value,
    response_body: Value,
) -> Result<(), AppError> {
    let mut records = state
        .idempotency_records
        .lock()
        .map_err(|_| AppError::internal("幂等记录锁异常"))?;

    records.insert(
        scope_key.to_string(),
        json!({
            "request_payload": request_payload,
            "response_body": response_body
        }),
    );

    Ok(())
}
