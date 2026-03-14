use std::collections::{HashMap, HashSet};

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{
    config::StorageBackend,
    error::AppError,
    models::{
        AuditLog, BarcodeLookupCache, BarcodeLookupStatus, Product, PurchaseOrder,
        PurchaseOrderItem, PurchaseOrderStatus, SalesOrder, SalesOrderItem, SalesOrderStatus,
        StockCheck, StockCheckItem, StockCheckStatus, StockLog, User, UserRole,
    },
};

#[derive(Debug, Clone)]
pub struct IdempotencyRecord {
    pub request_payload: Value,
    pub response_body: Value,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryRepository;

#[derive(Debug, Clone, Copy, Default)]
pub struct PostgresRepository;

#[derive(Debug, Clone, Copy)]
pub enum RepositoryProvider {
    Memory(MemoryRepository),
    Postgres(PostgresRepository),
}

pub fn build_repository_provider(backend: StorageBackend) -> RepositoryProvider {
    match backend {
        StorageBackend::Memory => RepositoryProvider::Memory(MemoryRepository),
        StorageBackend::Postgres => RepositoryProvider::Postgres(PostgresRepository),
    }
}

impl RepositoryProvider {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Memory(_) => "memory",
            Self::Postgres(_) => "postgres",
        }
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    pub async fn find_user_by_username(
        &self,
        pool: Option<&PgPool>,
        username: &str,
    ) -> Result<Option<User>, AppError> {
        match self {
            Self::Postgres(repo) => repo.find_user_by_username(pool, username).await,
            Self::Memory(_) => Ok(None),
        }
    }

    pub async fn find_user_by_id(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<User>, AppError> {
        match self {
            Self::Postgres(repo) => repo.find_user_by_id(pool, tenant_id, user_id).await,
            Self::Memory(_) => Ok(None),
        }
    }

    pub async fn create_tenant(
        &self,
        pool: Option<&PgPool>,
        tenant: &crate::models::Tenant,
    ) -> Result<(), AppError> {
        match self {
            Self::Postgres(repo) => repo.create_tenant(pool, tenant).await,
            Self::Memory(_) => Ok(()),
        }
    }

    pub async fn find_user_by_tenant_code_and_username(
        &self,
        pool: Option<&PgPool>,
        tenant_code: &str,
        username: &str,
    ) -> Result<Option<User>, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.find_user_by_tenant_code_and_username(pool, tenant_code, username)
                    .await
            }
            Self::Memory(_) => Ok(None),
        }
    }

    pub async fn list_users_by_tenant(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
    ) -> Result<Vec<User>, AppError> {
        match self {
            Self::Postgres(repo) => repo.list_users_by_tenant(pool, tenant_id).await,
            Self::Memory(_) => Ok(Vec::new()),
        }
    }

    pub async fn create_user(&self, pool: Option<&PgPool>, user: &User) -> Result<(), AppError> {
        match self {
            Self::Postgres(repo) => repo.create_user(pool, user).await,
            Self::Memory(_) => Err(unsupported_operation("create_user")),
        }
    }

    pub async fn update_user_role(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        user_id: Uuid,
        new_role: UserRole,
    ) -> Result<(), AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.update_user_role(pool, tenant_id, user_id, new_role)
                    .await
            }
            Self::Memory(_) => Err(unsupported_operation("update_user_role")),
        }
    }

    pub async fn reset_password(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        user_id: Uuid,
        new_password_hash: &str,
    ) -> Result<(), AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.reset_password(pool, tenant_id, user_id, new_password_hash)
                    .await
            }
            Self::Memory(_) => Err(unsupported_operation("reset_password")),
        }
    }

    pub async fn find_product_by_barcode(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        barcode: &str,
        include_deleted: bool,
    ) -> Result<Option<Product>, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.find_product_by_barcode(pool, tenant_id, barcode, include_deleted)
                    .await
            }
            Self::Memory(_) => Ok(None),
        }
    }

    pub async fn find_barcode_lookup_cache(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        barcode: &str,
    ) -> Result<Option<BarcodeLookupCache>, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.find_barcode_lookup_cache(pool, tenant_id, barcode).await
            }
            Self::Memory(_) => Ok(None),
        }
    }

    pub async fn upsert_barcode_lookup_cache(
        &self,
        pool: Option<&PgPool>,
        cache: &BarcodeLookupCache,
    ) -> Result<(), AppError> {
        match self {
            Self::Postgres(repo) => repo.upsert_barcode_lookup_cache(pool, cache).await,
            Self::Memory(_) => Ok(()),
        }
    }

    pub async fn list_products_by_tenant(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
    ) -> Result<Vec<Product>, AppError> {
        match self {
            Self::Postgres(repo) => repo.list_products_by_tenant(pool, tenant_id).await,
            Self::Memory(_) => Ok(Vec::new()),
        }
    }

    pub async fn list_products_by_tenant_all(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
    ) -> Result<Vec<Product>, AppError> {
        match self {
            Self::Postgres(repo) => repo.list_products_by_tenant_all(pool, tenant_id).await,
            Self::Memory(_) => Ok(Vec::new()),
        }
    }

    pub async fn list_sales_orders_by_tenant(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
    ) -> Result<Vec<SalesOrder>, AppError> {
        match self {
            Self::Postgres(repo) => repo.list_sales_orders_by_tenant(pool, tenant_id).await,
            Self::Memory(_) => Ok(Vec::new()),
        }
    }

    pub async fn list_purchase_orders_by_tenant(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
    ) -> Result<Vec<PurchaseOrder>, AppError> {
        match self {
            Self::Postgres(repo) => repo.list_purchase_orders_by_tenant(pool, tenant_id).await,
            Self::Memory(_) => Ok(Vec::new()),
        }
    }

    pub async fn list_stock_logs_by_tenant(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
    ) -> Result<Vec<StockLog>, AppError> {
        match self {
            Self::Postgres(repo) => repo.list_stock_logs_by_tenant(pool, tenant_id).await,
            Self::Memory(_) => Ok(Vec::new()),
        }
    }

    pub async fn list_audit_logs_by_tenant(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
    ) -> Result<Vec<AuditLog>, AppError> {
        match self {
            Self::Postgres(repo) => repo.list_audit_logs_by_tenant(pool, tenant_id).await,
            Self::Memory(_) => Ok(Vec::new()),
        }
    }

    pub async fn find_product_by_id(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        product_id: i64,
        include_deleted: bool,
    ) -> Result<Option<Product>, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.find_product_by_id(pool, tenant_id, product_id, include_deleted)
                    .await
            }
            Self::Memory(_) => Ok(None),
        }
    }

    pub async fn is_barcode_taken(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        barcode: &str,
        exclude_product_id: Option<i64>,
    ) -> Result<Option<i64>, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.is_barcode_taken(pool, tenant_id, barcode, exclude_product_id)
                    .await
            }
            Self::Memory(_) => Ok(None),
        }
    }

    pub async fn is_sku_taken(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        sku: &str,
        exclude_product_id: Option<i64>,
    ) -> Result<bool, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.is_sku_taken(pool, tenant_id, sku, exclude_product_id)
                    .await
            }
            Self::Memory(_) => Ok(false),
        }
    }

    pub async fn next_product_id(&self, pool: Option<&PgPool>) -> Result<i64, AppError> {
        match self {
            Self::Postgres(repo) => repo.next_product_id(pool).await,
            Self::Memory(_) => Err(unsupported_operation("next_product_id")),
        }
    }

    pub async fn create_product(
        &self,
        pool: Option<&PgPool>,
        product: &Product,
    ) -> Result<(), AppError> {
        match self {
            Self::Postgres(repo) => repo.create_product(pool, product).await,
            Self::Memory(_) => Err(unsupported_operation("create_product")),
        }
    }

    pub async fn update_product(
        &self,
        pool: Option<&PgPool>,
        product: &Product,
        expected_version: Option<i32>,
    ) -> Result<(), AppError> {
        match self {
            Self::Postgres(repo) => repo.update_product(pool, product, expected_version).await,
            Self::Memory(_) => Err(unsupported_operation("update_product")),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn inbound(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        product_id: i64,
        qty: i32,
        unit_cost: Option<Decimal>,
        expected_version: Option<i32>,
        biz_no: &str,
        operator_id: Uuid,
    ) -> Result<Product, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.inbound(
                    pool,
                    tenant_id,
                    product_id,
                    qty,
                    unit_cost,
                    expected_version,
                    biz_no,
                    operator_id,
                )
                .await
            }
            Self::Memory(_) => Err(unsupported_operation("inbound")),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn inbound_batch(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        biz_no: &str,
        items: &[(i64, i32, Option<Decimal>, Option<i32>)],
        operator_id: Uuid,
    ) -> Result<Vec<Product>, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.inbound_batch(pool, tenant_id, biz_no, items, operator_id)
                    .await
            }
            Self::Memory(_) => Err(unsupported_operation("inbound_batch")),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn outbound(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        biz_no: &str,
        default_expected_version: Option<i32>,
        items: &[(i64, i32, Option<i32>, Option<Decimal>)],
        allow_negative_stock: bool,
        operator_id: Uuid,
    ) -> Result<Vec<Product>, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.outbound(
                    pool,
                    tenant_id,
                    biz_no,
                    default_expected_version,
                    items,
                    allow_negative_stock,
                    operator_id,
                )
                .await
            }
            Self::Memory(_) => Err(unsupported_operation("outbound")),
        }
    }

    pub async fn load_idempotency_record(
        &self,
        pool: Option<&PgPool>,
        scope_key: &str,
    ) -> Result<Option<IdempotencyRecord>, AppError> {
        match self {
            Self::Postgres(repo) => repo.load_idempotency_record(pool, scope_key).await,
            Self::Memory(_) => Err(unsupported_operation("load_idempotency_record")),
        }
    }

    pub async fn save_idempotency_record(
        &self,
        pool: Option<&PgPool>,
        scope_key: &str,
        request_payload: &Value,
        response_body: &Value,
    ) -> Result<(), AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.save_idempotency_record(pool, scope_key, request_payload, response_body)
                    .await
            }
            Self::Memory(_) => Err(unsupported_operation("save_idempotency_record")),
        }
    }

    pub async fn next_purchase_order_id(&self, pool: Option<&PgPool>) -> Result<i64, AppError> {
        match self {
            Self::Postgres(repo) => repo.next_purchase_order_id(pool).await,
            Self::Memory(_) => Err(unsupported_operation("next_purchase_order_id")),
        }
    }

    pub async fn is_purchase_order_biz_no_taken(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        biz_no: &str,
    ) -> Result<bool, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.is_purchase_order_biz_no_taken(pool, tenant_id, biz_no)
                    .await
            }
            Self::Memory(_) => Ok(false),
        }
    }

    pub async fn create_purchase_order(
        &self,
        pool: Option<&PgPool>,
        order: &PurchaseOrder,
    ) -> Result<(), AppError> {
        match self {
            Self::Postgres(repo) => repo.create_purchase_order(pool, order).await,
            Self::Memory(_) => Err(unsupported_operation("create_purchase_order")),
        }
    }

    pub async fn find_purchase_order_by_id(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        order_id: i64,
    ) -> Result<Option<PurchaseOrder>, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.find_purchase_order_by_id(pool, tenant_id, order_id)
                    .await
            }
            Self::Memory(_) => Ok(None),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn confirm_purchase_order(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        order_id: i64,
        expected_version: Option<i32>,
        operator_id: Uuid,
        request_id: &str,
    ) -> Result<PurchaseOrder, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.confirm_purchase_order(
                    pool,
                    tenant_id,
                    order_id,
                    expected_version,
                    operator_id,
                    request_id,
                )
                .await
            }
            Self::Memory(_) => Err(unsupported_operation("confirm_purchase_order")),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn void_purchase_order(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        order_id: i64,
        expected_version: Option<i32>,
        allow_negative_stock: bool,
        operator_id: Uuid,
        request_id: &str,
    ) -> Result<PurchaseOrder, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.void_purchase_order(
                    pool,
                    tenant_id,
                    order_id,
                    expected_version,
                    allow_negative_stock,
                    operator_id,
                    request_id,
                )
                .await
            }
            Self::Memory(_) => Err(unsupported_operation("void_purchase_order")),
        }
    }

    pub async fn next_sales_order_id(&self, pool: Option<&PgPool>) -> Result<i64, AppError> {
        match self {
            Self::Postgres(repo) => repo.next_sales_order_id(pool).await,
            Self::Memory(_) => Err(unsupported_operation("next_sales_order_id")),
        }
    }

    pub async fn is_sales_order_biz_no_taken(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        biz_no: &str,
    ) -> Result<bool, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.is_sales_order_biz_no_taken(pool, tenant_id, biz_no)
                    .await
            }
            Self::Memory(_) => Ok(false),
        }
    }

    pub async fn create_sales_order(
        &self,
        pool: Option<&PgPool>,
        order: &SalesOrder,
    ) -> Result<(), AppError> {
        match self {
            Self::Postgres(repo) => repo.create_sales_order(pool, order).await,
            Self::Memory(_) => Err(unsupported_operation("create_sales_order")),
        }
    }

    pub async fn find_sales_order_by_id(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        order_id: i64,
    ) -> Result<Option<SalesOrder>, AppError> {
        match self {
            Self::Postgres(repo) => repo.find_sales_order_by_id(pool, tenant_id, order_id).await,
            Self::Memory(_) => Ok(None),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn confirm_sales_order(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        order_id: i64,
        expected_version: Option<i32>,
        allow_negative_stock: bool,
        operator_id: Uuid,
        request_id: &str,
    ) -> Result<SalesOrder, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.confirm_sales_order(
                    pool,
                    tenant_id,
                    order_id,
                    expected_version,
                    allow_negative_stock,
                    operator_id,
                    request_id,
                )
                .await
            }
            Self::Memory(_) => Err(unsupported_operation("confirm_sales_order")),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn void_sales_order(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        order_id: i64,
        expected_version: Option<i32>,
        operator_id: Uuid,
        request_id: &str,
    ) -> Result<SalesOrder, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.void_sales_order(
                    pool,
                    tenant_id,
                    order_id,
                    expected_version,
                    operator_id,
                    request_id,
                )
                .await
            }
            Self::Memory(_) => Err(unsupported_operation("void_sales_order")),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn return_sales_order(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        order_id: i64,
        expected_version: Option<i32>,
        return_items: &[(i64, i32)],
        remark: Option<String>,
        operator_id: Uuid,
        request_id: &str,
    ) -> Result<SalesOrder, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.return_sales_order(
                    pool,
                    tenant_id,
                    order_id,
                    expected_version,
                    return_items,
                    remark,
                    operator_id,
                    request_id,
                )
                .await
            }
            Self::Memory(_) => Err(unsupported_operation("return_sales_order")),
        }
    }

    pub async fn next_stock_check_id(&self, pool: Option<&PgPool>) -> Result<i64, AppError> {
        match self {
            Self::Postgres(repo) => repo.next_stock_check_id(pool).await,
            Self::Memory(_) => Err(unsupported_operation("next_stock_check_id")),
        }
    }

    pub async fn is_stock_check_biz_no_taken(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        biz_no: &str,
    ) -> Result<bool, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.is_stock_check_biz_no_taken(pool, tenant_id, biz_no)
                    .await
            }
            Self::Memory(_) => Ok(false),
        }
    }

    pub async fn create_stock_check(
        &self,
        pool: Option<&PgPool>,
        check: &StockCheck,
    ) -> Result<(), AppError> {
        match self {
            Self::Postgres(repo) => repo.create_stock_check(pool, check).await,
            Self::Memory(_) => Err(unsupported_operation("create_stock_check")),
        }
    }

    pub async fn find_stock_check_by_id(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        check_id: i64,
    ) -> Result<Option<StockCheck>, AppError> {
        match self {
            Self::Postgres(repo) => repo.find_stock_check_by_id(pool, tenant_id, check_id).await,
            Self::Memory(_) => Ok(None),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn start_stock_check(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        check_id: i64,
        expected_version: Option<i32>,
        operator_id: Uuid,
        request_id: &str,
    ) -> Result<StockCheck, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.start_stock_check(
                    pool,
                    tenant_id,
                    check_id,
                    expected_version,
                    operator_id,
                    request_id,
                )
                .await
            }
            Self::Memory(_) => Err(unsupported_operation("start_stock_check")),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn confirm_stock_check(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        check_id: i64,
        expected_version: Option<i32>,
        actual_items: &[(i64, i32)],
        remark: Option<String>,
        operator_id: Uuid,
        request_id: &str,
    ) -> Result<StockCheck, AppError> {
        match self {
            Self::Postgres(repo) => {
                repo.confirm_stock_check(
                    pool,
                    tenant_id,
                    check_id,
                    expected_version,
                    actual_items,
                    remark,
                    operator_id,
                    request_id,
                )
                .await
            }
            Self::Memory(_) => Err(unsupported_operation("confirm_stock_check")),
        }
    }
}

impl PostgresRepository {
    pub async fn find_user_by_username(
        &self,
        pool: Option<&PgPool>,
        username: &str,
    ) -> Result<Option<User>, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, username, name, role, password_hash
            FROM users
            WHERE username = $1
            LIMIT 1
            "#,
        )
        .bind(username)
        .fetch_optional(pool)
        .await
        .map_err(|err| map_sqlx_error("查询用户失败", err))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let role_raw: String = row
            .try_get("role")
            .map_err(|err| map_sqlx_error("读取用户角色失败", err))?;

        Ok(Some(User {
            id: row
                .try_get("id")
                .map_err(|err| map_sqlx_error("读取用户ID失败", err))?,
            tenant_id: row
                .try_get("tenant_id")
                .map_err(|err| map_sqlx_error("读取租户ID失败", err))?,
            username: row
                .try_get("username")
                .map_err(|err| map_sqlx_error("读取用户名失败", err))?,
            name: row
                .try_get("name")
                .map_err(|err| map_sqlx_error("读取姓名失败", err))?,
            role: parse_user_role(&role_raw)?,
            password_hash: row
                .try_get("password_hash")
                .map_err(|err| map_sqlx_error("读取密码摘要失败", err))?,
        }))
    }

    pub async fn find_user_by_id(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<User>, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, username, name, role, password_hash
            FROM users
            WHERE id = $1 AND tenant_id = $2
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
        .map_err(|err| map_sqlx_error("按ID查询用户失败", err))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let role_raw: String = row
            .try_get("role")
            .map_err(|err| map_sqlx_error("读取用户角色失败", err))?;

        Ok(Some(User {
            id: row
                .try_get("id")
                .map_err(|err| map_sqlx_error("读取用户ID失败", err))?,
            tenant_id: row
                .try_get("tenant_id")
                .map_err(|err| map_sqlx_error("读取租户ID失败", err))?,
            username: row
                .try_get("username")
                .map_err(|err| map_sqlx_error("读取用户名失败", err))?,
            name: row
                .try_get("name")
                .map_err(|err| map_sqlx_error("读取姓名失败", err))?,
            role: parse_user_role(&role_raw)?,
            password_hash: row
                .try_get("password_hash")
                .map_err(|err| map_sqlx_error("读取密码摘要失败", err))?,
        }))
    }

    pub async fn create_tenant(
        &self,
        pool: Option<&PgPool>,
        tenant: &crate::models::Tenant,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        sqlx::query(
            r#"
            INSERT INTO tenants (id, code, name)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(tenant.id)
        .bind(&tenant.code)
        .bind(&tenant.name)
        .execute(pool)
        .await
        .map_err(|err| map_sqlx_error("创建租户失败", err))?;
        Ok(())
    }

    /// 通过 tenant_code + username 找用户（供租户码登录使用）
    pub async fn find_user_by_tenant_code_and_username(
        &self,
        pool: Option<&PgPool>,
        tenant_code: &str,
        username: &str,
    ) -> Result<Option<User>, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query(
            r#"
            SELECT u.id, u.tenant_id, u.username, u.name, u.role, u.password_hash
            FROM users u
            JOIN tenants t ON u.tenant_id = t.id
            WHERE UPPER(t.code) = UPPER($1) AND u.username = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_code)
        .bind(username)
        .fetch_optional(pool)
        .await
        .map_err(|err| map_sqlx_error("按租户码查询用户失败", err))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let role_raw: String = row
            .try_get("role")
            .map_err(|err| map_sqlx_error("读取用户角色失败", err))?;

        Ok(Some(User {
            id: row
                .try_get("id")
                .map_err(|err| map_sqlx_error("读取用户ID失败", err))?,
            tenant_id: row
                .try_get("tenant_id")
                .map_err(|err| map_sqlx_error("读取租户ID失败", err))?,
            username: row
                .try_get("username")
                .map_err(|err| map_sqlx_error("读取用户名失败", err))?,
            name: row
                .try_get("name")
                .map_err(|err| map_sqlx_error("读取姓名失败", err))?,
            role: parse_user_role(&role_raw)?,
            password_hash: row
                .try_get("password_hash")
                .map_err(|err| map_sqlx_error("读取密码摘要失败", err))?,
        }))
    }

    pub async fn list_users_by_tenant(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
    ) -> Result<Vec<User>, AppError> {
        let pool = require_pool(pool)?;
        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, username, name, role, password_hash
            FROM users
            WHERE tenant_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .map_err(|err| map_sqlx_error("查询员工列表失败", err))?;

        let mut users = Vec::with_capacity(rows.len());
        for row in rows {
            let role_raw: String = row
                .try_get("role")
                .map_err(|err| map_sqlx_error("读取用户角色失败", err))?;

            users.push(User {
                id: row
                    .try_get("id")
                    .map_err(|err| map_sqlx_error("读取用户ID失败", err))?,
                tenant_id: row
                    .try_get("tenant_id")
                    .map_err(|err| map_sqlx_error("读取租户ID失败", err))?,
                username: row
                    .try_get("username")
                    .map_err(|err| map_sqlx_error("读取用户名失败", err))?,
                name: row
                    .try_get("name")
                    .map_err(|err| map_sqlx_error("读取姓名失败", err))?,
                role: parse_user_role(&role_raw)?,
                password_hash: row
                    .try_get("password_hash")
                    .map_err(|err| map_sqlx_error("读取密码摘要失败", err))?,
            });
        }
        Ok(users)
    }

    pub async fn create_user(&self, pool: Option<&PgPool>, user: &User) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        let result = sqlx::query(
            r#"
            INSERT INTO users (id, tenant_id, username, name, role, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
            "#,
        )
        .bind(user.id)
        .bind(user.tenant_id)
        .bind(&user.username)
        .bind(&user.name)
        .bind(user.role.as_str())
        .bind(&user.password_hash)
        .execute(pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(err) => {
                if is_sql_state(&err, "23505") {
                    return Err(AppError::conflict(4090, "用户名已存在")
                        .with_data(json!({ "username": user.username })));
                }
                Err(map_sqlx_error("创建员工失败", err))
            }
        }
    }

    pub async fn update_user_role(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        user_id: Uuid,
        new_role: UserRole,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        let result = sqlx::query(
            r#"
            UPDATE users
            SET role = $1, updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            "#,
        )
        .bind(new_role.as_str())
        .bind(user_id)
        .bind(tenant_id)
        .execute(pool)
        .await
        .map_err(|err| map_sqlx_error("修改员工角色失败", err))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("员工不存在"));
        }
        Ok(())
    }

    pub async fn reset_password(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        user_id: Uuid,
        new_password_hash: &str,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        let result = sqlx::query(
            r#"
            UPDATE users
            SET password_hash = $1, updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            "#,
        )
        .bind(new_password_hash)
        .bind(user_id)
        .bind(tenant_id)
        .execute(pool)
        .await
        .map_err(|err| map_sqlx_error("重置员工密码失败", err))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("员工不存在"));
        }
        Ok(())
    }

    pub async fn find_product_by_barcode(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        barcode: &str,
        include_deleted: bool,
    ) -> Result<Option<Product>, AppError> {
        let pool = require_pool(pool)?;
        let row = if include_deleted {
            sqlx::query(
                r#"
                SELECT id, tenant_id, sku, barcode, name, unit,
                       current_stock, cost_price, retail_price, last_inbound_unit_cost,
                       min_stock_limit, version, is_deleted
                FROM products
                WHERE tenant_id = $1 AND barcode = $2
                LIMIT 1
                "#,
            )
            .bind(tenant_id)
            .bind(barcode)
            .fetch_optional(pool)
            .await
            .map_err(|err| map_sqlx_error("按条码查询商品失败", err))?
        } else {
            sqlx::query(
                r#"
                SELECT id, tenant_id, sku, barcode, name, unit,
                       current_stock, cost_price, retail_price, last_inbound_unit_cost,
                       min_stock_limit, version, is_deleted
                FROM products
                WHERE tenant_id = $1 AND barcode = $2 AND is_deleted = FALSE
                LIMIT 1
                "#,
            )
            .bind(tenant_id)
            .bind(barcode)
            .fetch_optional(pool)
            .await
            .map_err(|err| map_sqlx_error("按条码查询商品失败", err))?
        };

        row.map(map_product_row).transpose()
    }

    pub async fn find_barcode_lookup_cache(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        barcode: &str,
    ) -> Result<Option<BarcodeLookupCache>, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query(
            r#"
            SELECT tenant_id, barcode, lookup_status, product_name, raw_payload, expires_at, updated_at
            FROM barcode_lookup_cache
            WHERE tenant_id = $1 AND barcode = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(barcode)
        .fetch_optional(pool)
        .await
        .map_err(|err| map_sqlx_error("查询条码缓存失败", err))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let lookup_status_raw: String = row
            .try_get("lookup_status")
            .map_err(|err| map_sqlx_error("读取条码缓存状态失败", err))?;

        Ok(Some(BarcodeLookupCache {
            tenant_id: row
                .try_get("tenant_id")
                .map_err(|err| map_sqlx_error("读取条码缓存租户ID失败", err))?,
            barcode: row
                .try_get("barcode")
                .map_err(|err| map_sqlx_error("读取条码缓存条码失败", err))?,
            lookup_status: parse_barcode_lookup_status(&lookup_status_raw)?,
            product_name: row
                .try_get("product_name")
                .map_err(|err| map_sqlx_error("读取条码缓存商品名失败", err))?,
            raw_payload: row
                .try_get("raw_payload")
                .map_err(|err| map_sqlx_error("读取条码缓存原始响应失败", err))?,
            expires_at: row
                .try_get("expires_at")
                .map_err(|err| map_sqlx_error("读取条码缓存过期时间失败", err))?,
            updated_at: row
                .try_get("updated_at")
                .map_err(|err| map_sqlx_error("读取条码缓存更新时间失败", err))?,
        }))
    }

    pub async fn upsert_barcode_lookup_cache(
        &self,
        pool: Option<&PgPool>,
        cache: &BarcodeLookupCache,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        sqlx::query(
            r#"
            INSERT INTO barcode_lookup_cache (
                tenant_id, barcode, lookup_status, product_name, raw_payload, expires_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (tenant_id, barcode)
            DO UPDATE SET lookup_status = EXCLUDED.lookup_status,
                          product_name = EXCLUDED.product_name,
                          raw_payload = EXCLUDED.raw_payload,
                          expires_at = EXCLUDED.expires_at,
                          updated_at = NOW()
            "#,
        )
        .bind(cache.tenant_id)
        .bind(&cache.barcode)
        .bind(cache.lookup_status.as_str())
        .bind(&cache.product_name)
        .bind(&cache.raw_payload)
        .bind(cache.expires_at)
        .execute(pool)
        .await
        .map_err(|err| map_sqlx_error("写入条码缓存失败", err))?;

        Ok(())
    }

    pub async fn list_products_by_tenant(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
    ) -> Result<Vec<Product>, AppError> {
        let pool = require_pool(pool)?;
        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, sku, barcode, name, unit,
                   current_stock, cost_price, retail_price, last_inbound_unit_cost,
                   min_stock_limit, version, is_deleted
            FROM products
            WHERE tenant_id = $1 AND is_deleted = FALSE
            ORDER BY id ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .map_err(|err| map_sqlx_error("查询商品列表失败", err))?;

        rows.into_iter().map(map_product_row).collect()
    }

    pub async fn list_products_by_tenant_all(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
    ) -> Result<Vec<Product>, AppError> {
        let pool = require_pool(pool)?;
        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, sku, barcode, name, unit,
                   current_stock, cost_price, retail_price, last_inbound_unit_cost,
                   min_stock_limit, version, is_deleted
            FROM products
            WHERE tenant_id = $1
            ORDER BY id ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .map_err(|err| map_sqlx_error("查询商品列表失败", err))?;

        rows.into_iter().map(map_product_row).collect()
    }

    pub async fn list_sales_orders_by_tenant(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
    ) -> Result<Vec<SalesOrder>, AppError> {
        let pool = require_pool(pool)?;

        let order_rows = sqlx::query(
            r#"
            SELECT id, tenant_id, biz_no, customer_id,
                   status, remark, created_by,
                   version, confirmed_at, returned_at, voided_at,
                   created_at, updated_at
            FROM sales_orders
            WHERE tenant_id = $1
            ORDER BY id ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .map_err(|err| map_sqlx_error("查询销售单列表失败", err))?;

        if order_rows.is_empty() {
            return Ok(Vec::new());
        }

        let item_rows = sqlx::query(
            r#"
            SELECT sales_order_id, product_id, qty, sell_price, returned_qty, product_name_snapshot
            FROM sales_order_items
            WHERE tenant_id = $1
            ORDER BY sales_order_id ASC, id ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .map_err(|err| map_sqlx_error("查询销售单明细列表失败", err))?;

        let mut items_by_order: HashMap<i64, Vec<SalesOrderItem>> = HashMap::new();
        for item_row in item_rows {
            let order_id: i64 = item_row
                .try_get("sales_order_id")
                .map_err(|err| map_sqlx_error("读取销售单ID失败", err))?;
            items_by_order
                .entry(order_id)
                .or_default()
                .push(SalesOrderItem {
                    product_id: item_row
                        .try_get("product_id")
                        .map_err(|err| map_sqlx_error("读取销售明细商品ID失败", err))?,
                    qty: item_row
                        .try_get("qty")
                        .map_err(|err| map_sqlx_error("读取销售明细数量失败", err))?,
                    sell_price: item_row
                        .try_get("sell_price")
                        .map_err(|err| map_sqlx_error("读取销售明细单价失败", err))?,
                    returned_qty: item_row
                        .try_get("returned_qty")
                        .map_err(|err| map_sqlx_error("读取销售明细已退数量失败", err))?,
                    product_name_snapshot: item_row
                        .try_get("product_name_snapshot")
                        .map_err(|err| map_sqlx_error("读取销售明细商品名称快照失败", err))?,
                });
        }

        let mut orders = Vec::with_capacity(order_rows.len());
        for row in order_rows {
            let order_id: i64 = row
                .try_get("id")
                .map_err(|err| map_sqlx_error("读取销售单ID失败", err))?;
            let status_raw: String = row
                .try_get("status")
                .map_err(|err| map_sqlx_error("读取销售单状态失败", err))?;

            let confirmed_at = row
                .try_get::<Option<DateTime<Utc>>, _>("confirmed_at")
                .map_err(|err| map_sqlx_error("读取确认时间失败", err))?
                .map(|dt| dt.to_rfc3339());
            let returned_at = row
                .try_get::<Option<DateTime<Utc>>, _>("returned_at")
                .map_err(|err| map_sqlx_error("读取退货时间失败", err))?
                .map(|dt| dt.to_rfc3339());
            let voided_at = row
                .try_get::<Option<DateTime<Utc>>, _>("voided_at")
                .map_err(|err| map_sqlx_error("读取作废时间失败", err))?
                .map(|dt| dt.to_rfc3339());

            let created_at = row
                .try_get::<DateTime<Utc>, _>("created_at")
                .map_err(|err| map_sqlx_error("读取创建时间失败", err))?
                .to_rfc3339();
            let updated_at = row
                .try_get::<DateTime<Utc>, _>("updated_at")
                .map_err(|err| map_sqlx_error("读取更新时间失败", err))?
                .to_rfc3339();

            orders.push(SalesOrder {
                id: order_id,
                tenant_id: row
                    .try_get("tenant_id")
                    .map_err(|err| map_sqlx_error("读取租户ID失败", err))?,
                biz_no: row
                    .try_get("biz_no")
                    .map_err(|err| map_sqlx_error("读取销售单号失败", err))?,
                customer_id: row
                    .try_get("customer_id")
                    .map_err(|err| map_sqlx_error("读取客户ID失败", err))?,
                status: parse_sales_order_status(&status_raw)?,
                items: items_by_order.remove(&order_id).unwrap_or_default(),
                remark: row
                    .try_get("remark")
                    .map_err(|err| map_sqlx_error("读取备注失败", err))?,
                created_by: row
                    .try_get("created_by")
                    .map_err(|err| map_sqlx_error("读取创建人失败", err))?,
                version: row
                    .try_get("version")
                    .map_err(|err| map_sqlx_error("读取版本失败", err))?,
                confirmed_at,
                returned_at,
                voided_at,
                created_at,
                updated_at,
            });
        }

        Ok(orders)
    }

    pub async fn list_purchase_orders_by_tenant(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
    ) -> Result<Vec<PurchaseOrder>, AppError> {
        let pool = require_pool(pool)?;

        let order_rows = sqlx::query(
            r#"
            SELECT id, tenant_id, biz_no, supplier_id,
                   status, remark, created_by,
                   version, confirmed_at, voided_at,
                   created_at, updated_at
            FROM purchase_orders
            WHERE tenant_id = $1
            ORDER BY id ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .map_err(|err| map_sqlx_error("查询采购单列表失败", err))?;

        if order_rows.is_empty() {
            return Ok(Vec::new());
        }

        let item_rows = sqlx::query(
            r#"
            SELECT purchase_order_id, product_id, qty, unit_cost, product_name_snapshot
            FROM purchase_order_items
            WHERE tenant_id = $1
            ORDER BY purchase_order_id ASC, id ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .map_err(|err| map_sqlx_error("查询采购单明细列表失败", err))?;

        let mut items_by_order: HashMap<i64, Vec<PurchaseOrderItem>> = HashMap::new();
        for item_row in item_rows {
            let order_id: i64 = item_row
                .try_get("purchase_order_id")
                .map_err(|err| map_sqlx_error("读取采购单ID失败", err))?;
            items_by_order
                .entry(order_id)
                .or_default()
                .push(PurchaseOrderItem {
                    product_id: item_row
                        .try_get("product_id")
                        .map_err(|err| map_sqlx_error("读取采购明细商品ID失败", err))?,
                    qty: item_row
                        .try_get("qty")
                        .map_err(|err| map_sqlx_error("读取采购明细数量失败", err))?,
                    unit_cost: item_row
                        .try_get("unit_cost")
                        .map_err(|err| map_sqlx_error("读取采购明细单价失败", err))?,
                    product_name_snapshot: item_row
                        .try_get("product_name_snapshot")
                        .map_err(|err| map_sqlx_error("读取采购明细商品名称快照失败", err))?,
                });
        }

        let mut orders = Vec::with_capacity(order_rows.len());
        for row in order_rows {
            let order_id: i64 = row
                .try_get("id")
                .map_err(|err| map_sqlx_error("读取采购单ID失败", err))?;
            let status_raw: String = row
                .try_get("status")
                .map_err(|err| map_sqlx_error("读取采购单状态失败", err))?;

            let confirmed_at = row
                .try_get::<Option<DateTime<Utc>>, _>("confirmed_at")
                .map_err(|err| map_sqlx_error("读取确认时间失败", err))?
                .map(|dt| dt.to_rfc3339());
            let voided_at = row
                .try_get::<Option<DateTime<Utc>>, _>("voided_at")
                .map_err(|err| map_sqlx_error("读取作废时间失败", err))?
                .map(|dt| dt.to_rfc3339());
            let created_at = row
                .try_get::<DateTime<Utc>, _>("created_at")
                .map_err(|err| map_sqlx_error("读取创建时间失败", err))?
                .to_rfc3339();
            let updated_at = row
                .try_get::<DateTime<Utc>, _>("updated_at")
                .map_err(|err| map_sqlx_error("读取更新时间失败", err))?
                .to_rfc3339();

            orders.push(PurchaseOrder {
                id: order_id,
                tenant_id: row
                    .try_get("tenant_id")
                    .map_err(|err| map_sqlx_error("读取租户ID失败", err))?,
                biz_no: row
                    .try_get("biz_no")
                    .map_err(|err| map_sqlx_error("读取采购单号失败", err))?,
                supplier_id: row
                    .try_get("supplier_id")
                    .map_err(|err| map_sqlx_error("读取供应商ID失败", err))?,
                status: parse_purchase_order_status(&status_raw)?,
                items: items_by_order.remove(&order_id).unwrap_or_default(),
                remark: row
                    .try_get("remark")
                    .map_err(|err| map_sqlx_error("读取备注失败", err))?,
                created_by: row
                    .try_get("created_by")
                    .map_err(|err| map_sqlx_error("读取创建人失败", err))?,
                version: row
                    .try_get("version")
                    .map_err(|err| map_sqlx_error("读取版本失败", err))?,
                confirmed_at,
                voided_at,
                created_at,
                updated_at,
            });
        }

        Ok(orders)
    }

    pub async fn list_stock_logs_by_tenant(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
    ) -> Result<Vec<StockLog>, AppError> {
        let pool = require_pool(pool)?;
        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, product_id, biz_type, biz_no,
                   delta_qty, snapshot_stock, snapshot_cost, snapshot_sell_price, snapshot_inbound_unit_cost, operator_id, created_at
            FROM stock_logs
            WHERE tenant_id = $1
            ORDER BY id ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .map_err(|err| map_sqlx_error("查询库存流水列表失败", err))?;

        let mut logs = Vec::with_capacity(rows.len());
        for row in rows {
            logs.push(StockLog {
                id: row
                    .try_get("id")
                    .map_err(|err| map_sqlx_error("读取库存流水ID失败", err))?,
                tenant_id: row
                    .try_get("tenant_id")
                    .map_err(|err| map_sqlx_error("读取租户ID失败", err))?,
                product_id: row
                    .try_get("product_id")
                    .map_err(|err| map_sqlx_error("读取商品ID失败", err))?,
                biz_type: row
                    .try_get("biz_type")
                    .map_err(|err| map_sqlx_error("读取业务类型失败", err))?,
                biz_no: row
                    .try_get("biz_no")
                    .map_err(|err| map_sqlx_error("读取业务单号失败", err))?,
                delta_qty: row
                    .try_get("delta_qty")
                    .map_err(|err| map_sqlx_error("读取数量变化失败", err))?,
                snapshot_stock: row
                    .try_get("snapshot_stock")
                    .map_err(|err| map_sqlx_error("读取快照库存失败", err))?,
                snapshot_cost: row
                    .try_get("snapshot_cost")
                    .map_err(|err| map_sqlx_error("读取快照成本失败", err))?,
                snapshot_sell_price: row
                    .try_get("snapshot_sell_price")
                    .map_err(|err| map_sqlx_error("读取快照售价失败", err))?,
                snapshot_inbound_unit_cost: row
                    .try_get("snapshot_inbound_unit_cost")
                    .map_err(|err| map_sqlx_error("读取入库进价快照失败", err))?,
                operator_id: row
                    .try_get("operator_id")
                    .map_err(|err| map_sqlx_error("读取操作人失败", err))?,
                created_at: row
                    .try_get::<DateTime<Utc>, _>("created_at")
                    .map_err(|err| map_sqlx_error("读取创建时间失败", err))?
                    .to_rfc3339(),
            });
        }

        Ok(logs)
    }

    pub async fn list_audit_logs_by_tenant(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
    ) -> Result<Vec<AuditLog>, AppError> {
        let pool = require_pool(pool)?;
        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, operator_id, action, target_type,
                   target_id, before_data, after_data, request_id, created_at
            FROM audit_logs
            WHERE tenant_id = $1
            ORDER BY id DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .map_err(|err| map_sqlx_error("查询审计日志列表失败", err))?;

        let mut logs = Vec::with_capacity(rows.len());
        for row in rows {
            logs.push(AuditLog {
                id: row
                    .try_get("id")
                    .map_err(|err| map_sqlx_error("读取审计日志ID失败", err))?,
                tenant_id: row
                    .try_get("tenant_id")
                    .map_err(|err| map_sqlx_error("读取租户ID失败", err))?,
                operator_id: row
                    .try_get("operator_id")
                    .map_err(|err| map_sqlx_error("读取操作人失败", err))?,
                action: row
                    .try_get("action")
                    .map_err(|err| map_sqlx_error("读取动作失败", err))?,
                target_type: row
                    .try_get("target_type")
                    .map_err(|err| map_sqlx_error("读取目标类型失败", err))?,
                target_id: row
                    .try_get("target_id")
                    .map_err(|err| map_sqlx_error("读取目标ID失败", err))?,
                before_data: row
                    .try_get("before_data")
                    .map_err(|err| map_sqlx_error("读取变更前快照失败", err))?,
                after_data: row
                    .try_get("after_data")
                    .map_err(|err| map_sqlx_error("读取变更后快照失败", err))?,
                request_id: row
                    .try_get("request_id")
                    .map_err(|err| map_sqlx_error("读取请求ID失败", err))?,
                created_at: row
                    .try_get::<DateTime<Utc>, _>("created_at")
                    .map_err(|err| map_sqlx_error("读取创建时间失败", err))?
                    .to_rfc3339(),
            });
        }

        Ok(logs)
    }

    pub async fn find_product_by_id(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        product_id: i64,
        include_deleted: bool,
    ) -> Result<Option<Product>, AppError> {
        let pool = require_pool(pool)?;
        let row = if include_deleted {
            sqlx::query(
                r#"
                SELECT id, tenant_id, sku, barcode, name, unit,
                       current_stock, cost_price, retail_price, last_inbound_unit_cost,
                       min_stock_limit, version, is_deleted
                FROM products
                WHERE tenant_id = $1 AND id = $2
                LIMIT 1
                "#,
            )
            .bind(tenant_id)
            .bind(product_id)
            .fetch_optional(pool)
            .await
            .map_err(|err| map_sqlx_error("按ID查询商品失败", err))?
        } else {
            sqlx::query(
                r#"
                SELECT id, tenant_id, sku, barcode, name, unit,
                       current_stock, cost_price, retail_price, last_inbound_unit_cost,
                       min_stock_limit, version, is_deleted
                FROM products
                WHERE tenant_id = $1 AND id = $2 AND is_deleted = FALSE
                LIMIT 1
                "#,
            )
            .bind(tenant_id)
            .bind(product_id)
            .fetch_optional(pool)
            .await
            .map_err(|err| map_sqlx_error("按ID查询商品失败", err))?
        };

        row.map(map_product_row).transpose()
    }

    pub async fn is_barcode_taken(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        barcode: &str,
        exclude_product_id: Option<i64>,
    ) -> Result<Option<i64>, AppError> {
        let pool = require_pool(pool)?;
        let row = if let Some(exclude_id) = exclude_product_id {
            sqlx::query(
                r#"
                SELECT id
                FROM products
                WHERE tenant_id = $1
                  AND barcode = $2
                  AND is_deleted = FALSE
                  AND id <> $3
                LIMIT 1
                "#,
            )
            .bind(tenant_id)
            .bind(barcode)
            .bind(exclude_id)
            .fetch_optional(pool)
            .await
            .map_err(|err| map_sqlx_error("检查条码唯一性失败", err))?
        } else {
            sqlx::query(
                r#"
                SELECT id
                FROM products
                WHERE tenant_id = $1
                  AND barcode = $2
                  AND is_deleted = FALSE
                LIMIT 1
                "#,
            )
            .bind(tenant_id)
            .bind(barcode)
            .fetch_optional(pool)
            .await
            .map_err(|err| map_sqlx_error("检查条码唯一性失败", err))?
        };

        row.map(|r| {
            r.try_get("id")
                .map_err(|err| map_sqlx_error("读取重复条码商品ID失败", err))
        })
        .transpose()
    }

    pub async fn is_sku_taken(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        sku: &str,
        exclude_product_id: Option<i64>,
    ) -> Result<bool, AppError> {
        let pool = require_pool(pool)?;
        let row = if let Some(exclude_id) = exclude_product_id {
            sqlx::query(
                r#"
                SELECT 1
                FROM products
                WHERE tenant_id = $1
                  AND LOWER(sku) = LOWER($2)
                  AND is_deleted = FALSE
                  AND id <> $3
                LIMIT 1
                "#,
            )
            .bind(tenant_id)
            .bind(sku)
            .bind(exclude_id)
            .fetch_optional(pool)
            .await
            .map_err(|err| map_sqlx_error("检查SKU唯一性失败", err))?
        } else {
            sqlx::query(
                r#"
                SELECT 1
                FROM products
                WHERE tenant_id = $1
                  AND LOWER(sku) = LOWER($2)
                  AND is_deleted = FALSE
                LIMIT 1
                "#,
            )
            .bind(tenant_id)
            .bind(sku)
            .fetch_optional(pool)
            .await
            .map_err(|err| map_sqlx_error("检查SKU唯一性失败", err))?
        };

        Ok(row.is_some())
    }

    pub async fn next_product_id(&self, pool: Option<&PgPool>) -> Result<i64, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query("SELECT COALESCE(MAX(id), 2000) + 1 AS next_id FROM products")
            .fetch_one(pool)
            .await
            .map_err(|err| map_sqlx_error("生成商品ID失败", err))?;

        row.try_get("next_id")
            .map_err(|err| map_sqlx_error("读取商品ID失败", err))
    }

    pub async fn create_product(
        &self,
        pool: Option<&PgPool>,
        product: &Product,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        sqlx::query(
            r#"
            INSERT INTO products (
                id, tenant_id, sku, barcode, name, unit,
                current_stock, cost_price, retail_price, last_inbound_unit_cost,
                min_stock_limit, version, is_deleted, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10,
                $11, $12, $13, NOW(), NOW()
            )
            "#,
        )
        .bind(product.id)
        .bind(product.tenant_id)
        .bind(&product.sku)
        .bind(&product.barcode)
        .bind(&product.name)
        .bind(&product.unit)
        .bind(product.current_stock)
        .bind(product.cost_price)
        .bind(product.retail_price)
        .bind(product.last_inbound_unit_cost)
        .bind(product.min_stock_limit)
        .bind(product.version)
        .bind(product.is_deleted)
        .execute(pool)
        .await
        .map_err(|err| map_sqlx_error("创建商品失败", err))?;

        Ok(())
    }

    pub async fn update_product(
        &self,
        pool: Option<&PgPool>,
        product: &Product,
        expected_version: Option<i32>,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        let result = if let Some(ev) = expected_version {
            sqlx::query(
                r#"
                UPDATE products
                SET sku = $1,
                    barcode = $2,
                    name = $3,
                    unit = $4,
                    current_stock = $5,
                    cost_price = $6,
                    retail_price = $7,
                    last_inbound_unit_cost = $8,
                    min_stock_limit = $9,
                    version = $10,
                    is_deleted = $11,
                    updated_at = NOW()
                WHERE id = $12
                  AND tenant_id = $13
                  AND version = $14
                "#,
            )
            .bind(&product.sku)
            .bind(&product.barcode)
            .bind(&product.name)
            .bind(&product.unit)
            .bind(product.current_stock)
            .bind(product.cost_price)
            .bind(product.retail_price)
            .bind(product.last_inbound_unit_cost)
            .bind(product.min_stock_limit)
            .bind(product.version)
            .bind(product.is_deleted)
            .bind(product.id)
            .bind(product.tenant_id)
            .bind(ev)
            .execute(pool)
            .await
            .map_err(|err| map_sqlx_error("更新商品失败", err))?
        } else {
            sqlx::query(
                r#"
                UPDATE products
                SET sku = $1,
                    barcode = $2,
                    name = $3,
                    unit = $4,
                    current_stock = $5,
                    cost_price = $6,
                    retail_price = $7,
                    last_inbound_unit_cost = $8,
                    min_stock_limit = $9,
                    version = $10,
                    is_deleted = $11,
                    updated_at = NOW()
                WHERE id = $12
                  AND tenant_id = $13
                "#,
            )
            .bind(&product.sku)
            .bind(&product.barcode)
            .bind(&product.name)
            .bind(&product.unit)
            .bind(product.current_stock)
            .bind(product.cost_price)
            .bind(product.retail_price)
            .bind(product.last_inbound_unit_cost)
            .bind(product.min_stock_limit)
            .bind(product.version)
            .bind(product.is_deleted)
            .bind(product.id)
            .bind(product.tenant_id)
            .execute(pool)
            .await
            .map_err(|err| map_sqlx_error("更新商品失败", err))?
        };

        if result.rows_affected() == 0 {
            if let Some(ev) = expected_version
                && let Some(latest) = self
                    .find_product_by_id(Some(pool), product.tenant_id, product.id, true)
                    .await?
            {
                if latest.is_deleted {
                    return Err(AppError::not_found("商品不存在"));
                }

                return Err(
                    AppError::conflict(4091, "版本冲突，请刷新后重试").with_data(json!({
                        "resource": "product",
                        "resource_id": latest.id,
                        "expected_version": ev,
                        "current_version": latest.version,
                        "latest_snapshot": product_snapshot(&latest)
                    })),
                );
            }

            return Err(AppError::not_found("商品不存在"));
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn inbound(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        product_id: i64,
        qty: i32,
        unit_cost: Option<Decimal>,
        expected_version: Option<i32>,
        biz_no: &str,
        operator_id: Uuid,
    ) -> Result<Product, AppError> {
        if qty <= 0 {
            return Err(AppError::bad_request("qty 必须大于 0"));
        }

        let pool = require_pool(pool)?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|err| map_sqlx_error("入库事务开启失败", err))?;

        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, sku, barcode, name, unit,
                   current_stock, cost_price, retail_price, last_inbound_unit_cost,
                   min_stock_limit, version, is_deleted
            FROM products
            WHERE tenant_id = $1 AND id = $2
            FOR UPDATE
            "#,
        )
        .bind(tenant_id)
        .bind(product_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("锁定商品失败", err))?
        .ok_or_else(|| AppError::not_found("商品不存在"))?;

        let mut existing = map_product_row(row)?;
        if existing.is_deleted {
            return Err(AppError::not_found("商品不存在"));
        }

        if let Some(ev) = expected_version
            && ev != existing.version
        {
            return Err(
                AppError::conflict(4091, "版本冲突，请刷新后重试").with_data(json!({
                    "resource": "product",
                    "resource_id": existing.id,
                    "expected_version": ev,
                    "current_version": existing.version,
                    "latest_snapshot": product_snapshot(&existing)
                })),
            );
        }

        let old_stock = existing.current_stock;
        let old_cost = existing.cost_price;
        let effective_unit_cost = unit_cost
            .or(existing.last_inbound_unit_cost)
            .unwrap_or(old_cost);
        let new_stock = old_stock + qty;
        let new_cost = if new_stock <= 0 {
            effective_unit_cost.round_dp(4)
        } else {
            let old_total = old_cost * Decimal::from(old_stock);
            let in_total = effective_unit_cost * Decimal::from(qty);
            ((old_total + in_total) / Decimal::from(new_stock)).round_dp(4)
        };

        existing.current_stock = new_stock;
        existing.cost_price = new_cost;
        existing.last_inbound_unit_cost = Some(effective_unit_cost.round_dp(4));
        existing.version += 1;

        sqlx::query(
            r#"
            UPDATE products
            SET current_stock = $1,
                cost_price = $2,
                last_inbound_unit_cost = $3,
                version = $4,
                updated_at = NOW()
            WHERE tenant_id = $5 AND id = $6
            "#,
        )
        .bind(existing.current_stock)
        .bind(existing.cost_price)
        .bind(existing.last_inbound_unit_cost)
        .bind(existing.version)
        .bind(tenant_id)
        .bind(product_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("更新商品库存失败", err))?;

        let stock_log_id = self.next_stock_log_id_in_tx(&mut tx).await?;
        sqlx::query(
            r#"
            INSERT INTO stock_logs (
                id, tenant_id, product_id, biz_type, biz_no,
                delta_qty, snapshot_stock, snapshot_cost, snapshot_sell_price, snapshot_inbound_unit_cost, operator_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
            "#,
        )
        .bind(stock_log_id)
        .bind(tenant_id)
        .bind(existing.id)
        .bind("IN_PURCHASE")
        .bind(biz_no)
        .bind(qty)
        .bind(existing.current_stock)
        .bind(existing.cost_price)
        .bind(Option::<Decimal>::None)
        .bind(Some(effective_unit_cost))
        .bind(operator_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("写入库存流水失败", err))?;

        tx.commit()
            .await
            .map_err(|err| map_sqlx_error("入库事务提交失败", err))?;

        Ok(existing)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn inbound_batch(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        biz_no: &str,
        items: &[(i64, i32, Option<Decimal>, Option<i32>)],
        operator_id: Uuid,
    ) -> Result<Vec<Product>, AppError> {
        if items.is_empty() {
            return Err(AppError::bad_request("items 不能为空"));
        }

        let pool = require_pool(pool)?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|err| map_sqlx_error("批量入库事务开启失败", err))?;

        let mut next_stock_log_id = self.next_stock_log_id_in_tx(&mut tx).await?;
        let mut updated_products = Vec::with_capacity(items.len());

        for &(product_id, qty, unit_cost, expected_version) in items {
            if qty <= 0 {
                return Err(AppError::bad_request("items.qty 必须大于 0"));
            }

            let row = sqlx::query(
                r#"
                SELECT id, tenant_id, sku, barcode, name, unit,
                       current_stock, cost_price, retail_price, last_inbound_unit_cost,
                       min_stock_limit, version, is_deleted
                FROM products
                WHERE tenant_id = $1 AND id = $2
                FOR UPDATE
                "#,
            )
            .bind(tenant_id)
            .bind(product_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("锁定商品失败", err))?
            .ok_or_else(|| AppError::not_found("商品不存在"))?;

            let mut existing = map_product_row(row)?;
            if existing.is_deleted {
                return Err(AppError::not_found("商品不存在"));
            }

            if let Some(ev) = expected_version
                && ev != existing.version
            {
                return Err(
                    AppError::conflict(4091, "版本冲突，请刷新后重试").with_data(json!({
                        "resource": "product",
                        "resource_id": existing.id,
                        "expected_version": ev,
                        "current_version": existing.version,
                        "latest_snapshot": product_snapshot(&existing)
                    })),
                );
            }

            let old_stock = existing.current_stock;
            let old_cost = existing.cost_price;
            let effective_unit_cost = unit_cost
                .or(existing.last_inbound_unit_cost)
                .unwrap_or(old_cost);
            let new_stock = old_stock + qty;
            let new_cost = if new_stock <= 0 {
                effective_unit_cost.round_dp(4)
            } else {
                let old_total = old_cost * Decimal::from(old_stock);
                let in_total = effective_unit_cost * Decimal::from(qty);
                ((old_total + in_total) / Decimal::from(new_stock)).round_dp(4)
            };

            existing.current_stock = new_stock;
            existing.cost_price = new_cost;
            existing.last_inbound_unit_cost = Some(effective_unit_cost.round_dp(4));
            existing.version += 1;

            sqlx::query(
                r#"
                UPDATE products
                SET current_stock = $1,
                    cost_price = $2,
                    last_inbound_unit_cost = $3,
                    version = $4,
                    updated_at = NOW()
                WHERE tenant_id = $5 AND id = $6
                "#,
            )
            .bind(existing.current_stock)
            .bind(existing.cost_price)
            .bind(existing.last_inbound_unit_cost)
            .bind(existing.version)
            .bind(tenant_id)
            .bind(product_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("更新商品库存失败", err))?;

            sqlx::query(
                r#"
                INSERT INTO stock_logs (
                    id, tenant_id, product_id, biz_type, biz_no,
                    delta_qty, snapshot_stock, snapshot_cost, snapshot_sell_price, snapshot_inbound_unit_cost, operator_id, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
                "#,
            )
            .bind(next_stock_log_id)
            .bind(tenant_id)
            .bind(existing.id)
            .bind("IN_PURCHASE")
            .bind(biz_no)
            .bind(qty)
            .bind(existing.current_stock)
            .bind(existing.cost_price)
            .bind(Option::<Decimal>::None)
            .bind(Some(effective_unit_cost))
            .bind(operator_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("写入库存流水失败", err))?;

            next_stock_log_id += 1;
            updated_products.push(existing);
        }

        tx.commit()
            .await
            .map_err(|err| map_sqlx_error("批量入库事务提交失败", err))?;

        Ok(updated_products)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn outbound(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        biz_no: &str,
        default_expected_version: Option<i32>,
        items: &[(i64, i32, Option<i32>, Option<Decimal>)],
        allow_negative_stock: bool,
        operator_id: Uuid,
    ) -> Result<Vec<Product>, AppError> {
        if items.is_empty() {
            return Err(AppError::bad_request("items 不能为空"));
        }

        let pool = require_pool(pool)?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|err| map_sqlx_error("出库事务开启失败", err))?;

        let mut next_stock_log_id = self.next_stock_log_id_in_tx(&mut tx).await?;
        let mut updated_products = Vec::with_capacity(items.len());

        for &(product_id, qty, item_expected_version, snapshot_sell_price) in items {
            if qty <= 0 {
                return Err(AppError::bad_request("items.qty 必须大于 0"));
            }

            let row = sqlx::query(
                r#"
                SELECT id, tenant_id, sku, barcode, name, unit,
                       current_stock, cost_price, retail_price, last_inbound_unit_cost,
                       min_stock_limit, version, is_deleted
                FROM products
                WHERE tenant_id = $1 AND id = $2
                FOR UPDATE
                "#,
            )
            .bind(tenant_id)
            .bind(product_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("锁定商品失败", err))?
            .ok_or_else(|| AppError::not_found("商品不存在"))?;

            let mut existing = map_product_row(row)?;
            if existing.is_deleted {
                return Err(AppError::not_found("商品不存在"));
            }

            let expected_version = item_expected_version.or(default_expected_version);
            if let Some(ev) = expected_version
                && ev != existing.version
            {
                return Err(
                    AppError::conflict(4091, "版本冲突，请刷新后重试").with_data(json!({
                        "resource": "product",
                        "resource_id": existing.id,
                        "expected_version": ev,
                        "current_version": existing.version,
                        "latest_snapshot": product_snapshot(&existing)
                    })),
                );
            }

            if !allow_negative_stock && existing.current_stock < qty {
                return Err(
                    AppError::business(StatusCode::BAD_REQUEST, 4001, "库存不足").with_data(
                        json!({
                            "failed_product_id": existing.id,
                            "available_stock": existing.current_stock,
                            "required_qty": qty
                        }),
                    ),
                );
            }

            existing.current_stock -= qty;
            existing.version += 1;

            sqlx::query(
                r#"
                UPDATE products
                SET current_stock = $1,
                    version = $2,
                    updated_at = NOW()
                WHERE tenant_id = $3 AND id = $4
                "#,
            )
            .bind(existing.current_stock)
            .bind(existing.version)
            .bind(tenant_id)
            .bind(existing.id)
            .execute(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("更新商品库存失败", err))?;

            sqlx::query(
                r#"
                INSERT INTO stock_logs (
                    id, tenant_id, product_id, biz_type, biz_no,
                    delta_qty, snapshot_stock, snapshot_cost, snapshot_sell_price, snapshot_inbound_unit_cost, operator_id, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
                "#,
            )
            .bind(next_stock_log_id)
            .bind(tenant_id)
            .bind(existing.id)
            .bind("OUT_SALE")
            .bind(biz_no)
            .bind(-qty)
            .bind(existing.current_stock)
            .bind(existing.cost_price)
            .bind(snapshot_sell_price)
            .bind(Option::<Decimal>::None)
            .bind(operator_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("写入库存流水失败", err))?;

            next_stock_log_id += 1;
            updated_products.push(existing);
        }

        tx.commit()
            .await
            .map_err(|err| map_sqlx_error("出库事务提交失败", err))?;

        Ok(updated_products)
    }

    pub async fn load_idempotency_record(
        &self,
        pool: Option<&PgPool>,
        scope_key: &str,
    ) -> Result<Option<IdempotencyRecord>, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query(
            r#"
            SELECT request_payload, response_body
            FROM idempotency_records
            WHERE scope_key = $1
            LIMIT 1
            "#,
        )
        .bind(scope_key)
        .fetch_optional(pool)
        .await
        .map_err(|err| map_sqlx_error("读取幂等记录失败", err))?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(IdempotencyRecord {
            request_payload: row
                .try_get("request_payload")
                .map_err(|err| map_sqlx_error("读取幂等请求体失败", err))?,
            response_body: row
                .try_get("response_body")
                .map_err(|err| map_sqlx_error("读取幂等响应体失败", err))?,
        }))
    }

    pub async fn save_idempotency_record(
        &self,
        pool: Option<&PgPool>,
        scope_key: &str,
        request_payload: &Value,
        response_body: &Value,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        sqlx::query(
            r#"
            INSERT INTO idempotency_records(scope_key, request_payload, response_body, created_at, updated_at)
            VALUES ($1, $2, $3, NOW(), NOW())
            ON CONFLICT (scope_key)
            DO UPDATE SET request_payload = EXCLUDED.request_payload,
                          response_body = EXCLUDED.response_body,
                          updated_at = NOW()
            "#,
        )
        .bind(scope_key)
        .bind(request_payload)
        .bind(response_body)
        .execute(pool)
        .await
        .map_err(|err| map_sqlx_error("写入幂等记录失败", err))?;

        Ok(())
    }

    pub async fn next_purchase_order_id(&self, pool: Option<&PgPool>) -> Result<i64, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query("SELECT COALESCE(MAX(id), 3000) + 1 AS next_id FROM purchase_orders")
            .fetch_one(pool)
            .await
            .map_err(|err| map_sqlx_error("生成采购单ID失败", err))?;

        row.try_get("next_id")
            .map_err(|err| map_sqlx_error("读取采购单ID失败", err))
    }

    pub async fn is_purchase_order_biz_no_taken(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        biz_no: &str,
    ) -> Result<bool, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query(
            r#"
            SELECT 1
            FROM purchase_orders
            WHERE tenant_id = $1 AND biz_no = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(biz_no)
        .fetch_optional(pool)
        .await
        .map_err(|err| map_sqlx_error("检查采购单号唯一性失败", err))?;

        Ok(row.is_some())
    }

    pub async fn create_purchase_order(
        &self,
        pool: Option<&PgPool>,
        order: &PurchaseOrder,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|err| map_sqlx_error("创建采购单事务开启失败", err))?;

        let main_insert = sqlx::query(
            r#"
            INSERT INTO purchase_orders (
                id, tenant_id, biz_no, supplier_id,
                status, remark, created_by,
                version, confirmed_at, voided_at,
                created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4,
                $5, $6, $7,
                $8, $9, $10,
                $11, $12
            )
            "#,
        )
        .bind(order.id)
        .bind(order.tenant_id)
        .bind(&order.biz_no)
        .bind(order.supplier_id)
        .bind(order.status.as_str())
        .bind(&order.remark)
        .bind(order.created_by)
        .bind(order.version)
        .bind(parse_optional_datetime(order.confirmed_at.as_deref())?)
        .bind(parse_optional_datetime(order.voided_at.as_deref())?)
        .bind(parse_datetime(&order.created_at)?)
        .bind(parse_datetime(&order.updated_at)?)
        .execute(&mut *tx)
        .await;

        if let Err(err) = main_insert {
            if is_sql_state(&err, "23505") {
                return Err(AppError::conflict(4090, "采购单号已存在")
                    .with_data(json!({ "biz_no": order.biz_no })));
            }
            return Err(map_sqlx_error("创建采购单主表失败", err));
        }

        for item in &order.items {
            let item_insert = sqlx::query(
                r#"
                INSERT INTO purchase_order_items (
                    tenant_id, purchase_order_id, product_id, qty, unit_cost, product_name_snapshot, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, NOW())
                "#,
            )
            .bind(order.tenant_id)
            .bind(order.id)
            .bind(item.product_id)
            .bind(item.qty)
            .bind(item.unit_cost)
            .bind(&item.product_name_snapshot)
            .execute(&mut *tx)
            .await;

            if let Err(err) = item_insert {
                if is_sql_state(&err, "23503") {
                    return Err(AppError::not_found("商品不存在"));
                }
                return Err(map_sqlx_error("写入采购单明细失败", err));
            }
        }

        tx.commit()
            .await
            .map_err(|err| map_sqlx_error("提交采购单事务失败", err))?;

        Ok(())
    }

    pub async fn find_purchase_order_by_id(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        order_id: i64,
    ) -> Result<Option<PurchaseOrder>, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, biz_no, supplier_id,
                   status, remark, created_by,
                   version, confirmed_at, voided_at,
                   created_at, updated_at
            FROM purchase_orders
            WHERE tenant_id = $1 AND id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(order_id)
        .fetch_optional(pool)
        .await
        .map_err(|err| map_sqlx_error("查询采购单失败", err))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let status_raw: String = row
            .try_get("status")
            .map_err(|err| map_sqlx_error("读取采购单状态失败", err))?;

        let items_rows = sqlx::query(
            r#"
            SELECT product_id, qty, unit_cost, product_name_snapshot
            FROM purchase_order_items
            WHERE tenant_id = $1 AND purchase_order_id = $2
            ORDER BY id ASC
            "#,
        )
        .bind(tenant_id)
        .bind(order_id)
        .fetch_all(pool)
        .await
        .map_err(|err| map_sqlx_error("查询采购单明细失败", err))?;

        let mut items = Vec::with_capacity(items_rows.len());
        for item_row in items_rows {
            items.push(PurchaseOrderItem {
                product_id: item_row
                    .try_get("product_id")
                    .map_err(|err| map_sqlx_error("读取采购明细商品ID失败", err))?,
                qty: item_row
                    .try_get("qty")
                    .map_err(|err| map_sqlx_error("读取采购明细数量失败", err))?,
                unit_cost: item_row
                    .try_get("unit_cost")
                    .map_err(|err| map_sqlx_error("读取采购明细单价失败", err))?,
                product_name_snapshot: item_row
                    .try_get("product_name_snapshot")
                    .map_err(|err| map_sqlx_error("读取采购明细商品名称快照失败", err))?,
            });
        }

        let confirmed_at = row
            .try_get::<Option<DateTime<Utc>>, _>("confirmed_at")
            .map_err(|err| map_sqlx_error("读取确认时间失败", err))?
            .map(|dt| dt.to_rfc3339());
        let voided_at = row
            .try_get::<Option<DateTime<Utc>>, _>("voided_at")
            .map_err(|err| map_sqlx_error("读取作废时间失败", err))?
            .map(|dt| dt.to_rfc3339());

        let created_at = row
            .try_get::<DateTime<Utc>, _>("created_at")
            .map_err(|err| map_sqlx_error("读取创建时间失败", err))?
            .to_rfc3339();
        let updated_at = row
            .try_get::<DateTime<Utc>, _>("updated_at")
            .map_err(|err| map_sqlx_error("读取更新时间失败", err))?
            .to_rfc3339();

        Ok(Some(PurchaseOrder {
            id: row
                .try_get("id")
                .map_err(|err| map_sqlx_error("读取采购单ID失败", err))?,
            tenant_id: row
                .try_get("tenant_id")
                .map_err(|err| map_sqlx_error("读取采购单租户失败", err))?,
            biz_no: row
                .try_get("biz_no")
                .map_err(|err| map_sqlx_error("读取采购单号失败", err))?,
            supplier_id: row
                .try_get("supplier_id")
                .map_err(|err| map_sqlx_error("读取供应商ID失败", err))?,
            status: parse_purchase_order_status(&status_raw)?,
            items,
            remark: row
                .try_get("remark")
                .map_err(|err| map_sqlx_error("读取备注失败", err))?,
            created_by: row
                .try_get("created_by")
                .map_err(|err| map_sqlx_error("读取创建人失败", err))?,
            version: row
                .try_get("version")
                .map_err(|err| map_sqlx_error("读取版本失败", err))?,
            confirmed_at,
            voided_at,
            created_at,
            updated_at,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn confirm_purchase_order(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        order_id: i64,
        expected_version: Option<i32>,
        operator_id: Uuid,
        request_id: &str,
    ) -> Result<PurchaseOrder, AppError> {
        let pool = require_pool(pool)?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|err| map_sqlx_error("确认采购单事务开启失败", err))?;

        let existing = self
            .find_purchase_order_by_id_for_update(&mut tx, tenant_id, order_id)
            .await?
            .ok_or_else(|| AppError::not_found("采购单不存在"))?;

        if let Some(ev) = expected_version
            && ev != existing.version
        {
            return Err(
                AppError::conflict(4091, "版本冲突，请刷新后重试").with_data(json!({
                    "resource": "purchase_order",
                    "resource_id": existing.id,
                    "expected_version": ev,
                    "current_version": existing.version,
                    "latest_snapshot": purchase_order_snapshot(&existing)
                })),
            );
        }

        if existing.status != PurchaseOrderStatus::Draft {
            return Err(AppError::conflict(4090, "仅 DRAFT 状态可确认采购单")
                .with_data(json!({ "status": existing.status.as_str() })));
        }

        let mut next_stock_log_id = self.next_stock_log_id_in_tx(&mut tx).await?;

        for item in &existing.items {
            let product_row = sqlx::query(
                r#"
                SELECT id, current_stock, cost_price, version, is_deleted
                FROM products
                WHERE tenant_id = $1 AND id = $2
                FOR UPDATE
                "#,
            )
            .bind(tenant_id)
            .bind(item.product_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("锁定商品失败", err))?
            .ok_or_else(|| AppError::not_found("商品不存在"))?;

            let is_deleted: bool = product_row
                .try_get("is_deleted")
                .map_err(|err| map_sqlx_error("读取商品删除标记失败", err))?;
            if is_deleted {
                return Err(AppError::not_found("商品不存在"));
            }

            let product_id: i64 = product_row
                .try_get("id")
                .map_err(|err| map_sqlx_error("读取商品ID失败", err))?;
            let old_stock: i32 = product_row
                .try_get("current_stock")
                .map_err(|err| map_sqlx_error("读取库存失败", err))?;
            let old_cost: Decimal = product_row
                .try_get("cost_price")
                .map_err(|err| map_sqlx_error("读取成本价失败", err))?;
            let old_version: i32 = product_row
                .try_get("version")
                .map_err(|err| map_sqlx_error("读取商品版本失败", err))?;

            let new_stock = old_stock + item.qty;
            let new_cost = if new_stock <= 0 {
                item.unit_cost.round_dp(4)
            } else {
                let old_total = old_cost * Decimal::from(old_stock);
                let in_total = item.unit_cost * Decimal::from(item.qty);
                ((old_total + in_total) / Decimal::from(new_stock)).round_dp(4)
            };

            sqlx::query(
                r#"
                UPDATE products
                SET current_stock = $1,
                    cost_price = $2,
                    last_inbound_unit_cost = $3,
                    version = $4,
                    updated_at = NOW()
                WHERE tenant_id = $5 AND id = $6
                "#,
            )
            .bind(new_stock)
            .bind(new_cost)
            .bind(Some(item.unit_cost.round_dp(4)))
            .bind(old_version + 1)
            .bind(tenant_id)
            .bind(product_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("更新商品库存失败", err))?;

            sqlx::query(
                r#"
                INSERT INTO stock_logs (
                    id, tenant_id, product_id, biz_type, biz_no,
                    delta_qty, snapshot_stock, snapshot_cost, snapshot_sell_price, snapshot_inbound_unit_cost, operator_id, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
                "#,
            )
            .bind(next_stock_log_id)
            .bind(tenant_id)
            .bind(product_id)
            .bind("IN_PURCHASE")
            .bind(&existing.biz_no)
            .bind(item.qty)
            .bind(new_stock)
            .bind(new_cost)
            .bind(Option::<Decimal>::None)
            .bind(Some(item.unit_cost))
            .bind(operator_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("写入库存流水失败", err))?;

            next_stock_log_id += 1;
        }

        let now = Utc::now();
        let now_rfc3339 = now.to_rfc3339();
        let mut updated = existing.clone();
        updated.status = PurchaseOrderStatus::Confirmed;
        updated.version += 1;
        updated.confirmed_at = Some(now_rfc3339.clone());
        updated.updated_at = now_rfc3339.clone();

        sqlx::query(
            r#"
            UPDATE purchase_orders
            SET status = 'CONFIRMED',
                version = $1,
                confirmed_at = $2,
                updated_at = $2
            WHERE tenant_id = $3 AND id = $4
            "#,
        )
        .bind(updated.version)
        .bind(now)
        .bind(tenant_id)
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("更新采购单状态失败", err))?;

        let audit_log_id = self.next_audit_log_id_in_tx(&mut tx).await?;
        sqlx::query(
            r#"
            INSERT INTO audit_logs (
                id, tenant_id, operator_id, action, target_type,
                target_id, before_data, after_data, request_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            "#,
        )
        .bind(audit_log_id)
        .bind(tenant_id)
        .bind(operator_id)
        .bind("PURCHASE_ORDER_CONFIRM")
        .bind("purchase_order")
        .bind(order_id.to_string())
        .bind(purchase_order_snapshot(&existing))
        .bind(purchase_order_snapshot(&updated))
        .bind(request_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("写入审计日志失败", err))?;

        tx.commit()
            .await
            .map_err(|err| map_sqlx_error("确认采购单事务提交失败", err))?;

        Ok(updated)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn void_sales_order(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        order_id: i64,
        expected_version: Option<i32>,
        operator_id: Uuid,
        request_id: &str,
    ) -> Result<SalesOrder, AppError> {
        let pool = require_pool(pool)?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|err| map_sqlx_error("作废销售单事务开启失败", err))?;

        let existing = self
            .find_sales_order_by_id_for_update(&mut tx, tenant_id, order_id)
            .await?
            .ok_or_else(|| AppError::not_found("销售单不存在"))?;

        if let Some(ev) = expected_version
            && ev != existing.version
        {
            return Err(
                AppError::conflict(4091, "版本冲突，请刷新后重试").with_data(json!({
                    "resource": "sales_order",
                    "resource_id": existing.id,
                    "expected_version": ev,
                    "current_version": existing.version,
                    "latest_snapshot": sales_order_snapshot(&existing)
                })),
            );
        }

        if existing.status != SalesOrderStatus::Draft {
            return Err(AppError::conflict(4090, "仅 DRAFT 状态可作废销售单")
                .with_data(json!({ "status": existing.status.as_str() })));
        }

        let now = Utc::now();
        let now_rfc3339 = now.to_rfc3339();
        let mut updated = existing.clone();
        updated.status = SalesOrderStatus::Voided;
        updated.version += 1;
        updated.voided_at = Some(now_rfc3339.clone());
        updated.updated_at = now_rfc3339;

        sqlx::query(
            r#"
            UPDATE sales_orders
            SET status = 'VOIDED',
                version = $1,
                voided_at = $2,
                updated_at = $2
            WHERE tenant_id = $3 AND id = $4
            "#,
        )
        .bind(updated.version)
        .bind(now)
        .bind(tenant_id)
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("更新销售单作废状态失败", err))?;

        let audit_log_id = self.next_audit_log_id_in_tx(&mut tx).await?;
        sqlx::query(
            r#"
            INSERT INTO audit_logs (
                id, tenant_id, operator_id, action, target_type,
                target_id, before_data, after_data, request_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            "#,
        )
        .bind(audit_log_id)
        .bind(tenant_id)
        .bind(operator_id)
        .bind("SALES_ORDER_VOID")
        .bind("sales_order")
        .bind(order_id.to_string())
        .bind(sales_order_snapshot(&existing))
        .bind(sales_order_snapshot(&updated))
        .bind(request_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("写入审计日志失败", err))?;

        tx.commit()
            .await
            .map_err(|err| map_sqlx_error("作废销售单事务提交失败", err))?;

        Ok(updated)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn return_sales_order(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        order_id: i64,
        expected_version: Option<i32>,
        return_items: &[(i64, i32)],
        remark: Option<String>,
        operator_id: Uuid,
        request_id: &str,
    ) -> Result<SalesOrder, AppError> {
        let pool = require_pool(pool)?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|err| map_sqlx_error("销售退货事务开启失败", err))?;

        let existing = self
            .find_sales_order_by_id_for_update(&mut tx, tenant_id, order_id)
            .await?
            .ok_or_else(|| AppError::not_found("销售单不存在"))?;

        if let Some(ev) = expected_version
            && ev != existing.version
        {
            return Err(
                AppError::conflict(4091, "版本冲突，请刷新后重试").with_data(json!({
                    "resource": "sales_order",
                    "resource_id": existing.id,
                    "expected_version": ev,
                    "current_version": existing.version,
                    "latest_snapshot": sales_order_snapshot(&existing)
                })),
            );
        }

        match existing.status {
            SalesOrderStatus::Confirmed | SalesOrderStatus::ReturnedPartial => {}
            _ => {
                return Err(AppError::conflict(4090, "当前状态不允许退货")
                    .with_data(json!({ "status": existing.status.as_str() })));
            }
        }

        if return_items.is_empty() {
            return Err(AppError::bad_request("items 不能为空"));
        }

        let mut pending_returns = Vec::with_capacity(return_items.len());
        for &(product_id, qty) in return_items {
            if qty <= 0 {
                return Err(AppError::bad_request("items.qty 必须大于 0"));
            }

            let order_item = existing
                .items
                .iter()
                .find(|oi| oi.product_id == product_id)
                .ok_or_else(|| {
                    AppError::bad_request("退货商品不在销售单中")
                        .with_data(json!({ "product_id": product_id }))
                })?;
            let remain = order_item.qty - order_item.returned_qty;
            if qty > remain {
                return Err(
                    AppError::conflict(4090, "退货数量超过可退数量").with_data(json!({
                        "product_id": product_id,
                        "remain_qty": remain,
                        "request_qty": qty
                    })),
                );
            }

            pending_returns.push((product_id, qty));
        }

        let mut next_stock_log_id = self.next_stock_log_id_in_tx(&mut tx).await?;
        for &(product_id, qty) in &pending_returns {
            let product_row = sqlx::query(
                r#"
                SELECT id, current_stock, cost_price, version, is_deleted
                FROM products
                WHERE tenant_id = $1 AND id = $2
                FOR UPDATE
                "#,
            )
            .bind(tenant_id)
            .bind(product_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("锁定商品失败", err))?
            .ok_or_else(|| AppError::not_found("商品不存在"))?;

            let is_deleted: bool = product_row
                .try_get("is_deleted")
                .map_err(|err| map_sqlx_error("读取商品删除标记失败", err))?;
            if is_deleted {
                return Err(AppError::not_found("商品不存在"));
            }

            let current_stock: i32 = product_row
                .try_get("current_stock")
                .map_err(|err| map_sqlx_error("读取库存失败", err))?;
            let product_version: i32 = product_row
                .try_get("version")
                .map_err(|err| map_sqlx_error("读取商品版本失败", err))?;
            let snapshot_cost: Decimal = product_row
                .try_get("cost_price")
                .map_err(|err| map_sqlx_error("读取成本价失败", err))?;

            let new_stock = current_stock + qty;

            sqlx::query(
                r#"
                UPDATE products
                SET current_stock = $1,
                    version = $2,
                    updated_at = NOW()
                WHERE tenant_id = $3 AND id = $4
                "#,
            )
            .bind(new_stock)
            .bind(product_version + 1)
            .bind(tenant_id)
            .bind(product_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("更新商品库存失败", err))?;

            sqlx::query(
                r#"
                INSERT INTO stock_logs (
                    id, tenant_id, product_id, biz_type, biz_no,
                    delta_qty, snapshot_stock, snapshot_cost, snapshot_sell_price, snapshot_inbound_unit_cost, operator_id, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
                "#,
            )
            .bind(next_stock_log_id)
            .bind(tenant_id)
            .bind(product_id)
            .bind("RETURN_SALE")
            .bind(&existing.biz_no)
            .bind(qty)
            .bind(new_stock)
            .bind(snapshot_cost)
            .bind(
                existing
                    .items
                    .iter()
                    .find(|item| item.product_id == product_id)
                    .map(|item| item.sell_price),
            )
            .bind(Option::<Decimal>::None)
            .bind(operator_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("写入库存流水失败", err))?;

            next_stock_log_id += 1;
        }

        let mut updated = existing.clone();
        for &(product_id, qty) in &pending_returns {
            if let Some(item) = updated
                .items
                .iter_mut()
                .find(|i| i.product_id == product_id)
            {
                item.returned_qty += qty;
            }
        }

        let is_full = updated.items.iter().all(|i| i.returned_qty >= i.qty);
        updated.status = if is_full {
            SalesOrderStatus::ReturnedFull
        } else {
            SalesOrderStatus::ReturnedPartial
        };
        updated.version += 1;
        if let Some(remark) = remark {
            updated.remark = Some(remark);
        }
        let now = Utc::now();
        let now_rfc3339 = now.to_rfc3339();
        updated.returned_at = Some(now_rfc3339.clone());
        updated.updated_at = now_rfc3339;

        sqlx::query(
            r#"
            UPDATE sales_orders
            SET status = $1,
                version = $2,
                remark = $3,
                returned_at = $4,
                updated_at = $4
            WHERE tenant_id = $5 AND id = $6
            "#,
        )
        .bind(updated.status.as_str())
        .bind(updated.version)
        .bind(&updated.remark)
        .bind(now)
        .bind(tenant_id)
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("更新销售单退货状态失败", err))?;

        for item in &updated.items {
            sqlx::query(
                r#"
                UPDATE sales_order_items
                SET returned_qty = $1
                WHERE tenant_id = $2 AND sales_order_id = $3 AND product_id = $4
                "#,
            )
            .bind(item.returned_qty)
            .bind(tenant_id)
            .bind(order_id)
            .bind(item.product_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("更新销售单明细退货数量失败", err))?;
        }

        let audit_log_id = self.next_audit_log_id_in_tx(&mut tx).await?;
        sqlx::query(
            r#"
            INSERT INTO audit_logs (
                id, tenant_id, operator_id, action, target_type,
                target_id, before_data, after_data, request_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            "#,
        )
        .bind(audit_log_id)
        .bind(tenant_id)
        .bind(operator_id)
        .bind("SALES_ORDER_RETURN")
        .bind("sales_order")
        .bind(order_id.to_string())
        .bind(sales_order_snapshot(&existing))
        .bind(sales_order_snapshot(&updated))
        .bind(request_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("写入审计日志失败", err))?;

        tx.commit()
            .await
            .map_err(|err| map_sqlx_error("销售退货事务提交失败", err))?;

        Ok(updated)
    }

    pub async fn next_stock_check_id(&self, pool: Option<&PgPool>) -> Result<i64, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query("SELECT COALESCE(MAX(id), 5000) + 1 AS next_id FROM stock_checks")
            .fetch_one(pool)
            .await
            .map_err(|err| map_sqlx_error("生成盘点单ID失败", err))?;

        row.try_get("next_id")
            .map_err(|err| map_sqlx_error("读取盘点单ID失败", err))
    }

    pub async fn is_stock_check_biz_no_taken(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        biz_no: &str,
    ) -> Result<bool, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query(
            r#"
            SELECT 1
            FROM stock_checks
            WHERE tenant_id = $1 AND biz_no = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(biz_no)
        .fetch_optional(pool)
        .await
        .map_err(|err| map_sqlx_error("检查盘点单号唯一性失败", err))?;

        Ok(row.is_some())
    }

    pub async fn create_stock_check(
        &self,
        pool: Option<&PgPool>,
        check: &StockCheck,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|err| map_sqlx_error("创建盘点单事务开启失败", err))?;

        let main_insert = sqlx::query(
            r#"
            INSERT INTO stock_checks (
                id, tenant_id, biz_no, status, remark, created_by,
                version, counting_at, confirmed_at,
                created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9,
                $10, $11
            )
            "#,
        )
        .bind(check.id)
        .bind(check.tenant_id)
        .bind(&check.biz_no)
        .bind(check.status.as_str())
        .bind(&check.remark)
        .bind(check.created_by)
        .bind(check.version)
        .bind(parse_optional_datetime(check.counting_at.as_deref())?)
        .bind(parse_optional_datetime(check.confirmed_at.as_deref())?)
        .bind(parse_datetime(&check.created_at)?)
        .bind(parse_datetime(&check.updated_at)?)
        .execute(&mut *tx)
        .await;

        if let Err(err) = main_insert {
            if is_sql_state(&err, "23505") {
                return Err(AppError::conflict(4090, "盘点单号已存在")
                    .with_data(json!({ "biz_no": check.biz_no })));
            }
            return Err(map_sqlx_error("创建盘点单主表失败", err));
        }

        for item in &check.items {
            let item_insert = sqlx::query(
                r#"
                INSERT INTO stock_check_items (
                    tenant_id, stock_check_id, product_id, book_stock, actual_stock, delta_qty, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, NOW())
                "#,
            )
            .bind(check.tenant_id)
            .bind(check.id)
            .bind(item.product_id)
            .bind(item.book_stock)
            .bind(item.actual_stock)
            .bind(item.delta_qty)
            .execute(&mut *tx)
            .await;

            if let Err(err) = item_insert {
                if is_sql_state(&err, "23503") {
                    return Err(AppError::not_found("商品不存在"));
                }
                return Err(map_sqlx_error("写入盘点单明细失败", err));
            }
        }

        tx.commit()
            .await
            .map_err(|err| map_sqlx_error("提交盘点单事务失败", err))?;

        Ok(())
    }

    pub async fn find_stock_check_by_id(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        check_id: i64,
    ) -> Result<Option<StockCheck>, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, biz_no,
                   status, remark, created_by,
                   version, counting_at, confirmed_at,
                   created_at, updated_at
            FROM stock_checks
            WHERE tenant_id = $1 AND id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(check_id)
        .fetch_optional(pool)
        .await
        .map_err(|err| map_sqlx_error("查询盘点单失败", err))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let status_raw: String = row
            .try_get("status")
            .map_err(|err| map_sqlx_error("读取盘点单状态失败", err))?;

        let item_rows = sqlx::query(
            r#"
            SELECT product_id, book_stock, actual_stock, delta_qty
            FROM stock_check_items
            WHERE tenant_id = $1 AND stock_check_id = $2
            ORDER BY id ASC
            "#,
        )
        .bind(tenant_id)
        .bind(check_id)
        .fetch_all(pool)
        .await
        .map_err(|err| map_sqlx_error("查询盘点单明细失败", err))?;

        let mut items = Vec::with_capacity(item_rows.len());
        for item_row in item_rows {
            items.push(StockCheckItem {
                product_id: item_row
                    .try_get("product_id")
                    .map_err(|err| map_sqlx_error("读取盘点明细商品ID失败", err))?,
                book_stock: item_row
                    .try_get("book_stock")
                    .map_err(|err| map_sqlx_error("读取盘点明细账面库存失败", err))?,
                actual_stock: item_row
                    .try_get("actual_stock")
                    .map_err(|err| map_sqlx_error("读取盘点明细实盘库存失败", err))?,
                delta_qty: item_row
                    .try_get("delta_qty")
                    .map_err(|err| map_sqlx_error("读取盘点明细差异数量失败", err))?,
            });
        }

        let counting_at = row
            .try_get::<Option<DateTime<Utc>>, _>("counting_at")
            .map_err(|err| map_sqlx_error("读取盘点开始时间失败", err))?
            .map(|dt| dt.to_rfc3339());
        let confirmed_at = row
            .try_get::<Option<DateTime<Utc>>, _>("confirmed_at")
            .map_err(|err| map_sqlx_error("读取盘点确认时间失败", err))?
            .map(|dt| dt.to_rfc3339());

        let created_at = row
            .try_get::<DateTime<Utc>, _>("created_at")
            .map_err(|err| map_sqlx_error("读取创建时间失败", err))?
            .to_rfc3339();
        let updated_at = row
            .try_get::<DateTime<Utc>, _>("updated_at")
            .map_err(|err| map_sqlx_error("读取更新时间失败", err))?
            .to_rfc3339();

        Ok(Some(StockCheck {
            id: row
                .try_get("id")
                .map_err(|err| map_sqlx_error("读取盘点单ID失败", err))?,
            tenant_id: row
                .try_get("tenant_id")
                .map_err(|err| map_sqlx_error("读取租户ID失败", err))?,
            biz_no: row
                .try_get("biz_no")
                .map_err(|err| map_sqlx_error("读取盘点单号失败", err))?,
            status: parse_stock_check_status(&status_raw)?,
            items,
            remark: row
                .try_get("remark")
                .map_err(|err| map_sqlx_error("读取备注失败", err))?,
            created_by: row
                .try_get("created_by")
                .map_err(|err| map_sqlx_error("读取创建人失败", err))?,
            version: row
                .try_get("version")
                .map_err(|err| map_sqlx_error("读取版本失败", err))?,
            counting_at,
            confirmed_at,
            created_at,
            updated_at,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn start_stock_check(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        check_id: i64,
        expected_version: Option<i32>,
        operator_id: Uuid,
        request_id: &str,
    ) -> Result<StockCheck, AppError> {
        let pool = require_pool(pool)?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|err| map_sqlx_error("开始盘点事务开启失败", err))?;

        let existing = self
            .find_stock_check_by_id_for_update(&mut tx, tenant_id, check_id)
            .await?
            .ok_or_else(|| AppError::not_found("盘点单不存在"))?;

        if let Some(ev) = expected_version
            && ev != existing.version
        {
            return Err(
                AppError::conflict(4091, "版本冲突，请刷新后重试").with_data(json!({
                    "resource": "stock_check",
                    "resource_id": existing.id,
                    "expected_version": ev,
                    "current_version": existing.version,
                    "latest_snapshot": stock_check_snapshot(&existing)
                })),
            );
        }

        if existing.status != StockCheckStatus::Draft {
            return Err(AppError::conflict(4090, "仅 DRAFT 状态可开始盘点")
                .with_data(json!({ "status": existing.status.as_str() })));
        }

        let now = Utc::now();
        let now_rfc3339 = now.to_rfc3339();
        let mut updated = existing.clone();
        updated.status = StockCheckStatus::Counting;
        updated.version += 1;
        updated.counting_at = Some(now_rfc3339.clone());
        updated.updated_at = now_rfc3339;

        sqlx::query(
            r#"
            UPDATE stock_checks
            SET status = 'COUNTING',
                version = $1,
                counting_at = $2,
                updated_at = $2
            WHERE tenant_id = $3 AND id = $4
            "#,
        )
        .bind(updated.version)
        .bind(now)
        .bind(tenant_id)
        .bind(check_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("更新盘点单状态失败", err))?;

        let audit_log_id = self.next_audit_log_id_in_tx(&mut tx).await?;
        sqlx::query(
            r#"
            INSERT INTO audit_logs (
                id, tenant_id, operator_id, action, target_type,
                target_id, before_data, after_data, request_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            "#,
        )
        .bind(audit_log_id)
        .bind(tenant_id)
        .bind(operator_id)
        .bind("STOCK_CHECK_START")
        .bind("stock_check")
        .bind(check_id.to_string())
        .bind(stock_check_snapshot(&existing))
        .bind(stock_check_snapshot(&updated))
        .bind(request_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("写入审计日志失败", err))?;

        tx.commit()
            .await
            .map_err(|err| map_sqlx_error("开始盘点事务提交失败", err))?;

        Ok(updated)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn confirm_stock_check(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        check_id: i64,
        expected_version: Option<i32>,
        actual_items: &[(i64, i32)],
        remark: Option<String>,
        operator_id: Uuid,
        request_id: &str,
    ) -> Result<StockCheck, AppError> {
        let pool = require_pool(pool)?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|err| map_sqlx_error("确认盘点事务开启失败", err))?;

        let existing = self
            .find_stock_check_by_id_for_update(&mut tx, tenant_id, check_id)
            .await?
            .ok_or_else(|| AppError::not_found("盘点单不存在"))?;

        if let Some(ev) = expected_version
            && ev != existing.version
        {
            return Err(
                AppError::conflict(4091, "版本冲突，请刷新后重试").with_data(json!({
                    "resource": "stock_check",
                    "resource_id": existing.id,
                    "expected_version": ev,
                    "current_version": existing.version,
                    "latest_snapshot": stock_check_snapshot(&existing)
                })),
            );
        }

        if existing.status != StockCheckStatus::Counting {
            return Err(AppError::conflict(4090, "仅 COUNTING 状态可确认盘点")
                .with_data(json!({ "status": existing.status.as_str() })));
        }

        if actual_items.is_empty() {
            return Err(AppError::bad_request("items 不能为空"));
        }

        let mut actual_map: HashMap<i64, i32> = HashMap::with_capacity(actual_items.len());
        for &(product_id, actual_stock) in actual_items {
            if actual_stock < 0 {
                return Err(AppError::bad_request("actual_stock 不能小于 0"));
            }
            if actual_map.insert(product_id, actual_stock).is_some() {
                return Err(AppError::bad_request("items.product_id 存在重复"));
            }
        }

        if actual_map.len() != existing.items.len() {
            return Err(AppError::bad_request("盘点项数量不匹配").with_data(json!({
                "expected_items": existing.items.len(),
                "actual_items": actual_map.len()
            })));
        }

        let expected_set: HashSet<i64> =
            existing.items.iter().map(|item| item.product_id).collect();
        for product_id in actual_map.keys() {
            if !expected_set.contains(product_id) {
                return Err(AppError::bad_request("盘点商品不在盘点单中")
                    .with_data(json!({ "product_id": product_id })));
            }
        }

        let mut next_stock_log_id = self.next_stock_log_id_in_tx(&mut tx).await?;
        let mut adjustment_map: HashMap<i64, (i32, i32)> =
            HashMap::with_capacity(existing.items.len());

        for item in &existing.items {
            let product_row = sqlx::query(
                r#"
                SELECT id, current_stock, cost_price, version, is_deleted
                FROM products
                WHERE tenant_id = $1 AND id = $2
                FOR UPDATE
                "#,
            )
            .bind(tenant_id)
            .bind(item.product_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("锁定商品失败", err))?
            .ok_or_else(|| AppError::not_found("商品不存在"))?;

            let is_deleted: bool = product_row
                .try_get("is_deleted")
                .map_err(|err| map_sqlx_error("读取商品删除标记失败", err))?;
            if is_deleted {
                return Err(AppError::not_found("商品不存在"));
            }

            let product_id: i64 = product_row
                .try_get("id")
                .map_err(|err| map_sqlx_error("读取商品ID失败", err))?;
            let current_stock: i32 = product_row
                .try_get("current_stock")
                .map_err(|err| map_sqlx_error("读取库存失败", err))?;
            let snapshot_cost: Decimal = product_row
                .try_get("cost_price")
                .map_err(|err| map_sqlx_error("读取成本价失败", err))?;
            let product_version: i32 = product_row
                .try_get("version")
                .map_err(|err| map_sqlx_error("读取商品版本失败", err))?;

            if current_stock != item.book_stock {
                return Err(
                    AppError::conflict(4091, "盘点基准库存已变化，请重新发起盘点").with_data(
                        json!({
                            "resource": "product",
                            "resource_id": product_id,
                            "book_stock": item.book_stock,
                            "current_stock": current_stock,
                            "latest_snapshot": {
                                "id": product_id,
                                "current_stock": current_stock,
                                "cost_price": snapshot_cost.round_dp(4).to_string(),
                                "version": product_version
                            }
                        }),
                    ),
                );
            }

            let actual_stock = actual_map.get(&item.product_id).copied().ok_or_else(|| {
                AppError::bad_request("盘点项不完整")
                    .with_data(json!({ "product_id": item.product_id }))
            })?;
            let delta_qty = actual_stock - item.book_stock;

            sqlx::query(
                r#"
                UPDATE products
                SET current_stock = $1,
                    version = $2,
                    updated_at = NOW()
                WHERE tenant_id = $3 AND id = $4
                "#,
            )
            .bind(actual_stock)
            .bind(product_version + 1)
            .bind(tenant_id)
            .bind(product_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("更新商品库存失败", err))?;

            if delta_qty != 0 {
                sqlx::query(
                    r#"
                    INSERT INTO stock_logs (
                        id, tenant_id, product_id, biz_type, biz_no,
                        delta_qty, snapshot_stock, snapshot_cost, snapshot_sell_price, snapshot_inbound_unit_cost, operator_id, created_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
                    "#,
                )
                .bind(next_stock_log_id)
                .bind(tenant_id)
                .bind(product_id)
                .bind("ADJ_CHECK")
                .bind(&existing.biz_no)
                .bind(delta_qty)
                .bind(actual_stock)
                .bind(snapshot_cost)
                .bind(Option::<Decimal>::None)
                .bind(Option::<Decimal>::None)
                .bind(operator_id)
                .execute(&mut *tx)
                .await
                .map_err(|err| map_sqlx_error("写入库存流水失败", err))?;

                next_stock_log_id += 1;
            }

            adjustment_map.insert(item.product_id, (actual_stock, delta_qty));
        }

        let now = Utc::now();
        let now_rfc3339 = now.to_rfc3339();
        let mut updated = existing.clone();
        for item in &mut updated.items {
            if let Some((actual_stock, delta_qty)) = adjustment_map.get(&item.product_id).copied() {
                item.actual_stock = Some(actual_stock);
                item.delta_qty = Some(delta_qty);
            }
        }
        updated.status = StockCheckStatus::Confirmed;
        updated.version += 1;
        updated.confirmed_at = Some(now_rfc3339.clone());
        updated.updated_at = now_rfc3339.clone();
        if let Some(remark) = remark {
            updated.remark = Some(remark);
        }

        sqlx::query(
            r#"
            UPDATE stock_checks
            SET status = 'CONFIRMED',
                version = $1,
                remark = $2,
                confirmed_at = $3,
                updated_at = $3
            WHERE tenant_id = $4 AND id = $5
            "#,
        )
        .bind(updated.version)
        .bind(&updated.remark)
        .bind(now)
        .bind(tenant_id)
        .bind(check_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("更新盘点单状态失败", err))?;

        for item in &updated.items {
            sqlx::query(
                r#"
                UPDATE stock_check_items
                SET actual_stock = $1,
                    delta_qty = $2
                WHERE tenant_id = $3 AND stock_check_id = $4 AND product_id = $5
                "#,
            )
            .bind(item.actual_stock)
            .bind(item.delta_qty)
            .bind(tenant_id)
            .bind(check_id)
            .bind(item.product_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("更新盘点单明细失败", err))?;
        }

        let audit_log_id = self.next_audit_log_id_in_tx(&mut tx).await?;
        sqlx::query(
            r#"
            INSERT INTO audit_logs (
                id, tenant_id, operator_id, action, target_type,
                target_id, before_data, after_data, request_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            "#,
        )
        .bind(audit_log_id)
        .bind(tenant_id)
        .bind(operator_id)
        .bind("STOCK_CHECK_CONFIRM")
        .bind("stock_check")
        .bind(check_id.to_string())
        .bind(stock_check_snapshot(&existing))
        .bind(stock_check_snapshot(&updated))
        .bind(request_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("写入审计日志失败", err))?;

        tx.commit()
            .await
            .map_err(|err| map_sqlx_error("确认盘点事务提交失败", err))?;

        Ok(updated)
    }

    async fn find_stock_check_by_id_for_update(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: Uuid,
        check_id: i64,
    ) -> Result<Option<StockCheck>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, biz_no,
                   status, remark, created_by,
                   version, counting_at, confirmed_at,
                   created_at, updated_at
            FROM stock_checks
            WHERE tenant_id = $1 AND id = $2
            FOR UPDATE
            "#,
        )
        .bind(tenant_id)
        .bind(check_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|err| map_sqlx_error("锁定盘点单失败", err))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let status_raw: String = row
            .try_get("status")
            .map_err(|err| map_sqlx_error("读取盘点单状态失败", err))?;
        let items_rows = sqlx::query(
            r#"
            SELECT product_id, book_stock, actual_stock, delta_qty
            FROM stock_check_items
            WHERE tenant_id = $1 AND stock_check_id = $2
            ORDER BY id ASC
            "#,
        )
        .bind(tenant_id)
        .bind(check_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|err| map_sqlx_error("查询盘点单明细失败", err))?;

        let mut items = Vec::with_capacity(items_rows.len());
        for item_row in items_rows {
            items.push(StockCheckItem {
                product_id: item_row
                    .try_get("product_id")
                    .map_err(|err| map_sqlx_error("读取盘点明细商品ID失败", err))?,
                book_stock: item_row
                    .try_get("book_stock")
                    .map_err(|err| map_sqlx_error("读取盘点明细账面库存失败", err))?,
                actual_stock: item_row
                    .try_get("actual_stock")
                    .map_err(|err| map_sqlx_error("读取盘点明细实盘库存失败", err))?,
                delta_qty: item_row
                    .try_get("delta_qty")
                    .map_err(|err| map_sqlx_error("读取盘点明细差异数量失败", err))?,
            });
        }

        let counting_at = row
            .try_get::<Option<DateTime<Utc>>, _>("counting_at")
            .map_err(|err| map_sqlx_error("读取盘点开始时间失败", err))?
            .map(|dt| dt.to_rfc3339());
        let confirmed_at = row
            .try_get::<Option<DateTime<Utc>>, _>("confirmed_at")
            .map_err(|err| map_sqlx_error("读取盘点确认时间失败", err))?
            .map(|dt| dt.to_rfc3339());

        let created_at = row
            .try_get::<DateTime<Utc>, _>("created_at")
            .map_err(|err| map_sqlx_error("读取创建时间失败", err))?
            .to_rfc3339();
        let updated_at = row
            .try_get::<DateTime<Utc>, _>("updated_at")
            .map_err(|err| map_sqlx_error("读取更新时间失败", err))?
            .to_rfc3339();

        Ok(Some(StockCheck {
            id: row
                .try_get("id")
                .map_err(|err| map_sqlx_error("读取盘点单ID失败", err))?,
            tenant_id: row
                .try_get("tenant_id")
                .map_err(|err| map_sqlx_error("读取租户ID失败", err))?,
            biz_no: row
                .try_get("biz_no")
                .map_err(|err| map_sqlx_error("读取盘点单号失败", err))?,
            status: parse_stock_check_status(&status_raw)?,
            items,
            remark: row
                .try_get("remark")
                .map_err(|err| map_sqlx_error("读取备注失败", err))?,
            created_by: row
                .try_get("created_by")
                .map_err(|err| map_sqlx_error("读取创建人失败", err))?,
            version: row
                .try_get("version")
                .map_err(|err| map_sqlx_error("读取版本失败", err))?,
            counting_at,
            confirmed_at,
            created_at,
            updated_at,
        }))
    }

    async fn find_sales_order_by_id_for_update(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: Uuid,
        order_id: i64,
    ) -> Result<Option<SalesOrder>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, biz_no, customer_id,
                   status, remark, created_by,
                   version, confirmed_at, returned_at, voided_at,
                   created_at, updated_at
            FROM sales_orders
            WHERE tenant_id = $1 AND id = $2
            FOR UPDATE
            "#,
        )
        .bind(tenant_id)
        .bind(order_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|err| map_sqlx_error("锁定销售单失败", err))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let status_raw: String = row
            .try_get("status")
            .map_err(|err| map_sqlx_error("读取销售单状态失败", err))?;
        let items_rows = sqlx::query(
            r#"
            SELECT product_id, qty, sell_price, returned_qty, product_name_snapshot
            FROM sales_order_items
            WHERE tenant_id = $1 AND sales_order_id = $2
            ORDER BY id ASC
            "#,
        )
        .bind(tenant_id)
        .bind(order_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|err| map_sqlx_error("查询销售单明细失败", err))?;

        let mut items = Vec::with_capacity(items_rows.len());
        for item_row in items_rows {
            items.push(SalesOrderItem {
                product_id: item_row
                    .try_get("product_id")
                    .map_err(|err| map_sqlx_error("读取销售明细商品ID失败", err))?,
                qty: item_row
                    .try_get("qty")
                    .map_err(|err| map_sqlx_error("读取销售明细数量失败", err))?,
                sell_price: item_row
                    .try_get("sell_price")
                    .map_err(|err| map_sqlx_error("读取销售明细单价失败", err))?,
                returned_qty: item_row
                    .try_get("returned_qty")
                    .map_err(|err| map_sqlx_error("读取销售明细已退数量失败", err))?,
                product_name_snapshot: item_row
                    .try_get("product_name_snapshot")
                    .map_err(|err| map_sqlx_error("读取销售明细商品名称快照失败", err))?,
            });
        }

        let confirmed_at = row
            .try_get::<Option<DateTime<Utc>>, _>("confirmed_at")
            .map_err(|err| map_sqlx_error("读取确认时间失败", err))?
            .map(|dt| dt.to_rfc3339());
        let returned_at = row
            .try_get::<Option<DateTime<Utc>>, _>("returned_at")
            .map_err(|err| map_sqlx_error("读取退货时间失败", err))?
            .map(|dt| dt.to_rfc3339());
        let voided_at = row
            .try_get::<Option<DateTime<Utc>>, _>("voided_at")
            .map_err(|err| map_sqlx_error("读取作废时间失败", err))?
            .map(|dt| dt.to_rfc3339());

        let created_at = row
            .try_get::<DateTime<Utc>, _>("created_at")
            .map_err(|err| map_sqlx_error("读取创建时间失败", err))?
            .to_rfc3339();
        let updated_at = row
            .try_get::<DateTime<Utc>, _>("updated_at")
            .map_err(|err| map_sqlx_error("读取更新时间失败", err))?
            .to_rfc3339();

        Ok(Some(SalesOrder {
            id: row
                .try_get("id")
                .map_err(|err| map_sqlx_error("读取销售单ID失败", err))?,
            tenant_id: row
                .try_get("tenant_id")
                .map_err(|err| map_sqlx_error("读取租户ID失败", err))?,
            biz_no: row
                .try_get("biz_no")
                .map_err(|err| map_sqlx_error("读取销售单号失败", err))?,
            customer_id: row
                .try_get("customer_id")
                .map_err(|err| map_sqlx_error("读取客户ID失败", err))?,
            status: parse_sales_order_status(&status_raw)?,
            items,
            remark: row
                .try_get("remark")
                .map_err(|err| map_sqlx_error("读取备注失败", err))?,
            created_by: row
                .try_get("created_by")
                .map_err(|err| map_sqlx_error("读取创建人失败", err))?,
            version: row
                .try_get("version")
                .map_err(|err| map_sqlx_error("读取版本失败", err))?,
            confirmed_at,
            returned_at,
            voided_at,
            created_at,
            updated_at,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn void_purchase_order(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        order_id: i64,
        expected_version: Option<i32>,
        allow_negative_stock: bool,
        operator_id: Uuid,
        request_id: &str,
    ) -> Result<PurchaseOrder, AppError> {
        let pool = require_pool(pool)?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|err| map_sqlx_error("作废采购单事务开启失败", err))?;

        let existing = self
            .find_purchase_order_by_id_for_update(&mut tx, tenant_id, order_id)
            .await?
            .ok_or_else(|| AppError::not_found("采购单不存在"))?;

        if let Some(ev) = expected_version
            && ev != existing.version
        {
            return Err(
                AppError::conflict(4091, "版本冲突，请刷新后重试").with_data(json!({
                    "resource": "purchase_order",
                    "resource_id": existing.id,
                    "expected_version": ev,
                    "current_version": existing.version,
                    "latest_snapshot": purchase_order_snapshot(&existing)
                })),
            );
        }

        if existing.status == PurchaseOrderStatus::Voided {
            return Err(AppError::conflict(4090, "采购单已作废"));
        }

        let mut next_stock_log_id = self.next_stock_log_id_in_tx(&mut tx).await?;

        if existing.status == PurchaseOrderStatus::Confirmed {
            for item in &existing.items {
                let product_row = sqlx::query(
                    r#"
                    SELECT id, current_stock, cost_price, version, is_deleted
                    FROM products
                    WHERE tenant_id = $1 AND id = $2
                    FOR UPDATE
                    "#,
                )
                .bind(tenant_id)
                .bind(item.product_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|err| map_sqlx_error("锁定商品失败", err))?
                .ok_or_else(|| AppError::not_found("商品不存在"))?;

                let is_deleted: bool = product_row
                    .try_get("is_deleted")
                    .map_err(|err| map_sqlx_error("读取商品删除标记失败", err))?;
                if is_deleted {
                    return Err(AppError::not_found("商品不存在"));
                }

                let product_id: i64 = product_row
                    .try_get("id")
                    .map_err(|err| map_sqlx_error("读取商品ID失败", err))?;
                let current_stock: i32 = product_row
                    .try_get("current_stock")
                    .map_err(|err| map_sqlx_error("读取库存失败", err))?;
                let product_version: i32 = product_row
                    .try_get("version")
                    .map_err(|err| map_sqlx_error("读取商品版本失败", err))?;
                let snapshot_cost: Decimal = product_row
                    .try_get("cost_price")
                    .map_err(|err| map_sqlx_error("读取成本价失败", err))?;

                if !allow_negative_stock && current_stock < item.qty {
                    return Err(AppError::business(
                        StatusCode::BAD_REQUEST,
                        4001,
                        "库存不足，无法执行反向作废",
                    )
                    .with_data(json!({
                        "failed_product_id": product_id,
                        "available_stock": current_stock,
                        "required_qty": item.qty
                    })));
                }

                let new_stock = current_stock - item.qty;

                sqlx::query(
                    r#"
                    UPDATE products
                    SET current_stock = $1,
                        version = $2,
                        updated_at = NOW()
                    WHERE tenant_id = $3 AND id = $4
                    "#,
                )
                .bind(new_stock)
                .bind(product_version + 1)
                .bind(tenant_id)
                .bind(product_id)
                .execute(&mut *tx)
                .await
                .map_err(|err| map_sqlx_error("回滚采购库存失败", err))?;

                sqlx::query(
                    r#"
                    INSERT INTO stock_logs (
                        id, tenant_id, product_id, biz_type, biz_no,
                        delta_qty, snapshot_stock, snapshot_cost, snapshot_sell_price, snapshot_inbound_unit_cost, operator_id, created_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
                    "#,
                )
                .bind(next_stock_log_id)
                .bind(tenant_id)
                .bind(product_id)
                .bind("VOID_PURCHASE")
                .bind(&existing.biz_no)
                .bind(-item.qty)
                .bind(new_stock)
                .bind(snapshot_cost)
                .bind(Option::<Decimal>::None)
                .bind(Option::<Decimal>::None)
                .bind(operator_id)
                .execute(&mut *tx)
                .await
                .map_err(|err| map_sqlx_error("写入反向库存流水失败", err))?;

                next_stock_log_id += 1;
            }
        }

        let now = Utc::now();
        let now_rfc3339 = now.to_rfc3339();
        let mut updated = existing.clone();
        updated.status = PurchaseOrderStatus::Voided;
        updated.version += 1;
        updated.voided_at = Some(now_rfc3339.clone());
        updated.updated_at = now_rfc3339.clone();

        sqlx::query(
            r#"
            UPDATE purchase_orders
            SET status = 'VOIDED',
                version = $1,
                voided_at = $2,
                updated_at = $2
            WHERE tenant_id = $3 AND id = $4
            "#,
        )
        .bind(updated.version)
        .bind(now)
        .bind(tenant_id)
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("更新采购单作废状态失败", err))?;

        let audit_log_id = self.next_audit_log_id_in_tx(&mut tx).await?;
        sqlx::query(
            r#"
            INSERT INTO audit_logs (
                id, tenant_id, operator_id, action, target_type,
                target_id, before_data, after_data, request_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            "#,
        )
        .bind(audit_log_id)
        .bind(tenant_id)
        .bind(operator_id)
        .bind("PURCHASE_ORDER_VOID")
        .bind("purchase_order")
        .bind(order_id.to_string())
        .bind(purchase_order_snapshot(&existing))
        .bind(purchase_order_snapshot(&updated))
        .bind(request_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("写入审计日志失败", err))?;

        tx.commit()
            .await
            .map_err(|err| map_sqlx_error("作废采购单事务提交失败", err))?;

        Ok(updated)
    }

    pub async fn next_sales_order_id(&self, pool: Option<&PgPool>) -> Result<i64, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query("SELECT COALESCE(MAX(id), 4000) + 1 AS next_id FROM sales_orders")
            .fetch_one(pool)
            .await
            .map_err(|err| map_sqlx_error("生成销售单ID失败", err))?;

        row.try_get("next_id")
            .map_err(|err| map_sqlx_error("读取销售单ID失败", err))
    }

    pub async fn is_sales_order_biz_no_taken(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        biz_no: &str,
    ) -> Result<bool, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query(
            r#"
            SELECT 1
            FROM sales_orders
            WHERE tenant_id = $1 AND biz_no = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(biz_no)
        .fetch_optional(pool)
        .await
        .map_err(|err| map_sqlx_error("检查销售单号唯一性失败", err))?;

        Ok(row.is_some())
    }

    pub async fn create_sales_order(
        &self,
        pool: Option<&PgPool>,
        order: &SalesOrder,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|err| map_sqlx_error("创建销售单事务开启失败", err))?;

        let main_insert = sqlx::query(
            r#"
            INSERT INTO sales_orders (
                id, tenant_id, biz_no, customer_id,
                status, remark, created_by,
                version, confirmed_at, returned_at, voided_at,
                created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4,
                $5, $6, $7,
                $8, $9, $10, $11,
                $12, $13
            )
            "#,
        )
        .bind(order.id)
        .bind(order.tenant_id)
        .bind(&order.biz_no)
        .bind(order.customer_id)
        .bind(order.status.as_str())
        .bind(&order.remark)
        .bind(order.created_by)
        .bind(order.version)
        .bind(parse_optional_datetime(order.confirmed_at.as_deref())?)
        .bind(parse_optional_datetime(order.returned_at.as_deref())?)
        .bind(parse_optional_datetime(order.voided_at.as_deref())?)
        .bind(parse_datetime(&order.created_at)?)
        .bind(parse_datetime(&order.updated_at)?)
        .execute(&mut *tx)
        .await;

        if let Err(err) = main_insert {
            if is_sql_state(&err, "23505") {
                return Err(AppError::conflict(4090, "销售单号已存在")
                    .with_data(json!({ "biz_no": order.biz_no })));
            }
            return Err(map_sqlx_error("创建销售单主表失败", err));
        }

        for item in &order.items {
            let item_insert = sqlx::query(
                r#"
                INSERT INTO sales_order_items (
                    tenant_id, sales_order_id, product_id, qty, sell_price, returned_qty, product_name_snapshot, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
                "#,
            )
            .bind(order.tenant_id)
            .bind(order.id)
            .bind(item.product_id)
            .bind(item.qty)
            .bind(item.sell_price)
            .bind(item.returned_qty)
            .bind(&item.product_name_snapshot)
            .execute(&mut *tx)
            .await;

            if let Err(err) = item_insert {
                if is_sql_state(&err, "23503") {
                    return Err(AppError::not_found("商品不存在"));
                }
                return Err(map_sqlx_error("写入销售单明细失败", err));
            }
        }

        tx.commit()
            .await
            .map_err(|err| map_sqlx_error("提交销售单事务失败", err))?;

        Ok(())
    }

    pub async fn find_sales_order_by_id(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        order_id: i64,
    ) -> Result<Option<SalesOrder>, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, biz_no, customer_id,
                   status, remark, created_by,
                   version, confirmed_at, returned_at, voided_at,
                   created_at, updated_at
            FROM sales_orders
            WHERE tenant_id = $1 AND id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(order_id)
        .fetch_optional(pool)
        .await
        .map_err(|err| map_sqlx_error("查询销售单失败", err))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let status_raw: String = row
            .try_get("status")
            .map_err(|err| map_sqlx_error("读取销售单状态失败", err))?;

        let items_rows = sqlx::query(
            r#"
            SELECT product_id, qty, sell_price, returned_qty, product_name_snapshot
            FROM sales_order_items
            WHERE tenant_id = $1 AND sales_order_id = $2
            ORDER BY id ASC
            "#,
        )
        .bind(tenant_id)
        .bind(order_id)
        .fetch_all(pool)
        .await
        .map_err(|err| map_sqlx_error("查询销售单明细失败", err))?;

        let mut items = Vec::with_capacity(items_rows.len());
        for item_row in items_rows {
            items.push(SalesOrderItem {
                product_id: item_row
                    .try_get("product_id")
                    .map_err(|err| map_sqlx_error("读取销售明细商品ID失败", err))?,
                qty: item_row
                    .try_get("qty")
                    .map_err(|err| map_sqlx_error("读取销售明细数量失败", err))?,
                sell_price: item_row
                    .try_get("sell_price")
                    .map_err(|err| map_sqlx_error("读取销售明细单价失败", err))?,
                returned_qty: item_row
                    .try_get("returned_qty")
                    .map_err(|err| map_sqlx_error("读取销售明细已退数量失败", err))?,
                product_name_snapshot: item_row
                    .try_get("product_name_snapshot")
                    .map_err(|err| map_sqlx_error("读取销售明细商品名称快照失败", err))?,
            });
        }

        let confirmed_at = row
            .try_get::<Option<DateTime<Utc>>, _>("confirmed_at")
            .map_err(|err| map_sqlx_error("读取确认时间失败", err))?
            .map(|dt| dt.to_rfc3339());
        let returned_at = row
            .try_get::<Option<DateTime<Utc>>, _>("returned_at")
            .map_err(|err| map_sqlx_error("读取退货时间失败", err))?
            .map(|dt| dt.to_rfc3339());
        let voided_at = row
            .try_get::<Option<DateTime<Utc>>, _>("voided_at")
            .map_err(|err| map_sqlx_error("读取作废时间失败", err))?
            .map(|dt| dt.to_rfc3339());

        let created_at = row
            .try_get::<DateTime<Utc>, _>("created_at")
            .map_err(|err| map_sqlx_error("读取创建时间失败", err))?
            .to_rfc3339();
        let updated_at = row
            .try_get::<DateTime<Utc>, _>("updated_at")
            .map_err(|err| map_sqlx_error("读取更新时间失败", err))?
            .to_rfc3339();

        Ok(Some(SalesOrder {
            id: row
                .try_get("id")
                .map_err(|err| map_sqlx_error("读取销售单ID失败", err))?,
            tenant_id: row
                .try_get("tenant_id")
                .map_err(|err| map_sqlx_error("读取租户ID失败", err))?,
            biz_no: row
                .try_get("biz_no")
                .map_err(|err| map_sqlx_error("读取销售单号失败", err))?,
            customer_id: row
                .try_get("customer_id")
                .map_err(|err| map_sqlx_error("读取客户ID失败", err))?,
            status: parse_sales_order_status(&status_raw)?,
            items,
            remark: row
                .try_get("remark")
                .map_err(|err| map_sqlx_error("读取备注失败", err))?,
            created_by: row
                .try_get("created_by")
                .map_err(|err| map_sqlx_error("读取创建人失败", err))?,
            version: row
                .try_get("version")
                .map_err(|err| map_sqlx_error("读取版本失败", err))?,
            confirmed_at,
            returned_at,
            voided_at,
            created_at,
            updated_at,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn confirm_sales_order(
        &self,
        pool: Option<&PgPool>,
        tenant_id: Uuid,
        order_id: i64,
        expected_version: Option<i32>,
        allow_negative_stock: bool,
        operator_id: Uuid,
        request_id: &str,
    ) -> Result<SalesOrder, AppError> {
        let pool = require_pool(pool)?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|err| map_sqlx_error("确认销售单事务开启失败", err))?;

        let existing = self
            .find_sales_order_by_id_for_update(&mut tx, tenant_id, order_id)
            .await?
            .ok_or_else(|| AppError::not_found("销售单不存在"))?;

        if let Some(ev) = expected_version
            && ev != existing.version
        {
            return Err(
                AppError::conflict(4091, "版本冲突，请刷新后重试").with_data(json!({
                    "resource": "sales_order",
                    "resource_id": existing.id,
                    "expected_version": ev,
                    "current_version": existing.version,
                    "latest_snapshot": sales_order_snapshot(&existing)
                })),
            );
        }

        if existing.status != SalesOrderStatus::Draft {
            return Err(AppError::conflict(4090, "仅 DRAFT 状态可确认销售单")
                .with_data(json!({ "status": existing.status.as_str() })));
        }

        let mut next_stock_log_id = self.next_stock_log_id_in_tx(&mut tx).await?;

        for item in &existing.items {
            let product_row = sqlx::query(
                r#"
                SELECT id, current_stock, cost_price, version, is_deleted
                FROM products
                WHERE tenant_id = $1 AND id = $2
                FOR UPDATE
                "#,
            )
            .bind(tenant_id)
            .bind(item.product_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("锁定商品失败", err))?
            .ok_or_else(|| AppError::not_found("商品不存在"))?;

            let is_deleted: bool = product_row
                .try_get("is_deleted")
                .map_err(|err| map_sqlx_error("读取商品删除标记失败", err))?;
            if is_deleted {
                return Err(AppError::not_found("商品不存在"));
            }

            let product_id: i64 = product_row
                .try_get("id")
                .map_err(|err| map_sqlx_error("读取商品ID失败", err))?;
            let current_stock: i32 = product_row
                .try_get("current_stock")
                .map_err(|err| map_sqlx_error("读取库存失败", err))?;
            let product_version: i32 = product_row
                .try_get("version")
                .map_err(|err| map_sqlx_error("读取商品版本失败", err))?;
            let snapshot_cost: Decimal = product_row
                .try_get("cost_price")
                .map_err(|err| map_sqlx_error("读取成本价失败", err))?;

            if !allow_negative_stock && current_stock < item.qty {
                return Err(
                    AppError::business(StatusCode::BAD_REQUEST, 4001, "库存不足").with_data(
                        json!({
                            "failed_product_id": product_id,
                            "available_stock": current_stock,
                            "required_qty": item.qty
                        }),
                    ),
                );
            }

            let new_stock = current_stock - item.qty;

            sqlx::query(
                r#"
                UPDATE products
                SET current_stock = $1,
                    version = $2,
                    updated_at = NOW()
                WHERE tenant_id = $3 AND id = $4
                "#,
            )
            .bind(new_stock)
            .bind(product_version + 1)
            .bind(tenant_id)
            .bind(product_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("更新商品库存失败", err))?;

            sqlx::query(
                r#"
                INSERT INTO stock_logs (
                    id, tenant_id, product_id, biz_type, biz_no,
                        delta_qty, snapshot_stock, snapshot_cost, snapshot_sell_price, snapshot_inbound_unit_cost, operator_id, created_at
                )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
                "#,
            )
            .bind(next_stock_log_id)
            .bind(tenant_id)
            .bind(product_id)
            .bind("OUT_SALE")
            .bind(&existing.biz_no)
            .bind(-item.qty)
            .bind(new_stock)
            .bind(snapshot_cost)
            .bind(Some(item.sell_price))
                .bind(Option::<Decimal>::None)
            .bind(operator_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("写入库存流水失败", err))?;

            next_stock_log_id += 1;
        }

        let now = Utc::now();
        let now_rfc3339 = now.to_rfc3339();
        let mut updated = existing.clone();
        updated.status = SalesOrderStatus::Confirmed;
        updated.version += 1;
        updated.confirmed_at = Some(now_rfc3339.clone());
        updated.updated_at = now_rfc3339.clone();

        sqlx::query(
            r#"
            UPDATE sales_orders
            SET status = 'CONFIRMED',
                version = $1,
                confirmed_at = $2,
                updated_at = $2
            WHERE tenant_id = $3 AND id = $4
            "#,
        )
        .bind(updated.version)
        .bind(now)
        .bind(tenant_id)
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("更新销售单状态失败", err))?;

        let audit_log_id = self.next_audit_log_id_in_tx(&mut tx).await?;
        sqlx::query(
            r#"
            INSERT INTO audit_logs (
                id, tenant_id, operator_id, action, target_type,
                target_id, before_data, after_data, request_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            "#,
        )
        .bind(audit_log_id)
        .bind(tenant_id)
        .bind(operator_id)
        .bind("SALES_ORDER_CONFIRM")
        .bind("sales_order")
        .bind(order_id.to_string())
        .bind(sales_order_snapshot(&existing))
        .bind(sales_order_snapshot(&updated))
        .bind(request_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("写入审计日志失败", err))?;

        tx.commit()
            .await
            .map_err(|err| map_sqlx_error("确认销售单事务提交失败", err))?;

        Ok(updated)
    }

    async fn find_purchase_order_by_id_for_update(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: Uuid,
        order_id: i64,
    ) -> Result<Option<PurchaseOrder>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, biz_no, supplier_id,
                   status, remark, created_by,
                   version, confirmed_at, voided_at,
                   created_at, updated_at
            FROM purchase_orders
            WHERE tenant_id = $1 AND id = $2
            FOR UPDATE
            "#,
        )
        .bind(tenant_id)
        .bind(order_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|err| map_sqlx_error("锁定采购单失败", err))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let status_raw: String = row
            .try_get("status")
            .map_err(|err| map_sqlx_error("读取采购单状态失败", err))?;
        let items_rows = sqlx::query(
            r#"
            SELECT product_id, qty, unit_cost, product_name_snapshot
            FROM purchase_order_items
            WHERE tenant_id = $1 AND purchase_order_id = $2
            ORDER BY id ASC
            "#,
        )
        .bind(tenant_id)
        .bind(order_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|err| map_sqlx_error("查询采购单明细失败", err))?;

        let mut items = Vec::with_capacity(items_rows.len());
        for item_row in items_rows {
            items.push(PurchaseOrderItem {
                product_id: item_row
                    .try_get("product_id")
                    .map_err(|err| map_sqlx_error("读取采购明细商品ID失败", err))?,
                qty: item_row
                    .try_get("qty")
                    .map_err(|err| map_sqlx_error("读取采购明细数量失败", err))?,
                unit_cost: item_row
                    .try_get("unit_cost")
                    .map_err(|err| map_sqlx_error("读取采购明细单价失败", err))?,
                product_name_snapshot: item_row
                    .try_get("product_name_snapshot")
                    .map_err(|err| map_sqlx_error("读取采购明细商品名称快照失败", err))?,
            });
        }

        let confirmed_at = row
            .try_get::<Option<DateTime<Utc>>, _>("confirmed_at")
            .map_err(|err| map_sqlx_error("读取确认时间失败", err))?
            .map(|dt| dt.to_rfc3339());
        let voided_at = row
            .try_get::<Option<DateTime<Utc>>, _>("voided_at")
            .map_err(|err| map_sqlx_error("读取作废时间失败", err))?
            .map(|dt| dt.to_rfc3339());

        let created_at = row
            .try_get::<DateTime<Utc>, _>("created_at")
            .map_err(|err| map_sqlx_error("读取创建时间失败", err))?
            .to_rfc3339();
        let updated_at = row
            .try_get::<DateTime<Utc>, _>("updated_at")
            .map_err(|err| map_sqlx_error("读取更新时间失败", err))?
            .to_rfc3339();

        Ok(Some(PurchaseOrder {
            id: row
                .try_get("id")
                .map_err(|err| map_sqlx_error("读取采购单ID失败", err))?,
            tenant_id: row
                .try_get("tenant_id")
                .map_err(|err| map_sqlx_error("读取租户ID失败", err))?,
            biz_no: row
                .try_get("biz_no")
                .map_err(|err| map_sqlx_error("读取采购单号失败", err))?,
            supplier_id: row
                .try_get("supplier_id")
                .map_err(|err| map_sqlx_error("读取供应商ID失败", err))?,
            status: parse_purchase_order_status(&status_raw)?,
            items,
            remark: row
                .try_get("remark")
                .map_err(|err| map_sqlx_error("读取备注失败", err))?,
            created_by: row
                .try_get("created_by")
                .map_err(|err| map_sqlx_error("读取创建人失败", err))?,
            version: row
                .try_get("version")
                .map_err(|err| map_sqlx_error("读取版本失败", err))?,
            confirmed_at,
            voided_at,
            created_at,
            updated_at,
        }))
    }

    async fn next_stock_log_id_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<i64, AppError> {
        let row = sqlx::query("SELECT COALESCE(MAX(id), 0) + 1 AS next_id FROM stock_logs")
            .fetch_one(&mut **tx)
            .await
            .map_err(|err| map_sqlx_error("生成库存流水ID失败", err))?;

        row.try_get("next_id")
            .map_err(|err| map_sqlx_error("读取库存流水ID失败", err))
    }

    async fn next_audit_log_id_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<i64, AppError> {
        let row = sqlx::query("SELECT COALESCE(MAX(id), 0) + 1 AS next_id FROM audit_logs")
            .fetch_one(&mut **tx)
            .await
            .map_err(|err| map_sqlx_error("生成审计日志ID失败", err))?;

        row.try_get("next_id")
            .map_err(|err| map_sqlx_error("读取审计日志ID失败", err))
    }
}

fn parse_user_role(raw: &str) -> Result<UserRole, AppError> {
    match raw {
        "OWNER" => Ok(UserRole::Owner),
        "PURCHASER" => Ok(UserRole::Purchaser),
        "SALES" => Ok(UserRole::Sales),
        _ => Err(AppError::internal(format!("未知角色: {raw}"))),
    }
}

fn parse_barcode_lookup_status(raw: &str) -> Result<BarcodeLookupStatus, AppError> {
    match raw {
        "FOUND" => Ok(BarcodeLookupStatus::Found),
        "NOT_FOUND" => Ok(BarcodeLookupStatus::NotFound),
        _ => Err(AppError::internal(format!("未知条码缓存状态: {raw}"))),
    }
}

fn parse_purchase_order_status(raw: &str) -> Result<PurchaseOrderStatus, AppError> {
    match raw {
        "DRAFT" => Ok(PurchaseOrderStatus::Draft),
        "CONFIRMED" => Ok(PurchaseOrderStatus::Confirmed),
        "VOIDED" => Ok(PurchaseOrderStatus::Voided),
        _ => Err(AppError::internal(format!("未知采购单状态: {raw}"))),
    }
}

fn parse_sales_order_status(raw: &str) -> Result<SalesOrderStatus, AppError> {
    match raw {
        "DRAFT" => Ok(SalesOrderStatus::Draft),
        "CONFIRMED" => Ok(SalesOrderStatus::Confirmed),
        "RETURNED_PARTIAL" => Ok(SalesOrderStatus::ReturnedPartial),
        "RETURNED_FULL" => Ok(SalesOrderStatus::ReturnedFull),
        "VOIDED" => Ok(SalesOrderStatus::Voided),
        _ => Err(AppError::internal(format!("未知销售单状态: {raw}"))),
    }
}

fn parse_stock_check_status(raw: &str) -> Result<StockCheckStatus, AppError> {
    match raw {
        "DRAFT" => Ok(StockCheckStatus::Draft),
        "COUNTING" => Ok(StockCheckStatus::Counting),
        "CONFIRMED" => Ok(StockCheckStatus::Confirmed),
        _ => Err(AppError::internal(format!("未知盘点单状态: {raw}"))),
    }
}

fn map_product_row(row: sqlx::postgres::PgRow) -> Result<Product, AppError> {
    Ok(Product {
        id: row
            .try_get("id")
            .map_err(|err| map_sqlx_error("读取商品ID失败", err))?,
        tenant_id: row
            .try_get("tenant_id")
            .map_err(|err| map_sqlx_error("读取租户ID失败", err))?,
        sku: row
            .try_get("sku")
            .map_err(|err| map_sqlx_error("读取SKU失败", err))?,
        barcode: row
            .try_get("barcode")
            .map_err(|err| map_sqlx_error("读取条码失败", err))?,
        name: row
            .try_get("name")
            .map_err(|err| map_sqlx_error("读取商品名失败", err))?,
        unit: row
            .try_get("unit")
            .map_err(|err| map_sqlx_error("读取单位失败", err))?,
        current_stock: row
            .try_get("current_stock")
            .map_err(|err| map_sqlx_error("读取库存失败", err))?,
        cost_price: row
            .try_get("cost_price")
            .map_err(|err| map_sqlx_error("读取成本价失败", err))?,
        retail_price: row
            .try_get("retail_price")
            .map_err(|err| map_sqlx_error("读取零售价失败", err))?,
        last_inbound_unit_cost: row
            .try_get("last_inbound_unit_cost")
            .map_err(|err| map_sqlx_error("读取最近入库单价失败", err))?,
        min_stock_limit: row
            .try_get("min_stock_limit")
            .map_err(|err| map_sqlx_error("读取最低库存失败", err))?,
        version: row
            .try_get("version")
            .map_err(|err| map_sqlx_error("读取版本失败", err))?,
        is_deleted: row
            .try_get("is_deleted")
            .map_err(|err| map_sqlx_error("读取删除标记失败", err))?,
    })
}

fn parse_datetime(raw: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|v| v.with_timezone(&Utc))
        .map_err(|_| AppError::internal("日期格式解析失败"))
}

fn parse_optional_datetime(raw: Option<&str>) -> Result<Option<DateTime<Utc>>, AppError> {
    match raw {
        Some(v) => parse_datetime(v).map(Some),
        None => Ok(None),
    }
}

fn product_snapshot(product: &Product) -> Value {
    json!({
        "id": product.id,
        "current_stock": product.current_stock,
        "cost_price": product.cost_price.round_dp(4).to_string(),
        "version": product.version
    })
}

fn purchase_order_snapshot(order: &PurchaseOrder) -> Value {
    json!({
        "id": order.id,
        "biz_no": order.biz_no,
        "status": order.status.as_str(),
        "version": order.version,
        "updated_at": order.updated_at
    })
}

fn sales_order_snapshot(order: &SalesOrder) -> Value {
    json!({
        "id": order.id,
        "biz_no": order.biz_no,
        "status": order.status.as_str(),
        "version": order.version,
        "updated_at": order.updated_at
    })
}

fn stock_check_snapshot(check: &StockCheck) -> Value {
    json!({
        "id": check.id,
        "biz_no": check.biz_no,
        "status": check.status.as_str(),
        "version": check.version,
        "updated_at": check.updated_at
    })
}

fn require_pool(pool: Option<&PgPool>) -> Result<&PgPool, AppError> {
    pool.ok_or_else(|| AppError::internal("PostgreSQL 连接池未初始化"))
}

fn unsupported_operation(name: &str) -> AppError {
    AppError::internal(format!("当前存储后端不支持操作: {name}"))
}

fn map_sqlx_error(prefix: &str, err: sqlx::Error) -> AppError {
    AppError::internal(format!("{prefix}: {err}"))
}

fn is_sql_state(err: &sqlx::Error, sql_state: &str) -> bool {
    match err {
        sqlx::Error::Database(db_err) => db_err.code().as_deref() == Some(sql_state),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicI64, Ordering};

    use axum::http::StatusCode;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use serde_json::json;
    use sqlx::{PgPool, postgres::PgPoolOptions};
    use uuid::Uuid;

    use super::{
        PostgresRepository, Product, PurchaseOrder, PurchaseOrderItem, PurchaseOrderStatus,
        SalesOrder, SalesOrderItem, SalesOrderStatus, StockCheck, StockCheckItem, StockCheckStatus,
    };

    static NEXT_TEST_PRODUCT_ID: AtomicI64 = AtomicI64::new(9_100_000_000);
    static NEXT_TEST_PURCHASE_ORDER_ID: AtomicI64 = AtomicI64::new(9_200_000_000);
    static NEXT_TEST_SALES_ORDER_ID: AtomicI64 = AtomicI64::new(9_300_000_000);
    static NEXT_TEST_STOCK_CHECK_ID: AtomicI64 = AtomicI64::new(9_400_000_000);

    fn next_test_product_id() -> i64 {
        NEXT_TEST_PRODUCT_ID.fetch_add(1, Ordering::Relaxed)
    }

    fn next_test_purchase_order_id() -> i64 {
        NEXT_TEST_PURCHASE_ORDER_ID.fetch_add(1, Ordering::Relaxed)
    }

    fn next_test_sales_order_id() -> i64 {
        NEXT_TEST_SALES_ORDER_ID.fetch_add(1, Ordering::Relaxed)
    }

    fn next_test_stock_check_id() -> i64 {
        NEXT_TEST_STOCK_CHECK_ID.fetch_add(1, Ordering::Relaxed)
    }

    fn test_database_url() -> Option<String> {
        std::env::var("TEST_DATABASE_URL")
            .ok()
            .or_else(|| std::env::var("DATABASE_URL").ok())
            .and_then(|value| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
    }

    async fn setup_pg_pool() -> Option<PgPool> {
        let Some(database_url) = test_database_url() else {
            eprintln!(
                "skip postgres repository regression tests: TEST_DATABASE_URL/DATABASE_URL 未配置"
            );
            return None;
        };

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("connect postgres for repository tests");

        ensure_test_tables(&pool).await;
        Some(pool)
    }

    async fn ensure_test_tables(pool: &PgPool) {
        ensure_products_table(pool).await;
        ensure_users_table(pool).await;
        ensure_stock_logs_table(pool).await;
        ensure_audit_logs_table(pool).await;
        ensure_purchase_orders_tables(pool).await;
        ensure_sales_orders_tables(pool).await;
        ensure_stock_checks_tables(pool).await;
    }

    async fn ensure_products_table(pool: &PgPool) {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS products (
                id BIGINT PRIMARY KEY,
                tenant_id UUID NOT NULL,
                sku VARCHAR(64) NOT NULL,
                barcode VARCHAR(64) NOT NULL,
                name VARCHAR(255) NOT NULL,
                unit VARCHAR(32) NOT NULL,
                current_stock INTEGER NOT NULL DEFAULT 0,
                cost_price NUMERIC(18,4) NOT NULL DEFAULT 0,
                retail_price NUMERIC(18,4) NOT NULL DEFAULT 0,
                last_inbound_unit_cost NUMERIC(18,4) NULL,
                min_stock_limit INTEGER NOT NULL DEFAULT 0,
                version INTEGER NOT NULL DEFAULT 1,
                is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("ensure products table for repository tests");
    }

    async fn ensure_users_table(pool: &PgPool) {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY,
                tenant_id UUID NOT NULL,
                username VARCHAR(64) NOT NULL,
                name VARCHAR(128) NOT NULL,
                role VARCHAR(32) NOT NULL,
                password_hash VARCHAR(128) NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                CONSTRAINT users_role_check CHECK (role IN ('OWNER', 'PURCHASER', 'SALES'))
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("ensure users table for repository tests");

        sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS ux_users_username ON users (username)")
            .execute(pool)
            .await
            .expect("ensure users username index for repository tests");
    }

    async fn ensure_stock_logs_table(pool: &PgPool) {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS stock_logs (
                id BIGINT PRIMARY KEY,
                tenant_id UUID NOT NULL,
                product_id BIGINT NOT NULL,
                biz_type VARCHAR(32) NOT NULL,
                biz_no VARCHAR(64) NOT NULL,
                delta_qty INTEGER NOT NULL,
                snapshot_stock INTEGER NOT NULL,
                snapshot_cost NUMERIC(18,4) NOT NULL DEFAULT 0,
                snapshot_sell_price NUMERIC(18,4) NULL,
                snapshot_inbound_unit_cost NUMERIC(18,4) NULL,
                operator_id UUID NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("ensure stock_logs table for repository tests");
    }

    async fn ensure_audit_logs_table(pool: &PgPool) {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_logs (
                id BIGINT PRIMARY KEY,
                tenant_id UUID NOT NULL,
                operator_id UUID NOT NULL,
                action VARCHAR(64) NOT NULL,
                target_type VARCHAR(64) NOT NULL,
                target_id VARCHAR(64) NOT NULL,
                before_data JSONB NOT NULL DEFAULT '{}'::jsonb,
                after_data JSONB NOT NULL DEFAULT '{}'::jsonb,
                request_id VARCHAR(128) NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                CONSTRAINT audit_logs_operator_fk
                    FOREIGN KEY (operator_id) REFERENCES users(id)
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("ensure audit_logs table for repository tests");
    }

    async fn ensure_purchase_orders_tables(pool: &PgPool) {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS purchase_orders (
                id BIGINT PRIMARY KEY,
                tenant_id UUID NOT NULL,
                biz_no VARCHAR(64) NOT NULL,
                supplier_id BIGINT NULL,
                status VARCHAR(32) NOT NULL,
                remark TEXT NULL,
                created_by UUID NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                confirmed_at TIMESTAMPTZ NULL,
                voided_at TIMESTAMPTZ NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                CONSTRAINT purchase_orders_status_check
                    CHECK (status IN ('DRAFT', 'CONFIRMED', 'VOIDED')),
                CONSTRAINT purchase_orders_created_by_fk
                    FOREIGN KEY (created_by) REFERENCES users(id)
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("ensure purchase_orders table for repository tests");

        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS ux_purchase_orders_tenant_biz_no ON purchase_orders (tenant_id, biz_no)",
        )
        .execute(pool)
        .await
        .expect("ensure purchase_orders unique index for repository tests");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS purchase_order_items (
                id BIGSERIAL PRIMARY KEY,
                tenant_id UUID NOT NULL,
                purchase_order_id BIGINT NOT NULL,
                product_id BIGINT NOT NULL,
                qty INTEGER NOT NULL,
                unit_cost NUMERIC(18,4) NOT NULL,
                product_name_snapshot TEXT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                CONSTRAINT purchase_order_items_qty_check CHECK (qty > 0),
                CONSTRAINT purchase_order_items_unit_cost_check CHECK (unit_cost >= 0),
                CONSTRAINT purchase_order_items_order_fk
                    FOREIGN KEY (purchase_order_id) REFERENCES purchase_orders(id) ON DELETE CASCADE,
                CONSTRAINT purchase_order_items_product_fk
                    FOREIGN KEY (product_id) REFERENCES products(id)
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("ensure purchase_order_items table for repository tests");

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS ix_purchase_order_items_tenant_order ON purchase_order_items (tenant_id, purchase_order_id)",
        )
        .execute(pool)
        .await
        .expect("ensure purchase_order_items order index for repository tests");
    }

    async fn ensure_sales_orders_tables(pool: &PgPool) {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sales_orders (
                id BIGINT PRIMARY KEY,
                tenant_id UUID NOT NULL,
                biz_no VARCHAR(64) NOT NULL,
                customer_id BIGINT NULL,
                status VARCHAR(32) NOT NULL,
                remark TEXT NULL,
                created_by UUID NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                confirmed_at TIMESTAMPTZ NULL,
                returned_at TIMESTAMPTZ NULL,
                voided_at TIMESTAMPTZ NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                CONSTRAINT sales_orders_status_check
                    CHECK (status IN ('DRAFT', 'CONFIRMED', 'RETURNED_PARTIAL', 'RETURNED_FULL', 'VOIDED')),
                CONSTRAINT sales_orders_created_by_fk
                    FOREIGN KEY (created_by) REFERENCES users(id)
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("ensure sales_orders table for repository tests");

        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS ux_sales_orders_tenant_biz_no ON sales_orders (tenant_id, biz_no)",
        )
        .execute(pool)
        .await
        .expect("ensure sales_orders unique index for repository tests");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sales_order_items (
                id BIGSERIAL PRIMARY KEY,
                tenant_id UUID NOT NULL,
                sales_order_id BIGINT NOT NULL,
                product_id BIGINT NOT NULL,
                qty INTEGER NOT NULL,
                sell_price NUMERIC(18,4) NOT NULL,
                returned_qty INTEGER NOT NULL DEFAULT 0,
                product_name_snapshot TEXT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                CONSTRAINT sales_order_items_qty_check CHECK (qty > 0),
                CONSTRAINT sales_order_items_sell_price_check CHECK (sell_price >= 0),
                CONSTRAINT sales_order_items_returned_qty_check
                    CHECK (returned_qty >= 0 AND returned_qty <= qty),
                CONSTRAINT sales_order_items_order_fk
                    FOREIGN KEY (sales_order_id) REFERENCES sales_orders(id) ON DELETE CASCADE,
                CONSTRAINT sales_order_items_product_fk
                    FOREIGN KEY (product_id) REFERENCES products(id)
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("ensure sales_order_items table for repository tests");

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS ix_sales_order_items_tenant_order ON sales_order_items (tenant_id, sales_order_id)",
        )
        .execute(pool)
        .await
        .expect("ensure sales_order_items order index for repository tests");
    }

    async fn ensure_stock_checks_tables(pool: &PgPool) {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS stock_checks (
                id BIGINT PRIMARY KEY,
                tenant_id UUID NOT NULL,
                biz_no VARCHAR(64) NOT NULL,
                status VARCHAR(32) NOT NULL,
                remark TEXT NULL,
                created_by UUID NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                counting_at TIMESTAMPTZ NULL,
                confirmed_at TIMESTAMPTZ NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                CONSTRAINT stock_checks_status_check
                    CHECK (status IN ('DRAFT', 'COUNTING', 'CONFIRMED')),
                CONSTRAINT stock_checks_created_by_fk
                    FOREIGN KEY (created_by) REFERENCES users(id)
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("ensure stock_checks table for repository tests");

        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS ux_stock_checks_tenant_biz_no ON stock_checks (tenant_id, biz_no)",
        )
        .execute(pool)
        .await
        .expect("ensure stock_checks unique index for repository tests");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS stock_check_items (
                id BIGSERIAL PRIMARY KEY,
                tenant_id UUID NOT NULL,
                stock_check_id BIGINT NOT NULL,
                product_id BIGINT NOT NULL,
                book_stock INTEGER NOT NULL,
                actual_stock INTEGER NULL,
                delta_qty INTEGER NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                CONSTRAINT stock_check_items_order_fk
                    FOREIGN KEY (stock_check_id) REFERENCES stock_checks(id) ON DELETE CASCADE,
                CONSTRAINT stock_check_items_product_fk
                    FOREIGN KEY (product_id) REFERENCES products(id)
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("ensure stock_check_items table for repository tests");

        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS ux_stock_check_items_tenant_order_product ON stock_check_items (tenant_id, stock_check_id, product_id)",
        )
        .execute(pool)
        .await
        .expect("ensure stock_check_items unique index for repository tests");
    }

    fn sample_product(tenant_id: Uuid, product_id: i64, version: i32) -> Product {
        Product {
            id: product_id,
            tenant_id,
            sku: format!("SKU-{product_id}"),
            barcode: format!("690{:09}", product_id % 1_000_000_000),
            name: format!("测试商品-{product_id}"),
            unit: "件".to_string(),
            current_stock: 50,
            cost_price: Decimal::new(210, 2),
            retail_price: Decimal::new(350, 2),
            last_inbound_unit_cost: Some(Decimal::new(320, 2)),
            min_stock_limit: 5,
            version,
            is_deleted: false,
        }
    }

    fn sample_purchase_order(
        tenant_id: Uuid,
        order_id: i64,
        product_id: i64,
        created_by: Uuid,
        version: i32,
    ) -> PurchaseOrder {
        let now = Utc::now().to_rfc3339();
        PurchaseOrder {
            id: order_id,
            tenant_id,
            biz_no: format!("PO-REG-{order_id}"),
            supplier_id: Some(1001),
            status: PurchaseOrderStatus::Draft,
            items: vec![PurchaseOrderItem {
                product_id,
                qty: 5,
                unit_cost: Decimal::new(250, 2),
                product_name_snapshot: Some("回归测试采购商品".to_string()),
            }],
            remark: Some("仓储回归采购单".to_string()),
            created_by,
            version,
            confirmed_at: None,
            voided_at: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    fn sample_sales_order(
        tenant_id: Uuid,
        order_id: i64,
        product_id: i64,
        created_by: Uuid,
        version: i32,
        qty: i32,
    ) -> SalesOrder {
        let now = Utc::now().to_rfc3339();
        SalesOrder {
            id: order_id,
            tenant_id,
            biz_no: format!("SO-REG-{order_id}"),
            customer_id: Some(2001),
            status: SalesOrderStatus::Draft,
            items: vec![SalesOrderItem {
                product_id,
                qty,
                sell_price: Decimal::new(350, 2),
                returned_qty: 0,
                product_name_snapshot: Some("回归测试销售商品".to_string()),
            }],
            remark: Some("仓储回归销售单".to_string()),
            created_by,
            version,
            confirmed_at: None,
            returned_at: None,
            voided_at: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    fn sample_stock_check(
        tenant_id: Uuid,
        check_id: i64,
        product_id: i64,
        created_by: Uuid,
        version: i32,
        book_stock: i32,
    ) -> StockCheck {
        let now = Utc::now().to_rfc3339();
        StockCheck {
            id: check_id,
            tenant_id,
            biz_no: format!("SC-REG-{check_id}"),
            status: StockCheckStatus::Draft,
            items: vec![StockCheckItem {
                product_id,
                book_stock,
                actual_stock: None,
                delta_qty: None,
            }],
            remark: Some("仓储回归盘点单".to_string()),
            created_by,
            version,
            counting_at: None,
            confirmed_at: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    async fn cleanup_tenant_products(pool: &PgPool, tenant_id: Uuid) {
        sqlx::query("DELETE FROM products WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await
            .expect("cleanup tenant products after repository tests");
    }

    async fn ensure_operator_user(pool: &PgPool, tenant_id: Uuid, operator_id: Uuid) {
        let username = format!("repo_test_{}", operator_id.simple());
        sqlx::query(
            r#"
            INSERT INTO users (id, tenant_id, username, name, role, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(operator_id)
        .bind(tenant_id)
        .bind(username)
        .bind("仓储回归测试用户")
        .bind("OWNER")
        .bind("repo-test-password-hash")
        .execute(pool)
        .await
        .expect("ensure operator user for repository tests");
    }

    async fn cleanup_tenant_stock_logs(pool: &PgPool, tenant_id: Uuid) {
        sqlx::query("DELETE FROM stock_logs WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await
            .expect("cleanup tenant stock logs after repository tests");
    }

    async fn cleanup_tenant_audit_logs(pool: &PgPool, tenant_id: Uuid) {
        sqlx::query("DELETE FROM audit_logs WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await
            .expect("cleanup tenant audit logs after repository tests");
    }

    async fn cleanup_tenant_purchase_orders(pool: &PgPool, tenant_id: Uuid) {
        sqlx::query("DELETE FROM purchase_order_items WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await
            .expect("cleanup tenant purchase_order_items after repository tests");

        sqlx::query("DELETE FROM purchase_orders WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await
            .expect("cleanup tenant purchase_orders after repository tests");
    }

    async fn cleanup_tenant_sales_orders(pool: &PgPool, tenant_id: Uuid) {
        sqlx::query("DELETE FROM sales_order_items WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await
            .expect("cleanup tenant sales_order_items after repository tests");

        sqlx::query("DELETE FROM sales_orders WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await
            .expect("cleanup tenant sales_orders after repository tests");
    }

    async fn cleanup_tenant_stock_checks(pool: &PgPool, tenant_id: Uuid) {
        sqlx::query("DELETE FROM stock_check_items WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await
            .expect("cleanup tenant stock_check_items after repository tests");

        sqlx::query("DELETE FROM stock_checks WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await
            .expect("cleanup tenant stock_checks after repository tests");
    }

    async fn cleanup_test_user(pool: &PgPool, user_id: Uuid) {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .expect("cleanup test user after repository tests");
    }

    async fn cleanup_inventory_fixture(pool: &PgPool, tenant_id: Uuid, operator_id: Uuid) {
        cleanup_tenant_stock_logs(pool, tenant_id).await;
        cleanup_tenant_audit_logs(pool, tenant_id).await;
        cleanup_tenant_sales_orders(pool, tenant_id).await;
        cleanup_tenant_purchase_orders(pool, tenant_id).await;
        cleanup_tenant_stock_checks(pool, tenant_id).await;
        cleanup_tenant_products(pool, tenant_id).await;
        cleanup_test_user(pool, operator_id).await;
    }

    async fn tenant_stock_log_count(pool: &PgPool, tenant_id: Uuid) -> i64 {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::BIGINT FROM stock_logs WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_one(pool)
            .await
            .expect("count tenant stock logs")
    }

    #[tokio::test]
    async fn postgres_update_product_stale_expected_version_returns_4091_with_latest_snapshot() {
        let Some(pool) = setup_pg_pool().await else {
            return;
        };

        let repo = PostgresRepository;
        let tenant_id = Uuid::new_v4();
        let product_id = next_test_product_id();
        let original = sample_product(tenant_id, product_id, 1);
        repo.create_product(Some(&pool), &original)
            .await
            .expect("insert original product");

        let mut first_update = original.clone();
        first_update.name = format!("{}-first", original.name);
        first_update.version = original.version + 1;
        repo.update_product(Some(&pool), &first_update, Some(original.version))
            .await
            .expect("first update should succeed");

        let mut stale_update = original.clone();
        stale_update.name = format!("{}-stale", original.name);
        stale_update.version = original.version + 1;

        let err = repo
            .update_product(Some(&pool), &stale_update, Some(original.version))
            .await
            .expect_err("second stale update should conflict");

        assert_eq!(err.status, StatusCode::CONFLICT);
        assert_eq!(err.code, 4091);
        assert_eq!(err.data["resource"], json!("product"));
        assert_eq!(err.data["resource_id"], json!(product_id));
        assert_eq!(err.data["expected_version"], json!(original.version));
        assert_eq!(err.data["current_version"], json!(first_update.version));
        assert_eq!(
            err.data["latest_snapshot"]["version"],
            json!(first_update.version)
        );
        assert_eq!(
            err.data["latest_snapshot"]["current_stock"],
            json!(first_update.current_stock)
        );

        cleanup_tenant_products(&pool, tenant_id).await;
    }

    #[tokio::test]
    async fn postgres_update_product_missing_resource_returns_4040_instead_of_4091() {
        let Some(pool) = setup_pg_pool().await else {
            return;
        };

        let repo = PostgresRepository;
        let tenant_id = Uuid::new_v4();
        let product_id = next_test_product_id();

        let mut missing = sample_product(tenant_id, product_id, 1);
        missing.name = format!("{}-missing", missing.name);
        missing.version = 2;

        let err = repo
            .update_product(Some(&pool), &missing, Some(1))
            .await
            .expect_err("missing product should be 404");

        assert_eq!(err.status, StatusCode::NOT_FOUND);
        assert_eq!(err.code, 4040);

        cleanup_tenant_products(&pool, tenant_id).await;
    }

    #[tokio::test]
    async fn postgres_inbound_and_outbound_stale_expected_version_returns_4091_with_latest_snapshot()
     {
        let Some(pool) = setup_pg_pool().await else {
            return;
        };

        let repo = PostgresRepository;
        let tenant_id = Uuid::new_v4();
        let operator_id = Uuid::new_v4();
        ensure_operator_user(&pool, tenant_id, operator_id).await;

        let product_id = next_test_product_id();
        let original = sample_product(tenant_id, product_id, 1);
        repo.create_product(Some(&pool), &original)
            .await
            .expect("insert product for inbound/outbound optimistic-lock regression");

        let first_inbound = repo
            .inbound(
                Some(&pool),
                tenant_id,
                product_id,
                5,
                Some(Decimal::new(300, 2)),
                Some(original.version),
                "PO-REG-OL-001",
                operator_id,
            )
            .await
            .expect("first inbound should succeed");

        let inbound_err = repo
            .inbound(
                Some(&pool),
                tenant_id,
                product_id,
                1,
                Some(Decimal::new(280, 2)),
                Some(original.version),
                "PO-REG-OL-002",
                operator_id,
            )
            .await
            .expect_err("stale inbound expected_version should conflict");

        assert_eq!(inbound_err.status, StatusCode::CONFLICT);
        assert_eq!(inbound_err.code, 4091);
        assert_eq!(inbound_err.data["resource"], json!("product"));
        assert_eq!(inbound_err.data["resource_id"], json!(product_id));
        assert_eq!(
            inbound_err.data["expected_version"],
            json!(original.version)
        );
        assert_eq!(
            inbound_err.data["current_version"],
            json!(first_inbound.version)
        );
        assert_eq!(
            inbound_err.data["latest_snapshot"]["version"],
            json!(first_inbound.version)
        );
        assert_eq!(
            inbound_err.data["latest_snapshot"]["current_stock"],
            json!(first_inbound.current_stock)
        );

        let outbound_items = vec![(product_id, 3, None, None)];
        let first_outbound = repo
            .outbound(
                Some(&pool),
                tenant_id,
                "SO-REG-OL-001",
                Some(first_inbound.version),
                &outbound_items,
                false,
                operator_id,
            )
            .await
            .expect("first outbound should succeed");
        assert_eq!(first_outbound.len(), 1);
        let outbound_snapshot = first_outbound[0].clone();

        let stale_outbound_items = vec![(product_id, 1, None, None)];
        let outbound_err = repo
            .outbound(
                Some(&pool),
                tenant_id,
                "SO-REG-OL-002",
                Some(first_inbound.version),
                &stale_outbound_items,
                false,
                operator_id,
            )
            .await
            .expect_err("stale outbound expected_version should conflict");

        assert_eq!(outbound_err.status, StatusCode::CONFLICT);
        assert_eq!(outbound_err.code, 4091);
        assert_eq!(outbound_err.data["resource"], json!("product"));
        assert_eq!(outbound_err.data["resource_id"], json!(product_id));
        assert_eq!(
            outbound_err.data["expected_version"],
            json!(first_inbound.version)
        );
        assert_eq!(
            outbound_err.data["current_version"],
            json!(outbound_snapshot.version)
        );
        assert_eq!(
            outbound_err.data["latest_snapshot"]["version"],
            json!(outbound_snapshot.version)
        );
        assert_eq!(
            outbound_err.data["latest_snapshot"]["current_stock"],
            json!(outbound_snapshot.current_stock)
        );

        cleanup_inventory_fixture(&pool, tenant_id, operator_id).await;
    }

    #[tokio::test]
    async fn postgres_outbound_insufficient_stock_returns_4001_without_mutating_product() {
        let Some(pool) = setup_pg_pool().await else {
            return;
        };

        let repo = PostgresRepository;
        let tenant_id = Uuid::new_v4();
        let operator_id = Uuid::new_v4();
        ensure_operator_user(&pool, tenant_id, operator_id).await;

        let product_id = next_test_product_id();
        let mut product = sample_product(tenant_id, product_id, 1);
        product.current_stock = 2;
        product.cost_price = Decimal::new(180, 2);
        repo.create_product(Some(&pool), &product)
            .await
            .expect("insert product for outbound insufficient-stock regression");

        let outbound_items = vec![(product_id, 5, None, None)];
        let err = repo
            .outbound(
                Some(&pool),
                tenant_id,
                "SO-REG-STOCK-001",
                Some(product.version),
                &outbound_items,
                false,
                operator_id,
            )
            .await
            .expect_err("outbound should fail when stock is insufficient");

        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.code, 4001);
        assert_eq!(err.data["failed_product_id"], json!(product_id));
        assert_eq!(err.data["available_stock"], json!(product.current_stock));
        assert_eq!(err.data["required_qty"], json!(5));

        let latest = repo
            .find_product_by_id(Some(&pool), tenant_id, product_id, false)
            .await
            .expect("load latest product after failed outbound")
            .expect("product should still exist after failed outbound");
        assert_eq!(latest.current_stock, product.current_stock);
        assert_eq!(latest.version, product.version);

        let stock_log_count = tenant_stock_log_count(&pool, tenant_id).await;
        assert_eq!(stock_log_count, 0);

        cleanup_inventory_fixture(&pool, tenant_id, operator_id).await;
    }

    #[tokio::test]
    async fn postgres_confirm_purchase_order_stale_expected_version_returns_4091_with_latest_snapshot()
     {
        let Some(pool) = setup_pg_pool().await else {
            return;
        };

        let repo = PostgresRepository;
        let tenant_id = Uuid::new_v4();
        let operator_id = Uuid::new_v4();
        ensure_operator_user(&pool, tenant_id, operator_id).await;

        let product_id = next_test_product_id();
        let product = sample_product(tenant_id, product_id, 1);
        repo.create_product(Some(&pool), &product)
            .await
            .expect("insert product for confirm purchase-order optimistic-lock regression");

        let order_id = next_test_purchase_order_id();
        let order = sample_purchase_order(tenant_id, order_id, product_id, operator_id, 1);
        repo.create_purchase_order(Some(&pool), &order)
            .await
            .expect("insert draft purchase order for confirm optimistic-lock regression");

        let confirmed = repo
            .confirm_purchase_order(
                Some(&pool),
                tenant_id,
                order_id,
                Some(order.version),
                operator_id,
                "req_repo_pg_po_confirm_001",
            )
            .await
            .expect("first purchase-order confirm should succeed");
        assert_eq!(confirmed.status, PurchaseOrderStatus::Confirmed);
        assert_eq!(confirmed.version, order.version + 1);

        let stale_err = repo
            .confirm_purchase_order(
                Some(&pool),
                tenant_id,
                order_id,
                Some(order.version),
                operator_id,
                "req_repo_pg_po_confirm_002",
            )
            .await
            .expect_err("stale purchase-order expected_version should conflict");

        assert_eq!(stale_err.status, StatusCode::CONFLICT);
        assert_eq!(stale_err.code, 4091);
        assert_eq!(stale_err.data["resource"], json!("purchase_order"));
        assert_eq!(stale_err.data["resource_id"], json!(order_id));
        assert_eq!(stale_err.data["expected_version"], json!(order.version));
        assert_eq!(stale_err.data["current_version"], json!(confirmed.version));
        assert_eq!(
            stale_err.data["latest_snapshot"]["status"],
            json!(confirmed.status.as_str())
        );
        assert_eq!(
            stale_err.data["latest_snapshot"]["version"],
            json!(confirmed.version)
        );

        let stock_log_count = tenant_stock_log_count(&pool, tenant_id).await;
        assert_eq!(stock_log_count, 1);

        cleanup_inventory_fixture(&pool, tenant_id, operator_id).await;
    }

    #[tokio::test]
    async fn postgres_confirm_sales_order_insufficient_stock_returns_4001_without_mutating_order_and_product()
     {
        let Some(pool) = setup_pg_pool().await else {
            return;
        };

        let repo = PostgresRepository;
        let tenant_id = Uuid::new_v4();
        let operator_id = Uuid::new_v4();
        ensure_operator_user(&pool, tenant_id, operator_id).await;

        let product_id = next_test_product_id();
        let mut product = sample_product(tenant_id, product_id, 1);
        product.current_stock = 2;
        product.cost_price = Decimal::new(180, 2);
        repo.create_product(Some(&pool), &product)
            .await
            .expect("insert product for confirm sales-order insufficient-stock regression");

        let order_id = next_test_sales_order_id();
        let order = sample_sales_order(tenant_id, order_id, product_id, operator_id, 1, 5);
        repo.create_sales_order(Some(&pool), &order)
            .await
            .expect("insert draft sales order for insufficient-stock regression");

        let err = repo
            .confirm_sales_order(
                Some(&pool),
                tenant_id,
                order_id,
                Some(order.version),
                false,
                operator_id,
                "req_repo_pg_so_confirm_001",
            )
            .await
            .expect_err("confirm sales-order should fail when stock is insufficient");

        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.code, 4001);
        assert_eq!(err.data["failed_product_id"], json!(product_id));
        assert_eq!(err.data["available_stock"], json!(product.current_stock));
        assert_eq!(err.data["required_qty"], json!(5));

        let latest_order = repo
            .find_sales_order_by_id(Some(&pool), tenant_id, order_id)
            .await
            .expect("load latest sales-order after failed confirm")
            .expect("sales-order should still exist after failed confirm");
        assert_eq!(latest_order.status, SalesOrderStatus::Draft);
        assert_eq!(latest_order.version, order.version);
        assert_eq!(latest_order.confirmed_at, None);

        let latest_product = repo
            .find_product_by_id(Some(&pool), tenant_id, product_id, false)
            .await
            .expect("load latest product after failed sales-order confirm")
            .expect("product should still exist after failed sales-order confirm");
        assert_eq!(latest_product.current_stock, product.current_stock);
        assert_eq!(latest_product.version, product.version);

        let stock_log_count = tenant_stock_log_count(&pool, tenant_id).await;
        assert_eq!(stock_log_count, 0);

        cleanup_inventory_fixture(&pool, tenant_id, operator_id).await;
    }

    #[tokio::test]
    async fn postgres_void_purchase_order_stale_expected_version_returns_4091_with_latest_snapshot()
    {
        let Some(pool) = setup_pg_pool().await else {
            return;
        };

        let repo = PostgresRepository;
        let tenant_id = Uuid::new_v4();
        let operator_id = Uuid::new_v4();
        ensure_operator_user(&pool, tenant_id, operator_id).await;

        let product_id = next_test_product_id();
        let product = sample_product(tenant_id, product_id, 1);
        repo.create_product(Some(&pool), &product)
            .await
            .expect("insert product for void purchase-order optimistic-lock regression");

        let order_id = next_test_purchase_order_id();
        let order = sample_purchase_order(tenant_id, order_id, product_id, operator_id, 1);
        repo.create_purchase_order(Some(&pool), &order)
            .await
            .expect("insert draft purchase order for void optimistic-lock regression");

        let confirmed = repo
            .confirm_purchase_order(
                Some(&pool),
                tenant_id,
                order_id,
                Some(order.version),
                operator_id,
                "req_repo_pg_po_confirm_101",
            )
            .await
            .expect("purchase-order confirm should succeed before void stale-check");
        assert_eq!(confirmed.status, PurchaseOrderStatus::Confirmed);

        let stale_err = repo
            .void_purchase_order(
                Some(&pool),
                tenant_id,
                order_id,
                Some(order.version),
                false,
                operator_id,
                "req_repo_pg_po_void_101",
            )
            .await
            .expect_err("stale purchase-order expected_version should conflict on void");

        assert_eq!(stale_err.status, StatusCode::CONFLICT);
        assert_eq!(stale_err.code, 4091);
        assert_eq!(stale_err.data["resource"], json!("purchase_order"));
        assert_eq!(stale_err.data["resource_id"], json!(order_id));
        assert_eq!(stale_err.data["expected_version"], json!(order.version));
        assert_eq!(stale_err.data["current_version"], json!(confirmed.version));
        assert_eq!(
            stale_err.data["latest_snapshot"]["status"],
            json!(confirmed.status.as_str())
        );
        assert_eq!(
            stale_err.data["latest_snapshot"]["version"],
            json!(confirmed.version)
        );

        let stock_log_count = tenant_stock_log_count(&pool, tenant_id).await;
        assert_eq!(stock_log_count, 1);

        cleanup_inventory_fixture(&pool, tenant_id, operator_id).await;
    }

    #[tokio::test]
    async fn postgres_void_purchase_order_insufficient_stock_returns_4001_without_mutating_order_and_product()
     {
        let Some(pool) = setup_pg_pool().await else {
            return;
        };

        let repo = PostgresRepository;
        let tenant_id = Uuid::new_v4();
        let operator_id = Uuid::new_v4();
        ensure_operator_user(&pool, tenant_id, operator_id).await;

        let product_id = next_test_product_id();
        let mut product = sample_product(tenant_id, product_id, 1);
        product.current_stock = 0;
        product.cost_price = Decimal::new(180, 2);
        repo.create_product(Some(&pool), &product)
            .await
            .expect("insert product for void purchase-order insufficient-stock regression");

        let order_id = next_test_purchase_order_id();
        let order = sample_purchase_order(tenant_id, order_id, product_id, operator_id, 1);
        repo.create_purchase_order(Some(&pool), &order)
            .await
            .expect("insert draft purchase order for insufficient-stock void regression");

        let confirmed = repo
            .confirm_purchase_order(
                Some(&pool),
                tenant_id,
                order_id,
                Some(order.version),
                operator_id,
                "req_repo_pg_po_confirm_201",
            )
            .await
            .expect("purchase-order confirm should succeed before insufficient-stock void");
        assert_eq!(confirmed.status, PurchaseOrderStatus::Confirmed);

        let product_after_confirm = repo
            .find_product_by_id(Some(&pool), tenant_id, product_id, false)
            .await
            .expect("load product after purchase-order confirm")
            .expect("product should exist after purchase-order confirm");
        assert_eq!(product_after_confirm.current_stock, 5);

        let outbound_items = vec![(product_id, 4, None, None)];
        let outbound_products = repo
            .outbound(
                Some(&pool),
                tenant_id,
                "SO-REG-VOID-001",
                Some(product_after_confirm.version),
                &outbound_items,
                false,
                operator_id,
            )
            .await
            .expect("outbound should reduce stock before void purchase-order");
        assert_eq!(outbound_products.len(), 1);
        let outbound_snapshot = outbound_products[0].clone();
        assert_eq!(outbound_snapshot.current_stock, 1);

        let err = repo
            .void_purchase_order(
                Some(&pool),
                tenant_id,
                order_id,
                Some(confirmed.version),
                false,
                operator_id,
                "req_repo_pg_po_void_201",
            )
            .await
            .expect_err("void purchase-order should fail when rollback stock is insufficient");

        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.code, 4001);
        assert_eq!(err.data["failed_product_id"], json!(product_id));
        assert_eq!(
            err.data["available_stock"],
            json!(outbound_snapshot.current_stock)
        );
        assert_eq!(err.data["required_qty"], json!(5));

        let latest_order = repo
            .find_purchase_order_by_id(Some(&pool), tenant_id, order_id)
            .await
            .expect("load latest purchase-order after failed void")
            .expect("purchase-order should still exist after failed void");
        assert_eq!(latest_order.status, PurchaseOrderStatus::Confirmed);
        assert_eq!(latest_order.version, confirmed.version);
        assert_eq!(latest_order.voided_at, None);

        let latest_product = repo
            .find_product_by_id(Some(&pool), tenant_id, product_id, false)
            .await
            .expect("load latest product after failed purchase-order void")
            .expect("product should still exist after failed purchase-order void");
        assert_eq!(
            latest_product.current_stock,
            outbound_snapshot.current_stock
        );
        assert_eq!(latest_product.version, outbound_snapshot.version);

        let stock_log_count = tenant_stock_log_count(&pool, tenant_id).await;
        assert_eq!(stock_log_count, 2);

        cleanup_inventory_fixture(&pool, tenant_id, operator_id).await;
    }

    #[tokio::test]
    async fn postgres_start_stock_check_stale_expected_version_returns_4091_with_latest_snapshot() {
        let Some(pool) = setup_pg_pool().await else {
            return;
        };

        let repo = PostgresRepository;
        let tenant_id = Uuid::new_v4();
        let operator_id = Uuid::new_v4();
        ensure_operator_user(&pool, tenant_id, operator_id).await;

        let product_id = next_test_product_id();
        let product = sample_product(tenant_id, product_id, 1);
        repo.create_product(Some(&pool), &product)
            .await
            .expect("insert product for start stock-check optimistic-lock regression");

        let check_id = next_test_stock_check_id();
        let check = sample_stock_check(
            tenant_id,
            check_id,
            product_id,
            operator_id,
            1,
            product.current_stock,
        );
        repo.create_stock_check(Some(&pool), &check)
            .await
            .expect("insert draft stock-check for start optimistic-lock regression");

        let started = repo
            .start_stock_check(
                Some(&pool),
                tenant_id,
                check_id,
                Some(check.version),
                operator_id,
                "req_repo_pg_sc_start_001",
            )
            .await
            .expect("first stock-check start should succeed");
        assert_eq!(started.status, StockCheckStatus::Counting);
        assert_eq!(started.version, check.version + 1);

        let stale_err = repo
            .start_stock_check(
                Some(&pool),
                tenant_id,
                check_id,
                Some(check.version),
                operator_id,
                "req_repo_pg_sc_start_002",
            )
            .await
            .expect_err("stale stock-check expected_version should conflict on start");

        assert_eq!(stale_err.status, StatusCode::CONFLICT);
        assert_eq!(stale_err.code, 4091);
        assert_eq!(stale_err.data["resource"], json!("stock_check"));
        assert_eq!(stale_err.data["resource_id"], json!(check_id));
        assert_eq!(stale_err.data["expected_version"], json!(check.version));
        assert_eq!(stale_err.data["current_version"], json!(started.version));
        assert_eq!(
            stale_err.data["latest_snapshot"]["status"],
            json!(started.status.as_str())
        );
        assert_eq!(
            stale_err.data["latest_snapshot"]["version"],
            json!(started.version)
        );

        let latest_check = repo
            .find_stock_check_by_id(Some(&pool), tenant_id, check_id)
            .await
            .expect("load latest stock-check after failed stale start")
            .expect("stock-check should still exist after failed stale start");
        assert_eq!(latest_check.status, StockCheckStatus::Counting);
        assert_eq!(latest_check.version, started.version);

        let stock_log_count = tenant_stock_log_count(&pool, tenant_id).await;
        assert_eq!(stock_log_count, 0);

        cleanup_inventory_fixture(&pool, tenant_id, operator_id).await;
    }

    #[tokio::test]
    async fn postgres_confirm_stock_check_book_stock_drift_returns_4091_without_mutating_check_and_product()
     {
        let Some(pool) = setup_pg_pool().await else {
            return;
        };

        let repo = PostgresRepository;
        let tenant_id = Uuid::new_v4();
        let operator_id = Uuid::new_v4();
        ensure_operator_user(&pool, tenant_id, operator_id).await;

        let product_id = next_test_product_id();
        let product = sample_product(tenant_id, product_id, 1);
        repo.create_product(Some(&pool), &product)
            .await
            .expect("insert product for stock-check book-stock-drift regression");

        let check_id = next_test_stock_check_id();
        let check = sample_stock_check(
            tenant_id,
            check_id,
            product_id,
            operator_id,
            1,
            product.current_stock,
        );
        repo.create_stock_check(Some(&pool), &check)
            .await
            .expect("insert draft stock-check for book-stock-drift regression");

        let started = repo
            .start_stock_check(
                Some(&pool),
                tenant_id,
                check_id,
                Some(check.version),
                operator_id,
                "req_repo_pg_sc_confirm_001",
            )
            .await
            .expect("stock-check start should succeed before book-stock-drift confirm");
        assert_eq!(started.status, StockCheckStatus::Counting);

        let outbound_items = vec![(product_id, 1, None, None)];
        let drifted_products = repo
            .outbound(
                Some(&pool),
                tenant_id,
                "SO-REG-SC-001",
                Some(product.version),
                &outbound_items,
                false,
                operator_id,
            )
            .await
            .expect("outbound should mutate product stock before confirm stock-check");
        assert_eq!(drifted_products.len(), 1);
        let drifted_product = drifted_products[0].clone();
        assert_eq!(drifted_product.current_stock, product.current_stock - 1);

        let actual_items = vec![(product_id, drifted_product.current_stock)];
        let err = repo
            .confirm_stock_check(
                Some(&pool),
                tenant_id,
                check_id,
                Some(started.version),
                &actual_items,
                Some("book_stock drift regression".to_string()),
                operator_id,
                "req_repo_pg_sc_confirm_002",
            )
            .await
            .expect_err("confirm stock-check should fail when book_stock has drifted");

        assert_eq!(err.status, StatusCode::CONFLICT);
        assert_eq!(err.code, 4091);
        assert_eq!(err.data["resource"], json!("product"));
        assert_eq!(err.data["resource_id"], json!(product_id));
        assert_eq!(err.data["book_stock"], json!(product.current_stock));
        assert_eq!(
            err.data["current_stock"],
            json!(drifted_product.current_stock)
        );
        assert_eq!(
            err.data["latest_snapshot"]["version"],
            json!(drifted_product.version)
        );

        let latest_check = repo
            .find_stock_check_by_id(Some(&pool), tenant_id, check_id)
            .await
            .expect("load latest stock-check after failed confirm")
            .expect("stock-check should still exist after failed confirm");
        assert_eq!(latest_check.status, StockCheckStatus::Counting);
        assert_eq!(latest_check.version, started.version);
        assert_eq!(latest_check.confirmed_at, None);
        assert_eq!(latest_check.items.len(), 1);
        assert_eq!(latest_check.items[0].actual_stock, None);
        assert_eq!(latest_check.items[0].delta_qty, None);

        let latest_product = repo
            .find_product_by_id(Some(&pool), tenant_id, product_id, false)
            .await
            .expect("load latest product after failed stock-check confirm")
            .expect("product should still exist after failed stock-check confirm");
        assert_eq!(latest_product.current_stock, drifted_product.current_stock);
        assert_eq!(latest_product.version, drifted_product.version);

        let stock_log_count = tenant_stock_log_count(&pool, tenant_id).await;
        assert_eq!(stock_log_count, 1);

        cleanup_inventory_fixture(&pool, tenant_id, operator_id).await;
    }

    #[tokio::test]
    async fn postgres_void_sales_order_non_draft_returns_4090_without_mutating_order_and_stock_logs()
     {
        let Some(pool) = setup_pg_pool().await else {
            return;
        };

        let repo = PostgresRepository;
        let tenant_id = Uuid::new_v4();
        let operator_id = Uuid::new_v4();
        ensure_operator_user(&pool, tenant_id, operator_id).await;

        let product_id = next_test_product_id();
        let mut product = sample_product(tenant_id, product_id, 1);
        product.current_stock = 10;
        product.cost_price = Decimal::new(200, 2);
        repo.create_product(Some(&pool), &product)
            .await
            .expect("insert product for void-sales-order non-draft regression");

        let order_id = next_test_sales_order_id();
        let order = sample_sales_order(tenant_id, order_id, product_id, operator_id, 1, 3);
        repo.create_sales_order(Some(&pool), &order)
            .await
            .expect("insert draft sales-order for void non-draft regression");

        let confirmed = repo
            .confirm_sales_order(
                Some(&pool),
                tenant_id,
                order_id,
                Some(order.version),
                false,
                operator_id,
                "req_repo_pg_so_void_001",
            )
            .await
            .expect("confirm sales-order before void non-draft regression");
        assert_eq!(confirmed.status, SalesOrderStatus::Confirmed);

        let err = repo
            .void_sales_order(
                Some(&pool),
                tenant_id,
                order_id,
                Some(confirmed.version),
                operator_id,
                "req_repo_pg_so_void_002",
            )
            .await
            .expect_err("void confirmed sales-order should be rejected with 4090");

        assert_eq!(err.status, StatusCode::CONFLICT);
        assert_eq!(err.code, 4090);
        assert_eq!(err.data["status"], json!(confirmed.status.as_str()));

        let latest_order = repo
            .find_sales_order_by_id(Some(&pool), tenant_id, order_id)
            .await
            .expect("load latest sales-order after failed void")
            .expect("sales-order should still exist after failed void");
        assert_eq!(latest_order.status, SalesOrderStatus::Confirmed);
        assert_eq!(latest_order.version, confirmed.version);
        assert_eq!(latest_order.voided_at, None);

        let stock_log_count = tenant_stock_log_count(&pool, tenant_id).await;
        assert_eq!(stock_log_count, 1);

        cleanup_inventory_fixture(&pool, tenant_id, operator_id).await;
    }

    #[tokio::test]
    async fn postgres_return_sales_order_over_remaining_qty_returns_4090_without_mutating_order_product_and_stock_logs()
     {
        let Some(pool) = setup_pg_pool().await else {
            return;
        };

        let repo = PostgresRepository;
        let tenant_id = Uuid::new_v4();
        let operator_id = Uuid::new_v4();
        ensure_operator_user(&pool, tenant_id, operator_id).await;

        let product_id = next_test_product_id();
        let mut product = sample_product(tenant_id, product_id, 1);
        product.current_stock = 10;
        product.cost_price = Decimal::new(200, 2);
        repo.create_product(Some(&pool), &product)
            .await
            .expect("insert product for return-sales-order over-remain regression");

        let order_id = next_test_sales_order_id();
        let order = sample_sales_order(tenant_id, order_id, product_id, operator_id, 1, 3);
        repo.create_sales_order(Some(&pool), &order)
            .await
            .expect("insert draft sales-order for return over-remain regression");

        let confirmed = repo
            .confirm_sales_order(
                Some(&pool),
                tenant_id,
                order_id,
                Some(order.version),
                false,
                operator_id,
                "req_repo_pg_so_return_001",
            )
            .await
            .expect("confirm sales-order before return over-remain regression");
        assert_eq!(confirmed.status, SalesOrderStatus::Confirmed);

        let before_failed_return = repo
            .find_sales_order_by_id(Some(&pool), tenant_id, order_id)
            .await
            .expect("load sales-order before failed return")
            .expect("sales-order should exist before failed return");
        let before_product = repo
            .find_product_by_id(Some(&pool), tenant_id, product_id, false)
            .await
            .expect("load product before failed return")
            .expect("product should exist before failed return");

        let err = repo
            .return_sales_order(
                Some(&pool),
                tenant_id,
                order_id,
                Some(before_failed_return.version),
                &[(product_id, 4)],
                Some("超额退货应失败".to_string()),
                operator_id,
                "req_repo_pg_so_return_002",
            )
            .await
            .expect_err("return qty greater than remain should be rejected with 4090");

        assert_eq!(err.status, StatusCode::CONFLICT);
        assert_eq!(err.code, 4090);
        assert_eq!(err.data["product_id"], json!(product_id));
        assert_eq!(err.data["remain_qty"], json!(3));
        assert_eq!(err.data["request_qty"], json!(4));

        let latest_order = repo
            .find_sales_order_by_id(Some(&pool), tenant_id, order_id)
            .await
            .expect("load latest sales-order after failed return")
            .expect("sales-order should still exist after failed return");
        assert_eq!(latest_order.status, before_failed_return.status);
        assert_eq!(latest_order.version, before_failed_return.version);
        assert_eq!(latest_order.returned_at, before_failed_return.returned_at);
        assert_eq!(latest_order.items.len(), 1);
        assert_eq!(latest_order.items[0].returned_qty, 0);

        let latest_product = repo
            .find_product_by_id(Some(&pool), tenant_id, product_id, false)
            .await
            .expect("load latest product after failed return")
            .expect("product should still exist after failed return");
        assert_eq!(latest_product.current_stock, before_product.current_stock);
        assert_eq!(latest_product.version, before_product.version);

        let stock_log_count = tenant_stock_log_count(&pool, tenant_id).await;
        assert_eq!(stock_log_count, 1);

        cleanup_inventory_fixture(&pool, tenant_id, operator_id).await;
    }

    #[tokio::test]
    async fn postgres_void_sales_order_stale_expected_version_returns_4091_with_latest_snapshot() {
        let Some(pool) = setup_pg_pool().await else {
            return;
        };

        let repo = PostgresRepository;
        let tenant_id = Uuid::new_v4();
        let operator_id = Uuid::new_v4();
        ensure_operator_user(&pool, tenant_id, operator_id).await;

        let product_id = next_test_product_id();
        let mut product = sample_product(tenant_id, product_id, 1);
        product.current_stock = 10;
        product.cost_price = Decimal::new(200, 2);
        repo.create_product(Some(&pool), &product)
            .await
            .expect("insert product for void-sales-order stale version regression");

        let order_id = next_test_sales_order_id();
        let order = sample_sales_order(tenant_id, order_id, product_id, operator_id, 1, 3);
        repo.create_sales_order(Some(&pool), &order)
            .await
            .expect("insert draft sales-order for void stale version regression");

        let voided = repo
            .void_sales_order(
                Some(&pool),
                tenant_id,
                order_id,
                Some(order.version),
                operator_id,
                "req_repo_pg_so_void_stale_001",
            )
            .await
            .expect("first void sales-order should succeed");
        assert_eq!(voided.status, SalesOrderStatus::Voided);
        assert_eq!(voided.version, order.version + 1);

        let stale_err = repo
            .void_sales_order(
                Some(&pool),
                tenant_id,
                order_id,
                Some(order.version),
                operator_id,
                "req_repo_pg_so_void_stale_002",
            )
            .await
            .expect_err("stale expected_version should return 4091 on void sales-order");

        assert_eq!(stale_err.status, StatusCode::CONFLICT);
        assert_eq!(stale_err.code, 4091);
        assert_eq!(stale_err.data["resource"], json!("sales_order"));
        assert_eq!(stale_err.data["resource_id"], json!(order_id));
        assert_eq!(stale_err.data["expected_version"], json!(order.version));
        assert_eq!(stale_err.data["current_version"], json!(voided.version));
        assert_eq!(
            stale_err.data["latest_snapshot"]["status"],
            json!(voided.status.as_str())
        );
        assert_eq!(
            stale_err.data["latest_snapshot"]["version"],
            json!(voided.version)
        );

        let latest_order = repo
            .find_sales_order_by_id(Some(&pool), tenant_id, order_id)
            .await
            .expect("load latest sales-order after stale void")
            .expect("sales-order should still exist after stale void");
        assert_eq!(latest_order.status, voided.status);
        assert_eq!(latest_order.version, voided.version);
        assert_eq!(latest_order.voided_at, voided.voided_at);

        let stock_log_count = tenant_stock_log_count(&pool, tenant_id).await;
        assert_eq!(stock_log_count, 0);

        cleanup_inventory_fixture(&pool, tenant_id, operator_id).await;
    }

    #[tokio::test]
    async fn postgres_return_sales_order_stale_expected_version_returns_4091_without_mutating_order_product_and_stock_logs()
     {
        let Some(pool) = setup_pg_pool().await else {
            return;
        };

        let repo = PostgresRepository;
        let tenant_id = Uuid::new_v4();
        let operator_id = Uuid::new_v4();
        ensure_operator_user(&pool, tenant_id, operator_id).await;

        let product_id = next_test_product_id();
        let mut product = sample_product(tenant_id, product_id, 1);
        product.current_stock = 10;
        product.cost_price = Decimal::new(200, 2);
        repo.create_product(Some(&pool), &product)
            .await
            .expect("insert product for return-sales-order stale version regression");

        let order_id = next_test_sales_order_id();
        let order = sample_sales_order(tenant_id, order_id, product_id, operator_id, 1, 3);
        repo.create_sales_order(Some(&pool), &order)
            .await
            .expect("insert draft sales-order for return stale version regression");

        let confirmed = repo
            .confirm_sales_order(
                Some(&pool),
                tenant_id,
                order_id,
                Some(order.version),
                false,
                operator_id,
                "req_repo_pg_so_return_stale_001",
            )
            .await
            .expect("confirm sales-order before stale return regression");
        assert_eq!(confirmed.status, SalesOrderStatus::Confirmed);

        let before_failed_return = repo
            .find_sales_order_by_id(Some(&pool), tenant_id, order_id)
            .await
            .expect("load sales-order before stale return")
            .expect("sales-order should exist before stale return");
        let before_product = repo
            .find_product_by_id(Some(&pool), tenant_id, product_id, false)
            .await
            .expect("load product before stale return")
            .expect("product should exist before stale return");

        let stale_err = repo
            .return_sales_order(
                Some(&pool),
                tenant_id,
                order_id,
                Some(order.version),
                &[(product_id, 1)],
                Some("stale return should fail".to_string()),
                operator_id,
                "req_repo_pg_so_return_stale_002",
            )
            .await
            .expect_err("stale expected_version should return 4091 on return sales-order");

        assert_eq!(stale_err.status, StatusCode::CONFLICT);
        assert_eq!(stale_err.code, 4091);
        assert_eq!(stale_err.data["resource"], json!("sales_order"));
        assert_eq!(stale_err.data["resource_id"], json!(order_id));
        assert_eq!(stale_err.data["expected_version"], json!(order.version));
        assert_eq!(stale_err.data["current_version"], json!(confirmed.version));
        assert_eq!(
            stale_err.data["latest_snapshot"]["status"],
            json!(confirmed.status.as_str())
        );
        assert_eq!(
            stale_err.data["latest_snapshot"]["version"],
            json!(confirmed.version)
        );

        let latest_order = repo
            .find_sales_order_by_id(Some(&pool), tenant_id, order_id)
            .await
            .expect("load latest sales-order after stale return")
            .expect("sales-order should still exist after stale return");
        assert_eq!(latest_order.status, before_failed_return.status);
        assert_eq!(latest_order.version, before_failed_return.version);
        assert_eq!(latest_order.returned_at, before_failed_return.returned_at);
        assert_eq!(latest_order.items.len(), before_failed_return.items.len());
        assert_eq!(
            latest_order.items[0].returned_qty,
            before_failed_return.items[0].returned_qty
        );

        let latest_product = repo
            .find_product_by_id(Some(&pool), tenant_id, product_id, false)
            .await
            .expect("load latest product after stale return")
            .expect("product should still exist after stale return");
        assert_eq!(latest_product.current_stock, before_product.current_stock);
        assert_eq!(latest_product.version, before_product.version);

        let stock_log_count = tenant_stock_log_count(&pool, tenant_id).await;
        assert_eq!(stock_log_count, 1);

        cleanup_inventory_fixture(&pool, tenant_id, operator_id).await;
    }
}
