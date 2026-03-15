use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserRole {
    Owner,
    Admin,
    Purchaser,
    Sales,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Owner => "OWNER",
            UserRole::Admin => "ADMIN",
            UserRole::Purchaser => "PURCHASER",
            UserRole::Sales => "SALES",
        }
    }

    /// 角色层级（数值越大权限越高）
    pub fn rank(&self) -> u8 {
        match self {
            UserRole::Owner => 100,
            UserRole::Admin => 50,
            UserRole::Purchaser => 10,
            UserRole::Sales => 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub username: String,
    pub name: String,
    pub role: UserRole,
    pub password_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: Uuid,
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: i64,
    pub tenant_id: Uuid,
    pub sku: String,
    pub barcode: String,
    pub name: String,
    pub unit: String,
    pub current_stock: i32,
    pub cost_price: Decimal,
    pub retail_price: Decimal,
    pub last_inbound_unit_cost: Option<Decimal>,
    pub min_stock_limit: i32,

    pub is_deleted: bool,
    pub category_id: Option<i64>,
    pub track_batches: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub tenant_id: Uuid,
    pub parent_id: Option<i64>,
    pub name: String,
    pub level: i16,
    pub sort_order: i32,
    pub is_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Supplier {
    pub id: i64,
    pub tenant_id: Uuid,
    pub name: String,
    pub phone: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductBatch {
    pub id: i64,
    pub tenant_id: Uuid,
    pub product_id: i64,
    pub lot_number: String,
    pub supplier: Option<String>,
    pub inbound_at: chrono::NaiveDate,
    pub produced_at: Option<chrono::NaiveDate>,
    pub expires_at: Option<chrono::NaiveDate>,
    pub notes: Option<String>,
    pub is_sold_out: bool,
    pub sold_out_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BarcodeLookupStatus {
    Found,
    NotFound,
}

impl BarcodeLookupStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Found => "FOUND",
            Self::NotFound => "NOT_FOUND",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarcodeLookupCache {
    pub tenant_id: Uuid,
    pub barcode: String,
    pub lookup_status: BarcodeLookupStatus,
    pub product_name: Option<String>,
    pub raw_payload: Value,
    pub expires_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PurchaseOrderStatus {
    Draft,
    Confirmed,
    Voided,
}

impl PurchaseOrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PurchaseOrderStatus::Draft => "DRAFT",
            PurchaseOrderStatus::Confirmed => "CONFIRMED",
            PurchaseOrderStatus::Voided => "VOIDED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SalesOrderStatus {
    Draft,
    Confirmed,
    ReturnedPartial,
    ReturnedFull,
    Voided,
}

impl SalesOrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SalesOrderStatus::Draft => "DRAFT",
            SalesOrderStatus::Confirmed => "CONFIRMED",
            SalesOrderStatus::ReturnedPartial => "RETURNED_PARTIAL",
            SalesOrderStatus::ReturnedFull => "RETURNED_FULL",
            SalesOrderStatus::Voided => "VOIDED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StockCheckStatus {
    Draft,
    Counting,
    Confirmed,
}

impl StockCheckStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            StockCheckStatus::Draft => "DRAFT",
            StockCheckStatus::Counting => "COUNTING",
            StockCheckStatus::Confirmed => "CONFIRMED",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderItem {
    pub product_id: i64,
    pub qty: i32,
    pub unit_cost: Decimal,
    pub product_name_snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrder {
    pub id: i64,
    pub tenant_id: Uuid,
    pub biz_no: String,
    pub supplier_id: Option<i64>,
    pub status: PurchaseOrderStatus,
    pub items: Vec<PurchaseOrderItem>,
    pub remark: Option<String>,
    pub created_by: Uuid,

    pub confirmed_at: Option<String>,
    pub voided_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrderItem {
    pub product_id: i64,
    pub qty: i32,
    pub sell_price: Decimal,
    pub returned_qty: i32,
    pub product_name_snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrder {
    pub id: i64,
    pub tenant_id: Uuid,
    pub biz_no: String,
    pub customer_id: Option<i64>,
    pub status: SalesOrderStatus,
    pub items: Vec<SalesOrderItem>,
    pub remark: Option<String>,
    pub created_by: Uuid,

    pub confirmed_at: Option<String>,
    pub returned_at: Option<String>,
    pub voided_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockCheckItem {
    pub product_id: i64,
    pub book_stock: i32,
    pub actual_stock: Option<i32>,
    pub delta_qty: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockCheck {
    pub id: i64,
    pub tenant_id: Uuid,
    pub biz_no: String,
    pub status: StockCheckStatus,
    pub items: Vec<StockCheckItem>,
    pub remark: Option<String>,
    pub created_by: Uuid,

    pub counting_at: Option<String>,
    pub confirmed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: i64,
    pub tenant_id: Uuid,
    pub operator_id: Uuid,
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub before_data: Value,
    pub after_data: Value,
    pub request_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockLog {
    pub id: i64,
    pub tenant_id: Uuid,
    pub product_id: i64,
    pub biz_type: String,
    pub biz_no: String,
    pub delta_qty: i32,
    pub snapshot_stock: i32,
    pub snapshot_cost: Decimal,
    pub snapshot_sell_price: Option<Decimal>,
    pub snapshot_inbound_unit_cost: Option<Decimal>,
    pub operator_id: Uuid,
    pub created_at: String,
}

impl StockLog {
    #[allow(clippy::too_many_arguments)]
    pub fn now(
        id: i64,
        tenant_id: Uuid,
        product_id: i64,
        biz_type: impl Into<String>,
        biz_no: impl Into<String>,
        delta_qty: i32,
        snapshot_stock: i32,
        snapshot_cost: Decimal,
        snapshot_sell_price: Option<Decimal>,
        snapshot_inbound_unit_cost: Option<Decimal>,
        operator_id: Uuid,
    ) -> Self {
        Self {
            id,
            tenant_id,
            product_id,
            biz_type: biz_type.into(),
            biz_no: biz_no.into(),
            delta_qty,
            snapshot_stock,
            snapshot_cost,
            snapshot_sell_price,
            snapshot_inbound_unit_cost,
            operator_id,
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

impl AuditLog {
    #[allow(clippy::too_many_arguments)]
    pub fn now(
        id: i64,
        tenant_id: Uuid,
        operator_id: Uuid,
        action: impl Into<String>,
        target_type: impl Into<String>,
        target_id: impl Into<String>,
        before_data: Value,
        after_data: Value,
        request_id: impl Into<String>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            operator_id,
            action: action.into(),
            target_type: target_type.into(),
            target_id: target_id.into(),
            before_data,
            after_data,
            request_id: request_id.into(),
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

pub fn hash_password(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    format!("{:x}", hasher.finalize())
}
