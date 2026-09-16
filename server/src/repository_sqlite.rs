use sqlx::{SqlitePool, Sqlite, Row, Transaction, sqlite::SqliteRow};
use uuid::Uuid;
use serde_json::{Value, json};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc, NaiveDate};
use axum::http::StatusCode;
use std::collections::{HashMap, HashSet};
use crate::models::*;
use crate::error::AppError;
use crate::repository::*;

#[derive(Debug, Clone, Copy, Default)]
pub struct SqliteRepository;

impl SqliteRepository {
    pub async fn find_user_by_username(
        &self,
        pool: Option<&SqlitePool>,
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
            id: read_uuid_column(&row, "id", "读取用户ID失败")?,
            tenant_id: read_uuid_column(&row, "tenant_id", "读取租户ID失败")?,
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
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<User>, AppError> {
        let pool = require_pool(pool)?;
        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, username, name, role, password_hash
            FROM users
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(|err| map_sqlx_error("按ID查询用户失败", err))?;

        for row in rows {
            let row_user_id = read_uuid_column(&row, "id", "读取用户ID失败")?;
            let row_tenant_id = read_uuid_column(&row, "tenant_id", "读取租户ID失败")?;
            if row_user_id != user_id || row_tenant_id != tenant_id {
                continue;
            }

            let role_raw: String = row
                .try_get("role")
                .map_err(|err| map_sqlx_error("读取用户角色失败", err))?;

            return Ok(Some(User {
                id: row_user_id,
                tenant_id: row_tenant_id,
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
            }));
        }

        Ok(None)
    }

    pub async fn create_tenant(
        &self,
        pool: Option<&SqlitePool>,
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
        pool: Option<&SqlitePool>,
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
            id: read_uuid_column(&row, "id", "读取用户ID失败")?,
            tenant_id: read_uuid_column(&row, "tenant_id", "读取租户ID失败")?,
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
        pool: Option<&SqlitePool>,
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
                id: read_uuid_column(&row, "id", "读取用户ID失败")?,
                tenant_id: read_uuid_column(&row, "tenant_id", "读取租户ID失败")?,
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

    pub async fn create_user(&self, pool: Option<&SqlitePool>, user: &User) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        let result = sqlx::query(
            r#"
            INSERT INTO users (id, tenant_id, username, name, role, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, datetime('now', 'localtime'), datetime('now', 'localtime'))
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
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        user_id: Uuid,
        new_role: UserRole,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        let _result = sqlx::query(
            r#"
            UPDATE users
            SET role = $1, updated_at = datetime('now', 'localtime')
            WHERE id = $2 AND tenant_id = $3
            "#,
        )
        .bind(new_role.as_str())
        .bind(user_id)
        .bind(tenant_id)
        .execute(pool)
        .await
        .map_err(|err| map_sqlx_error("修改员工角色失败", err))?;

        Ok(())
    }

    pub async fn reset_password(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        user_id: Uuid,
        new_password_hash: &str,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        let _result = sqlx::query(
            r#"
            UPDATE users
            SET password_hash = $1, updated_at = datetime('now', 'localtime')
            WHERE id = $2 AND tenant_id = $3
            "#,
        )
        .bind(new_password_hash)
        .bind(user_id)
        .bind(tenant_id)
        .execute(pool)
        .await
        .map_err(|err| map_sqlx_error("重置员工密码失败", err))?;

        Ok(())
    }

    pub async fn delete_user(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        let result = sqlx::query(
            r#"
            DELETE FROM users
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .execute(pool)
        .await
        .map_err(|err| map_sqlx_error("删除员工失败", err))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("员工不存在"));
        }
        Ok(())
    }

    pub async fn find_product_by_barcode(
        &self,
        pool: Option<&SqlitePool>,
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
                       min_stock_limit, version, is_deleted, category_id, track_batches, track_serials
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
                       min_stock_limit, version, is_deleted, category_id, track_batches, track_serials
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

        row.map(|r| map_product_row(&r)).transpose()
    }

    pub async fn find_barcode_lookup_cache(
        &self,
        pool: Option<&SqlitePool>,
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
            raw_payload: get_json(&row, "raw_payload")?,
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
        pool: Option<&SqlitePool>,
        cache: &BarcodeLookupCache,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        sqlx::query(
            r#"
            INSERT INTO barcode_lookup_cache (
                tenant_id, barcode, lookup_status, product_name, raw_payload, expires_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, datetime('now', 'localtime'))
            ON CONFLICT (tenant_id, barcode)
            DO UPDATE SET lookup_status = EXCLUDED.lookup_status,
                          product_name = EXCLUDED.product_name,
                          raw_payload = EXCLUDED.raw_payload,
                          expires_at = EXCLUDED.expires_at,
                          updated_at = datetime('now', 'localtime')
            "#,
        )
        .bind(cache.tenant_id)
        .bind(&cache.barcode)
        .bind(cache.lookup_status.as_str())
        .bind(&cache.product_name)
        .bind(cache.raw_payload.to_string())
        .bind(cache.expires_at)
        .execute(pool)
        .await
        .map_err(|err| map_sqlx_error("写入条码缓存失败", err))?;

        Ok(())
    }

    pub async fn list_products_by_tenant(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
    ) -> Result<Vec<Product>, AppError> {
        let pool = require_pool(pool)?;
        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, sku, barcode, name, unit,
                   current_stock, cost_price, retail_price, last_inbound_unit_cost,
                   min_stock_limit, version, is_deleted, category_id, track_batches, track_serials
            FROM products
            WHERE tenant_id = $1 AND is_deleted = FALSE
            ORDER BY id ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .map_err(|err| map_sqlx_error("查询商品列表失败", err))?;

        rows.iter().map(map_product_row).collect()
    }

    pub async fn list_products_by_tenant_all(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
    ) -> Result<Vec<Product>, AppError> {
        let pool = require_pool(pool)?;
        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, sku, barcode, name, unit,
                   current_stock, cost_price, retail_price, last_inbound_unit_cost,
                   min_stock_limit, version, is_deleted, category_id, track_batches, track_serials
            FROM products
            WHERE tenant_id = $1
            ORDER BY id ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .map_err(|err| map_sqlx_error("查询商品列表失败", err))?;

        rows.iter().map(map_product_row).collect()
    }

    pub async fn list_sales_orders_by_tenant(
        &self,
        pool: Option<&SqlitePool>,
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
                    sell_price: get_decimal(&item_row, "sell_price")?,
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
                id: order_id, tenant_id: row
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
        pool: Option<&SqlitePool>,
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
                        unit_cost: get_decimal(&item_row, "unit_cost")?,
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
                id: order_id, tenant_id: row
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
        pool: Option<&SqlitePool>,
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
                snapshot_cost: get_decimal(&row, "snapshot_cost")?,
                snapshot_sell_price: get_optional_decimal(&row, "snapshot_sell_price")?,
                snapshot_inbound_unit_cost: get_optional_decimal(&row, "snapshot_inbound_unit_cost")?,
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
        pool: Option<&SqlitePool>,
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
                before_data: get_json(&row, "before_data")?,
                after_data: get_json(&row, "after_data")?,
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
        pool: Option<&SqlitePool>,
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
                       min_stock_limit, version, is_deleted, category_id, track_batches, track_serials
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
                       min_stock_limit, version, is_deleted, category_id, track_batches, track_serials
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

        row.map(|r| map_product_row(&r)).transpose()
    }

    pub async fn is_barcode_taken(
        &self,
        pool: Option<&SqlitePool>,
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
        pool: Option<&SqlitePool>,
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

    pub async fn next_product_id(&self, pool: Option<&SqlitePool>) -> Result<i64, AppError> {
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
        pool: Option<&SqlitePool>,
        product: &Product,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        sqlx::query(
            r#"
            INSERT INTO products (
                id, tenant_id, sku, barcode, name, unit,
                current_stock, cost_price, retail_price, last_inbound_unit_cost,
                min_stock_limit, version, is_deleted, category_id, track_batches, track_serials, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, datetime('now', 'localtime'), datetime('now', 'localtime')
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
        .bind(d_to_f(product.cost_price))
        .bind(d_to_f(product.retail_price))
        .bind(opt_d_to_f(product.last_inbound_unit_cost))
        .bind(product.min_stock_limit)
        .bind(product.is_deleted)
        .bind(product.category_id)
        .bind(product.track_batches)
        .bind(product.track_serials)
        .execute(pool)
        .await
        .map_err(|err| map_sqlx_error("创建商品失败", err))?;

        Ok(())
    }

    pub async fn update_product(
        &self,
        pool: Option<&SqlitePool>,
        product: &Product,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        let result = sqlx::query(
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
                is_deleted = $10,
                category_id = $11,
                track_batches = $12,
                track_serials = $13,
                updated_at = datetime('now', 'localtime')
            WHERE id = $14
              AND tenant_id = $15
            "#,
        )
        .bind(&product.sku)
        .bind(&product.barcode)
        .bind(&product.name)
        .bind(&product.unit)
        .bind(product.current_stock)
        .bind(d_to_f(product.cost_price))
        .bind(d_to_f(product.retail_price))
        .bind(opt_d_to_f(product.last_inbound_unit_cost))
        .bind(product.min_stock_limit)
        .bind(product.is_deleted)
        .bind(product.category_id)
        .bind(product.track_batches)
        .bind(product.track_serials)
        .bind(product.id)
        .bind(product.tenant_id)
        .execute(pool)
        .await
        .map_err(|err| map_sqlx_error("更新商品失败", err))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("商品不存在"));
        }

        Ok(())
    }

    // ── Categories ───────────────────────────────────────────────────────────

    pub async fn list_categories_by_tenant(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
    ) -> Result<Vec<Category>, AppError> {
        let pool = require_pool(pool)?;
        let rows = sqlx::query(
            "SELECT id, tenant_id, parent_id, name, level, sort_order, is_deleted 
             FROM categories WHERE tenant_id = ? AND is_deleted = 0 ORDER BY sort_order"
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .map_err(|e| map_sqlx_error("查询分类列表失败", e))?;

        rows.into_iter().map(|row| {
            Ok(Category {
                id: row.try_get("id")?,
                tenant_id: row.try_get("tenant_id")?,
                parent_id: row.try_get("parent_id")?,
                name: row.try_get("name")?,
                level: row.try_get("level")?,
                sort_order: row.try_get("sort_order")?,
                is_deleted: row.try_get("is_deleted")?,
            })
        }).collect::<Result<Vec<_>, _>>()
          .map_err(|e| map_sqlx_error("解析分类失败", e))
    }

    pub async fn find_category_by_id(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        category_id: i64,
    ) -> Result<Option<Category>, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query(
            "SELECT id, tenant_id, parent_id, name, level, sort_order, is_deleted 
             FROM categories WHERE tenant_id = ? AND id = ?"
        )
        .bind(tenant_id)
        .bind(category_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| map_sqlx_error("查询分类失败", e))?;

        row.map(|row| {
            Ok(Category {
                id: row.try_get("id")?,
                tenant_id: row.try_get("tenant_id")?,
                parent_id: row.try_get("parent_id")?,
                name: row.try_get("name")?,
                level: row.try_get("level")?,
                sort_order: row.try_get("sort_order")?,
                is_deleted: row.try_get("is_deleted")?,
            })
        }).transpose().map_err(|e| map_sqlx_error("解析分类失败", e))
    }

    pub async fn create_category(
        &self,
        pool: Option<&SqlitePool>,
        category: &Category,
    ) -> Result<Category, AppError> {
        let pool = require_pool(pool)?;
        sqlx::query(
            "INSERT INTO categories (tenant_id, parent_id, name, level, sort_order) 
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(category.tenant_id)
        .bind(category.parent_id)
        .bind(&category.name)
        .bind(category.level)
        .bind(category.sort_order)
        .execute(pool)
        .await
        .map_err(|e| map_sqlx_error("创建分类失败", e))?;

        let id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
            .fetch_one(pool)
            .await
            .map_err(|e| map_sqlx_error("获取新分类ID失败", e))?;

        self.find_category_by_id(Some(pool), category.tenant_id, id).await?
            .ok_or_else(|| AppError::internal("新创建的分类未找到"))
    }

    pub async fn update_category(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        category_id: i64,
        name: &str,
        sort_order: i32,
    ) -> Result<Category, AppError> {
        let pool = require_pool(pool)?;
        sqlx::query(
            "UPDATE categories SET name = ?, sort_order = ? WHERE tenant_id = ? AND id = ?"
        )
        .bind(name)
        .bind(sort_order)
        .bind(tenant_id)
        .bind(category_id)
        .execute(pool)
        .await
        .map_err(|e| map_sqlx_error("更新分类失败", e))?;

        self.find_category_by_id(Some(pool), tenant_id, category_id).await?
            .ok_or_else(|| AppError::not_found("分类不存在"))
    }

    pub async fn delete_category(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        category_id: i64,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        sqlx::query(
            "UPDATE categories SET is_deleted = 1 WHERE tenant_id = ? AND id = ?"
        )
        .bind(tenant_id)
        .bind(category_id)
        .execute(pool)
        .await
        .map_err(|e| map_sqlx_error("删除分类失败", e))?;
        Ok(())
    }

    // ── Suppliers ────────────────────────────────────────────────────────────

    pub async fn list_suppliers(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        keyword: Option<String>,
    ) -> Result<Vec<Supplier>, AppError> {
        let pool = require_pool(pool)?;
        let mut query = "SELECT * FROM suppliers WHERE tenant_id = ?".to_string();
        if let Some(ref k) = keyword {
            if !k.is_empty() {
                query.push_str(" AND (name LIKE ? OR phone LIKE ?)");
            }
        }
        query.push_str(" ORDER BY name ASC");

        let mut q = sqlx::query(&query).bind(tenant_id);
        if let Some(ref k) = keyword {
            if !k.is_empty() {
                let pattern = format!("%{}%", k);
                q = q.bind(pattern.clone()).bind(pattern);
            }
        }
        
        let rows = q.fetch_all(pool)
            .await
            .map_err(|e| map_sqlx_error("查询供应商失败", e))?;

        rows.iter().map(|row| {
            Ok(Supplier {
                id: row.try_get("id")?,
                tenant_id: row.try_get("tenant_id")?,
                name: row.try_get("name")?,
                phone: row.try_get("phone")?,
                notes: row.try_get("notes")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        }).collect::<Result<Vec<_>, _>>().map_err(|e| map_sqlx_error("解析供应商数据失败", e))
    }

    pub async fn find_supplier_by_id(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        id: i64,
    ) -> Result<Option<Supplier>, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query(
            "SELECT id, tenant_id, name, phone, notes, created_at, updated_at 
             FROM suppliers WHERE tenant_id = ? AND id = ?"
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| map_sqlx_error("查询供应商失败", e))?;

        row.map(|row| {
            Ok(Supplier {
                id: row.try_get("id")?,
                tenant_id: row.try_get("tenant_id")?,
                name: row.try_get("name")?,
                phone: row.try_get("phone")?,
                notes: row.try_get("notes")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        }).transpose().map_err(|e| map_sqlx_error("解析供应商数据失败", e))
    }

    pub async fn create_supplier(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        name: String,
        phone: Option<String>,
        notes: Option<String>,
    ) -> Result<Supplier, AppError> {
        let pool = require_pool(pool)?;
        sqlx::query(
            "INSERT INTO suppliers (tenant_id, name, phone, notes, created_at, updated_at) 
             VALUES (?, ?, ?, ?, datetime('now', 'localtime'), datetime('now', 'localtime'))"
        )
        .bind(tenant_id)
        .bind(name)
        .bind(phone)
        .bind(notes)
        .execute(pool)
        .await
        .map_err(|e| map_sqlx_error("创建供应商失败", e))?;

        let id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
            .fetch_one(pool)
            .await
            .map_err(|e| map_sqlx_error("获取新供应商ID失败", e))?;

        self.find_supplier_by_id(Some(pool), tenant_id, id).await?
            .ok_or_else(|| AppError::not_found("新创建的供应商不存在"))
    }

    pub async fn update_supplier(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        id: i64,
        name: Option<String>,
        phone: Option<String>,
        notes: Option<String>,
    ) -> Result<Supplier, AppError> {
        let pool = require_pool(pool)?;
        sqlx::query(
            "UPDATE suppliers SET name = COALESCE(?, name), phone = COALESCE(?, phone), notes = COALESCE(?, notes), updated_at = datetime('now', 'localtime') 
             WHERE tenant_id = ? AND id = ?"
        )
        .bind(name)
        .bind(phone)
        .bind(notes)
        .bind(tenant_id)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| map_sqlx_error("更新供应商失败", e))?;

        self.find_supplier_by_id(Some(pool), tenant_id, id).await?
            .ok_or_else(|| AppError::not_found("供应商不存在"))
    }

    pub async fn delete_supplier(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        id: i64,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        sqlx::query("DELETE FROM suppliers WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| map_sqlx_error("删除供应商失败", e))?;
        Ok(())
    }

    // ── Serial Numbers ───────────────────────────────────────────────────────

    pub async fn serial_inbound(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        product_id: i64,
        batch_id: Option<Uuid>,
        unit_cost: Option<Decimal>,
        inbound_biz_no: &str,
        sns: &[String],
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        if sns.is_empty() {
            return Err(AppError::bad_request("序列号列表不能为空"));
        }

        let mut tx = pool.begin().await.map_err(|e| map_sqlx_error("开启事务失败", e))?;

        for sn in sns {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM serial_numbers WHERE tenant_id=? AND sn=? AND status != 'RETURNED')"
            )
            .bind(tenant_id)
            .bind(sn)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| map_sqlx_error("检查序列号重复失败", e))?;

            if exists {
                return Err(AppError::conflict(4093, format!("序列号「{sn}」已在库，不能重复入库")));
            }
        }

        for sn in sns {
            sqlx::query(
                r#"
                INSERT INTO serial_numbers
                    (tenant_id, sn, product_id, batch_id, status, unit_cost, inbound_biz_no, created_at, updated_at)
                VALUES (?, ?, ?, ?, 'IN_STOCK', ?, ?, datetime('now', 'localtime'), datetime('now', 'localtime'))
                ON CONFLICT (tenant_id, sn) DO UPDATE SET
                    status = 'IN_STOCK',
                    product_id = excluded.product_id,
                    batch_id = excluded.batch_id,
                    unit_cost = excluded.unit_cost,
                    inbound_biz_no = excluded.inbound_biz_no,
                    outbound_biz_no = NULL,
                    updated_at = datetime('now', 'localtime')
                "#
            )
            .bind(tenant_id)
            .bind(sn)
            .bind(product_id)
            .bind(batch_id)
            .bind(opt_d_to_f(unit_cost))
            .bind(inbound_biz_no)
            .execute(&mut *tx)
            .await
            .map_err(|e| map_sqlx_error("写入序列号失败", e))?;
        }

        sqlx::query("UPDATE products SET current_stock = current_stock + ?, updated_at = datetime('now', 'localtime') WHERE id = ? AND tenant_id = ?")
            .bind(sns.len() as i32)
            .bind(product_id)
            .bind(tenant_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| map_sqlx_error("更新库存失败", e))?;

        tx.commit().await.map_err(|e| map_sqlx_error("提交事务失败", e))?;
        Ok(())
    }

    pub async fn serial_outbound(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        outbound_biz_no: &str,
        sell_price: Option<Decimal>,
        sns: &[String],
    ) -> Result<Vec<SerialNumber>, AppError> {
        let pool = require_pool(pool)?;
        if sns.is_empty() {
            return Err(AppError::bad_request("序列号列表不能为空"));
        }

        let mut tx = pool.begin().await.map_err(|e| map_sqlx_error("开启事务失败", e))?;
        let mut results = Vec::with_capacity(sns.len());
        let mut product_deltas: std::collections::HashMap<i64, i32> = std::collections::HashMap::new();

        for sn in sns {
            let row = sqlx::query(
                "SELECT id, tenant_id, sn, product_id, batch_id, status, unit_cost, sell_price, inbound_biz_no, outbound_biz_no, created_at, updated_at 
                 FROM serial_numbers WHERE tenant_id=? AND sn=?"
            )
            .bind(tenant_id)
            .bind(sn)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| map_sqlx_error("查询序列号失败", e))?
            .ok_or_else(|| AppError::not_found(format!("序列号「{sn}」不存在")))?;

            let status: String = row.try_get("status")?;
            if status != "IN_STOCK" {
                return Err(AppError::conflict(4094, format!("序列号「{sn}」状态为 {status}，不能出库")));
            }

            let product_id: i64 = row.try_get("product_id")?;
            *product_deltas.entry(product_id).or_insert(0) += 1;

            sqlx::query("UPDATE serial_numbers SET status='SOLD', sell_price=?, outbound_biz_no=?, updated_at=datetime('now', 'localtime') WHERE tenant_id=? AND sn=?")
                .bind(opt_d_to_f(sell_price))
                .bind(outbound_biz_no)
                .bind(tenant_id)
                .bind(sn)
                .execute(&mut *tx)
                .await
                .map_err(|e| map_sqlx_error("更新序列号状态失败", e))?;

            results.push(SerialNumber {
                id: row.try_get("id")?,
                tenant_id,
                sn: sn.clone(),
                product_id,
                batch_id: row.try_get("batch_id")?,
                status: "SOLD".to_string(),
                unit_cost: get_optional_decimal(&row, "unit_cost")?,
                sell_price,
                inbound_biz_no: row.try_get("inbound_biz_no")?,
                outbound_biz_no: Some(outbound_biz_no.to_string()),
                created_at: row.try_get("created_at")?,
                updated_at: Utc::now(),
            });
        }

        for (product_id, delta) in &product_deltas {
            sqlx::query("UPDATE products SET current_stock = current_stock - ?, updated_at = datetime('now', 'localtime') WHERE id = ? AND tenant_id = ?")
                .bind(*delta)
                .bind(product_id)
                .bind(tenant_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| map_sqlx_error("更新商品库存失败", e))?;
        }

        tx.commit().await.map_err(|e| map_sqlx_error("提交事务失败", e))?;
        Ok(results)
    }

    pub async fn list_serials_by_product(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        product_id: i64,
        status_filter: Option<&str>,
    ) -> Result<Vec<SerialNumber>, AppError> {
        let pool = require_pool(pool)?;
        let mut q = "SELECT id, tenant_id, sn, product_id, batch_id, status, unit_cost, sell_price, inbound_biz_no, outbound_biz_no, created_at, updated_at 
                     FROM serial_numbers WHERE tenant_id = ? AND product_id = ?".to_string();
        if status_filter.is_some() {
            q.push_str(" AND status = ?");
        }
        q.push_str(" ORDER BY updated_at DESC");

        let mut query = sqlx::query(&q).bind(tenant_id).bind(product_id);
        if let Some(status) = status_filter {
            query = query.bind(status);
        }

        let rows = query.fetch_all(pool).await.map_err(|e| map_sqlx_error("查询序列号列表失败", e))?;
        rows.into_iter().map(Self::row_to_serial).collect()
    }

    pub async fn list_serials_history(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        status_filter: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<SerialNumber>, i64), AppError> {
        let pool = require_pool(pool)?;
        let offset = (page - 1) * page_size;

        let total: i64 = if let Some(status) = status_filter {
            sqlx::query_scalar("SELECT COUNT(*) FROM serial_numbers WHERE tenant_id = ? AND status = ?")
                .bind(tenant_id).bind(status).fetch_one(pool).await
        } else {
            sqlx::query_scalar("SELECT COUNT(*) FROM serial_numbers WHERE tenant_id = ?")
                .bind(tenant_id).fetch_one(pool).await
        }.map_err(|e| map_sqlx_error("统计序列号失败", e))?;

        let mut q = "SELECT id, tenant_id, sn, product_id, batch_id, status, unit_cost, sell_price, inbound_biz_no, outbound_biz_no, created_at, updated_at 
                     FROM serial_numbers WHERE tenant_id = ?".to_string();
        if status_filter.is_some() {
            q.push_str(" AND status = ?");
        }
        q.push_str(" ORDER BY updated_at DESC LIMIT ? OFFSET ?");

        let mut query = sqlx::query(&q).bind(tenant_id);
        if let Some(status) = status_filter {
            query = query.bind(status);
        }
        query = query.bind(page_size).bind(offset);

        let rows = query.fetch_all(pool).await.map_err(|e| map_sqlx_error("查询序列号历史失败", e))?;
        let list = rows.into_iter().map(Self::row_to_serial).collect::<Result<Vec<_>, _>>()?;
        Ok((list, total))
    }

    pub async fn find_serial_by_sn(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        sn: &str,
    ) -> Result<Option<SerialNumber>, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query(
            "SELECT id, tenant_id, sn, product_id, batch_id, status, unit_cost, sell_price, inbound_biz_no, outbound_biz_no, created_at, updated_at 
             FROM serial_numbers WHERE tenant_id = ? AND sn = ?"
        )
        .bind(tenant_id)
        .bind(sn)
        .fetch_optional(pool)
        .await
        .map_err(|e| map_sqlx_error("查询序列号失败", e))?;

        row.map(Self::row_to_serial).transpose()
    }

    fn row_to_serial(row: sqlx::sqlite::SqliteRow) -> Result<SerialNumber, AppError> {
        Ok(SerialNumber {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            sn: row.try_get("sn")?,
            product_id: row.try_get("product_id")?,
            batch_id: row.try_get("batch_id")?,
            status: row.try_get("status")?,
            unit_cost: get_optional_decimal(&row, "unit_cost")?,
            sell_price: get_optional_decimal(&row, "sell_price")?,
            inbound_biz_no: row.try_get("inbound_biz_no")?,
            outbound_biz_no: row.try_get("outbound_biz_no")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    // ── Product Batches ───────────────────────────────────────────────────────

    pub async fn create_product_batch(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        product_id: i64,
        lot_number: String,
        supplier: Option<String>,
        inbound_at: chrono::NaiveDate,
        produced_at: Option<chrono::NaiveDate>,
        expires_at: Option<chrono::NaiveDate>,
        notes: Option<String>,
    ) -> Result<ProductBatch, AppError> {
        let pool = require_pool(pool)?;
        sqlx::query(
            "INSERT INTO product_batches (tenant_id, product_id, lot_number, supplier, inbound_at, produced_at, expires_at, notes, is_sold_out, created_at, updated_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, datetime('now', 'localtime'), datetime('now', 'localtime'))"
        )
        .bind(tenant_id)
        .bind(product_id)
        .bind(lot_number)
        .bind(supplier)
        .bind(inbound_at)
        .bind(produced_at)
        .bind(expires_at)
        .bind(notes)
        .execute(pool)
        .await
        .map_err(|e| map_sqlx_error("创建批次失败", e))?;

        let id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()").fetch_one(pool).await.map_err(|e| map_sqlx_error("获取新批次ID失败", e))?;
        
        self.find_product_batch_by_id(Some(pool), tenant_id, id).await?
            .ok_or_else(|| AppError::internal("新创建的批次未找到"))
    }

    pub async fn find_product_batch_by_id(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        id: i64,
    ) -> Result<Option<ProductBatch>, AppError> {
        let pool = require_pool(pool)?;
        let row = sqlx::query(
            "SELECT id, tenant_id, product_id, lot_number, supplier, inbound_at, produced_at, expires_at, notes, is_sold_out, sold_out_at, created_at, updated_at 
             FROM product_batches WHERE tenant_id = ? AND id = ?"
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| map_sqlx_error("查询批次失败", e))?;

        row.map(|r| Self::row_to_batch(&r)).transpose()
    }

    pub async fn list_product_batches(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        product_id: Option<i64>,
        only_active: bool,
    ) -> Result<Vec<ProductBatch>, AppError> {
        let pool = require_pool(pool)?;
        let mut query = "SELECT * FROM product_batches WHERE tenant_id = ?".to_string();
        if let Some(_) = product_id {
            query.push_str(" AND product_id = ?");
        }
        if only_active {
            query.push_str(" AND (is_sold_out = 0 OR is_sold_out IS NULL)");
        }
        query.push_str(" ORDER BY expires_at ASC");

        let mut q = sqlx::query(&query).bind(tenant_id);
        if let Some(pid) = product_id {
            q = q.bind(pid);
        }
        let rows = q.fetch_all(pool).await.map_err(|e| map_sqlx_error("查询商品批次列表失败", e))?;

        rows.iter().map(Self::row_to_batch).collect()
    }

    pub async fn list_expiring_batches(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        days: i32,
    ) -> Result<Vec<crate::repository::ProductBatchWithProduct>, AppError> {
        let pool = require_pool(pool)?;
        let target_date = Utc::now().date_naive() + chrono::Duration::days(days as i64);
        
        let rows = sqlx::query(
            r#"SELECT b.*, p.sku as p_sku, p.barcode as p_barcode, p.name as p_name, p.unit as p_unit, 
                      p.current_stock as p_current_stock, p.cost_price as p_cost_price, p.retail_price as p_retail_price,
                      p.last_inbound_unit_cost as p_last_inbound_unit_cost, p.min_stock_limit as p_min_stock_limit,
                      p.is_deleted as p_is_deleted, p.category_id as p_category_id, p.track_batches as p_track_batches, p.track_serials as p_track_serials
               FROM product_batches b
               JOIN products p ON b.product_id = p.id AND b.tenant_id = p.tenant_id
               WHERE b.tenant_id = ? AND b.is_sold_out = 0 AND b.expires_at <= ?
               ORDER BY b.expires_at ASC"#
        )
        .bind(tenant_id)
        .bind(target_date)
        .fetch_all(pool)
        .await
        .map_err(|e| map_sqlx_error("查询即将过期批次失败", e))?;

        rows.iter().map(|row| {
            let batch = Self::row_to_batch(&row)?;
            Ok(crate::repository::ProductBatchWithProduct {
                batch,
                product_name: row.try_get("p_name")?,
                product_sku: row.try_get("p_sku")?,
            })
        }).collect()
    }

    pub async fn mark_batch_sold_out(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        batch_id: i64,
    ) -> Result<ProductBatch, AppError> {
        let pool = require_pool(pool)?;
        sqlx::query("UPDATE product_batches SET is_sold_out = 1, sold_out_at = datetime('now', 'localtime'), updated_at = datetime('now', 'localtime') WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(batch_id)
            .execute(pool)
            .await
            .map_err(|e| map_sqlx_error("更新批次售罄状态失败", e))?;
        
        self.find_product_batch_by_id(Some(pool), tenant_id, batch_id).await?
            .ok_or_else(|| AppError::not_found("批次不存在"))
    }

    pub async fn update_product_batch(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        batch_id: i64,
        lot_number: Option<String>,
        supplier: Option<String>,
        produced_at: Option<chrono::NaiveDate>,
        expires_at: Option<chrono::NaiveDate>,
        notes: Option<String>,
    ) -> Result<ProductBatch, AppError> {
        let pool = require_pool(pool)?;
        sqlx::query(
            "UPDATE product_batches SET lot_number = COALESCE(?, lot_number), supplier = COALESCE(?, supplier), produced_at = COALESCE(?, produced_at), expires_at = COALESCE(?, expires_at), notes = COALESCE(?, notes), updated_at = datetime('now', 'localtime') 
             WHERE tenant_id = ? AND id = ?"
        )
        .bind(lot_number)
        .bind(supplier)
        .bind(produced_at)
        .bind(expires_at)
        .bind(notes)
        .bind(tenant_id)
        .bind(batch_id)
        .execute(pool)
        .await
        .map_err(|e| map_sqlx_error("更新批次失败", e))?;

        self.find_product_batch_by_id(Some(pool), tenant_id, batch_id).await?
            .ok_or_else(|| AppError::not_found("批次不存在"))
    }

    pub async fn delete_product_batch(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        id: i64,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        sqlx::query("DELETE FROM product_batches WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| map_sqlx_error("删除批次失败", e))?;
        Ok(())
    }

    pub async fn count_batches_by_date(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        product_id: i64,
        date: chrono::NaiveDate,
    ) -> Result<i64, AppError> {
        let pool = require_pool(pool)?;
        sqlx::query_scalar("SELECT COUNT(*) FROM product_batches WHERE tenant_id = ? AND product_id = ? AND inbound_at = ?")
            .bind(tenant_id)
            .bind(product_id)
            .bind(date)
            .fetch_one(pool)
            .await
            .map_err(|e| map_sqlx_error("统计批次失败", e))
    }

    fn row_to_batch(row: &sqlx::sqlite::SqliteRow) -> Result<ProductBatch, AppError> {
        Ok(ProductBatch {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            product_id: row.try_get("product_id")?,
            lot_number: row.try_get("lot_number")?,
            supplier: row.try_get("supplier")?,
            inbound_at: row.try_get("inbound_at")?,
            produced_at: row.try_get("produced_at")?,
            expires_at: row.try_get("expires_at")?,
            notes: row.try_get("notes")?,
            is_sold_out: row.try_get("is_sold_out")?,
            sold_out_at: row.try_get("sold_out_at")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn inbound(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        product_id: i64,
        qty: i32,
        unit_cost: Option<Decimal>,

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
                   min_stock_limit, version, is_deleted, category_id, track_batches, track_serials
            FROM products
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(product_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("锁定商品失败", err))?
        .ok_or_else(|| AppError::not_found("商品不存在"))?;

        let mut existing = map_product_row(&row)?;
        if existing.is_deleted {
            return Err(AppError::not_found("商品不存在"));
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

        sqlx::query(
            r#"
            UPDATE products
            SET current_stock = $1,
                cost_price = $2,
                last_inbound_unit_cost = $3,
                updated_at = datetime('now', 'localtime')
            WHERE tenant_id = $4 AND id = $5
            "#,
        )
        .bind(existing.current_stock)
        .bind(d_to_f(existing.cost_price))
        .bind(opt_d_to_f(existing.last_inbound_unit_cost))
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, datetime('now', 'localtime'))
            "#,
        )
        .bind(stock_log_id)
        .bind(tenant_id)
        .bind(existing.id)
        .bind("IN_PURCHASE")
        .bind(biz_no)
        .bind(qty)
        .bind(existing.current_stock)
        .bind(d_to_f(existing.cost_price))
        .bind(Option::<f64>::None)
        .bind(Some(d_to_f(effective_unit_cost)))
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
        pool: Option<&SqlitePool>,
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

        for &(product_id, qty, unit_cost, _) in items {
            if qty <= 0 {
                return Err(AppError::bad_request("items.qty 必须大于 0"));
            }

            let row = sqlx::query(
                r#"
                SELECT id, tenant_id, sku, barcode, name, unit,
                       current_stock, cost_price, retail_price, last_inbound_unit_cost,
                       min_stock_limit, version, is_deleted, category_id, track_batches, track_serials
                FROM products
                WHERE tenant_id = $1 AND id = $2
                    "#,
            )
            .bind(tenant_id)
            .bind(product_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("锁定商品失败", err))?
            .ok_or_else(|| AppError::not_found("商品不存在"))?;

            let mut existing = map_product_row(&row)?;
            if existing.is_deleted {
                return Err(AppError::not_found("商品不存在"));
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

            sqlx::query(
                r#"
                UPDATE products
                SET current_stock = $1,
                    cost_price = $2,
                    last_inbound_unit_cost = $3,
                    updated_at = datetime('now', 'localtime')
                WHERE tenant_id = $4 AND id = $5
                "#,
            )
            .bind(existing.current_stock)
            .bind(d_to_f(existing.cost_price))
            .bind(opt_d_to_f(existing.last_inbound_unit_cost))
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
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, datetime('now', 'localtime'))
                "#,
            )
            .bind(next_stock_log_id)
            .bind(tenant_id)
            .bind(existing.id)
            .bind("IN_PURCHASE")
            .bind(biz_no)
            .bind(qty)
            .bind(existing.current_stock)
            .bind(d_to_f(existing.cost_price))
            .bind(Option::<f64>::None)
            .bind(Some(d_to_f(effective_unit_cost)))
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
        pool: Option<&SqlitePool>,
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
                       min_stock_limit, version, is_deleted, category_id, track_batches, track_serials
                FROM products
                WHERE tenant_id = $1 AND id = $2
                    "#,
            )
            .bind(tenant_id)
            .bind(product_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| map_sqlx_error("锁定商品失败", err))?
            .ok_or_else(|| AppError::not_found("商品不存在"))?;

            let mut existing = map_product_row(&row)?;
            if existing.is_deleted {
                return Err(AppError::not_found("商品不存在"));
            }

            let _expected_version = item_expected_version.or(default_expected_version);
            

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

            sqlx::query(
                r#"
                UPDATE products
                SET current_stock = $1,
                    updated_at = datetime('now', 'localtime')
                WHERE tenant_id = $2 AND id = $3
                "#,
            )
            .bind(existing.current_stock)
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
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, datetime('now', 'localtime'))
                "#,
            )
            .bind(next_stock_log_id)
            .bind(tenant_id)
            .bind(existing.id)
            .bind("OUT_SALE")
            .bind(biz_no)
            .bind(-qty)
            .bind(existing.current_stock)
            .bind(d_to_f(existing.cost_price))
            .bind(opt_d_to_f(snapshot_sell_price))
            .bind(Option::<f64>::None)
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
        pool: Option<&SqlitePool>,
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
            request_payload: get_json(&row, "request_payload")?,
            response_body: get_json(&row, "response_body")?,
        }))
    }

    pub async fn save_idempotency_record(
        &self,
        pool: Option<&SqlitePool>,
        scope_key: &str,
        request_payload: &Value,
        response_body: &Value,
    ) -> Result<(), AppError> {
        let pool = require_pool(pool)?;
        sqlx::query(
            r#"
            INSERT INTO idempotency_records(scope_key, request_payload, response_body, created_at, updated_at)
            VALUES ($1, $2, $3, datetime('now', 'localtime'), datetime('now', 'localtime'))
            ON CONFLICT (scope_key)
            DO UPDATE SET request_payload = EXCLUDED.request_payload,
                          response_body = EXCLUDED.response_body,
                          updated_at = datetime('now', 'localtime')
            "#,
        )
        .bind(scope_key)
        .bind(request_payload.to_string())
        .bind(response_body.to_string())
        .execute(pool)
        .await
        .map_err(|err| map_sqlx_error("写入幂等记录失败", err))?;

        Ok(())
    }

    pub async fn next_purchase_order_id(&self, pool: Option<&SqlitePool>) -> Result<i64, AppError> {
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
        pool: Option<&SqlitePool>,
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
        pool: Option<&SqlitePool>,
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
                VALUES ($1, $2, $3, $4, $5, $6, datetime('now', 'localtime'))
                "#,
            )
            .bind(order.tenant_id)
            .bind(order.id)
            .bind(item.product_id)
            .bind(item.qty)
            .bind(d_to_f(item.unit_cost))
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
        pool: Option<&SqlitePool>,
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
                unit_cost: get_decimal(&item_row, "unit_cost")?,
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
            confirmed_at,
            voided_at,
            created_at,
            updated_at,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn confirm_purchase_order(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        order_id: i64,
        _expected_version: Option<i32>,
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
            let old_cost = get_decimal(&product_row, "cost_price")?;

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
                    updated_at = datetime('now', 'localtime')
                WHERE tenant_id = $4 AND id = $5
                "#,
            )
            .bind(new_stock)
            .bind(d_to_f(new_cost))
            .bind(Some(d_to_f(item.unit_cost.round_dp(4))))
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
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, datetime('now', 'localtime'))
                "#,
            )
            .bind(next_stock_log_id)
            .bind(tenant_id)
            .bind(product_id)
            .bind("IN_PURCHASE")
            .bind(&existing.biz_no)
            .bind(item.qty)
            .bind(new_stock)
            .bind(d_to_f(new_cost))
            .bind(Option::<f64>::None)
            .bind(Some(d_to_f(item.unit_cost)))
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
        updated.confirmed_at = Some(now_rfc3339.clone());
        updated.updated_at = now_rfc3339.clone();

        sqlx::query(
            r#"
            UPDATE purchase_orders
            SET status = 'CONFIRMED',
                confirmed_at = $2,
                updated_at = $2
            WHERE tenant_id = $3 AND id = $4
            "#,
        )
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, datetime('now', 'localtime'))
            "#,
        )
        .bind(audit_log_id)
        .bind(tenant_id)
        .bind(operator_id)
        .bind("PURCHASE_ORDER_CONFIRM")
        .bind("purchase_order")
        .bind(order_id.to_string())
        .bind(purchase_order_snapshot(&existing).to_string())
        .bind(purchase_order_snapshot(&updated).to_string())
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
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        order_id: i64,
        _expected_version: Option<i32>,
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

        

        if existing.status != SalesOrderStatus::Draft {
            return Err(AppError::conflict(4090, "仅 DRAFT 状态可作废销售单")
                .with_data(json!({ "status": existing.status.as_str() })));
        }

        let now = Utc::now();
        let now_rfc3339 = now.to_rfc3339();
        let mut updated = existing.clone();
        updated.status = SalesOrderStatus::Voided;
        updated.voided_at = Some(now_rfc3339.clone());
        updated.updated_at = now_rfc3339;

        sqlx::query(
            r#"
            UPDATE sales_orders
            SET status = 'VOIDED',
                voided_at = $2,
                updated_at = $2
            WHERE tenant_id = $3 AND id = $4
            "#,
        )
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, datetime('now', 'localtime'))
            "#,
        )
        .bind(audit_log_id)
        .bind(tenant_id)
        .bind(operator_id)
        .bind("SALES_ORDER_VOID")
        .bind("sales_order")
        .bind(order_id.to_string())
        .bind(sales_order_snapshot(&existing).to_string())
        .bind(sales_order_snapshot(&updated).to_string())
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
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        order_id: i64,
        _expected_version: Option<i32>,
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
            let snapshot_cost = get_decimal(&product_row, "cost_price")?;

            let new_stock = current_stock + qty;

            sqlx::query(
                r#"
                UPDATE products
                SET current_stock = $1,
                    updated_at = datetime('now', 'localtime')
                WHERE tenant_id = $3 AND id = $4
                "#,
            )
            .bind(new_stock)
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
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, datetime('now', 'localtime'))
                "#,
            )
            .bind(next_stock_log_id)
            .bind(tenant_id)
            .bind(product_id)
            .bind("RETURN_SALE")
            .bind(&existing.biz_no)
            .bind(qty)
            .bind(new_stock)
            .bind(d_to_f(snapshot_cost))
            .bind(
                existing
                    .items
                    .iter()
                    .find(|item| item.product_id == product_id)
                    .map(|item| d_to_f(item.sell_price)),
            )
            .bind(Option::<f64>::None)
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
                remark = $2,
                returned_at = $3,
                updated_at = $3
            WHERE tenant_id = $4 AND id = $5
            "#,
        )
        .bind(updated.status.as_str())
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, datetime('now', 'localtime'))
            "#,
        )
        .bind(audit_log_id)
        .bind(tenant_id)
        .bind(operator_id)
        .bind("SALES_ORDER_RETURN")
        .bind("sales_order")
        .bind(order_id.to_string())
        .bind(sales_order_snapshot(&existing).to_string())
        .bind(sales_order_snapshot(&updated).to_string())
        .bind(request_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("写入审计日志失败", err))?;

        tx.commit()
            .await
            .map_err(|err| map_sqlx_error("销售退货事务提交失败", err))?;

        Ok(updated)
    }

    pub async fn next_stock_check_id(&self, pool: Option<&SqlitePool>) -> Result<i64, AppError> {
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
        pool: Option<&SqlitePool>,
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
        pool: Option<&SqlitePool>,
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
                VALUES ($1, $2, $3, $4, $5, $6, datetime('now', 'localtime'))
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
        pool: Option<&SqlitePool>,
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
            counting_at,
            confirmed_at,
            created_at,
            updated_at,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn start_stock_check(
        &self,
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        check_id: i64,
        _expected_version: Option<i32>,
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

        

        if existing.status != StockCheckStatus::Draft {
            return Err(AppError::conflict(4090, "仅 DRAFT 状态可开始盘点")
                .with_data(json!({ "status": existing.status.as_str() })));
        }

        let now = Utc::now();
        let now_rfc3339 = now.to_rfc3339();
        let mut updated = existing.clone();
        updated.status = StockCheckStatus::Counting;
        updated.counting_at = Some(now_rfc3339.clone());
        updated.updated_at = now_rfc3339;

        sqlx::query(
            r#"
            UPDATE stock_checks
            SET status = 'COUNTING',
                counting_at = $2,
                updated_at = $2
            WHERE tenant_id = $3 AND id = $4
            "#,
        )
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, datetime('now', 'localtime'))
            "#,
        )
        .bind(audit_log_id)
        .bind(tenant_id)
        .bind(operator_id)
        .bind("STOCK_CHECK_START")
        .bind("stock_check")
        .bind(check_id.to_string())
        .bind(stock_check_snapshot(&existing).to_string())
        .bind(stock_check_snapshot(&updated).to_string())
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
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        check_id: i64,
        _expected_version: Option<i32>,
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
            let snapshot_cost = get_decimal(&product_row, "cost_price")?;

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
                    updated_at = datetime('now', 'localtime')
                WHERE tenant_id = $3 AND id = $4
                "#,
            )
            .bind(actual_stock)
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
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, datetime('now', 'localtime'))
                    "#,
                )
                .bind(next_stock_log_id)
                .bind(tenant_id)
                .bind(product_id)
                .bind("ADJ_CHECK")
                .bind(&existing.biz_no)
                .bind(delta_qty)
                .bind(actual_stock)
                .bind(d_to_f(snapshot_cost))
                .bind(Option::<f64>::None)
                .bind(Option::<f64>::None)
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
        updated.confirmed_at = Some(now_rfc3339.clone());
        updated.updated_at = now_rfc3339.clone();
        if let Some(remark) = remark {
            updated.remark = Some(remark);
        }

        sqlx::query(
            r#"
            UPDATE stock_checks
            SET status = 'CONFIRMED',
                remark = $2,
                confirmed_at = $3,
                updated_at = $3
            WHERE tenant_id = $4 AND id = $5
            "#,
        )
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, datetime('now', 'localtime'))
            "#,
        )
        .bind(audit_log_id)
        .bind(tenant_id)
        .bind(operator_id)
        .bind("STOCK_CHECK_CONFIRM")
        .bind("stock_check")
        .bind(check_id.to_string())
        .bind(stock_check_snapshot(&existing).to_string())
        .bind(stock_check_snapshot(&updated).to_string())
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
        tx: &mut Transaction<'_, Sqlite>,
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
            counting_at,
            confirmed_at,
            created_at,
            updated_at,
        }))
    }

    async fn find_sales_order_by_id_for_update(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
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
                sell_price: get_decimal(&item_row, "sell_price")?,
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
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        order_id: i64,
        _expected_version: Option<i32>,
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
                let snapshot_cost = get_decimal(&product_row, "cost_price")?;

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
                        updated_at = datetime('now', 'localtime')
                    WHERE tenant_id = $3 AND id = $4
                    "#,
                )
                .bind(new_stock)
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
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, datetime('now', 'localtime'))
                    "#,
                )
                .bind(next_stock_log_id)
                .bind(tenant_id)
                .bind(product_id)
                .bind("VOID_PURCHASE")
                .bind(&existing.biz_no)
                .bind(-item.qty)
                .bind(new_stock)
                .bind(d_to_f(snapshot_cost))
                .bind(Option::<f64>::None)
                .bind(Option::<f64>::None)
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
        updated.voided_at = Some(now_rfc3339.clone());
        updated.updated_at = now_rfc3339.clone();

        sqlx::query(
            r#"
            UPDATE purchase_orders
            SET status = 'VOIDED',
                voided_at = $2,
                updated_at = $2
            WHERE tenant_id = $3 AND id = $4
            "#,
        )
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, datetime('now', 'localtime'))
            "#,
        )
        .bind(audit_log_id)
        .bind(tenant_id)
        .bind(operator_id)
        .bind("PURCHASE_ORDER_VOID")
        .bind("purchase_order")
        .bind(order_id.to_string())
        .bind(purchase_order_snapshot(&existing).to_string())
        .bind(purchase_order_snapshot(&updated).to_string())
        .bind(request_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_sqlx_error("写入审计日志失败", err))?;

        tx.commit()
            .await
            .map_err(|err| map_sqlx_error("作废采购单事务提交失败", err))?;

        Ok(updated)
    }

    pub async fn next_sales_order_id(&self, pool: Option<&SqlitePool>) -> Result<i64, AppError> {
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
        pool: Option<&SqlitePool>,
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
        pool: Option<&SqlitePool>,
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
                VALUES ($1, $2, $3, $4, $5, $6, $7, datetime('now', 'localtime'))
                "#,
            )
            .bind(order.tenant_id)
            .bind(order.id)
            .bind(item.product_id)
            .bind(item.qty)
            .bind(d_to_f(item.sell_price))
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
        pool: Option<&SqlitePool>,
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
                sell_price: get_decimal(&item_row, "sell_price")?,
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
        pool: Option<&SqlitePool>,
        tenant_id: Uuid,
        order_id: i64,
        _expected_version: Option<i32>,
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
            let snapshot_cost = get_decimal(&product_row, "cost_price")?;

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
                    updated_at = datetime('now', 'localtime')
                WHERE tenant_id = $3 AND id = $4
                "#,
            )
            .bind(new_stock)
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
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, datetime('now', 'localtime'))
                "#,
            )
            .bind(next_stock_log_id)
            .bind(tenant_id)
            .bind(product_id)
            .bind("OUT_SALE")
            .bind(&existing.biz_no)
            .bind(-item.qty)
            .bind(new_stock)
            .bind(d_to_f(snapshot_cost))
            .bind(Some(d_to_f(item.sell_price)))
                .bind(Option::<f64>::None)
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
        updated.confirmed_at = Some(now_rfc3339.clone());
        updated.updated_at = now_rfc3339.clone();

        sqlx::query(
            r#"
            UPDATE sales_orders
            SET status = 'CONFIRMED',
                confirmed_at = $2,
                updated_at = $2
            WHERE tenant_id = $3 AND id = $4
            "#,
        )
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, datetime('now', 'localtime'))
            "#,
        )
        .bind(audit_log_id)
        .bind(tenant_id)
        .bind(operator_id)
        .bind("SALES_ORDER_CONFIRM")
        .bind("sales_order")
        .bind(order_id.to_string())
        .bind(sales_order_snapshot(&existing).to_string())
        .bind(sales_order_snapshot(&updated).to_string())
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
        tx: &mut Transaction<'_, Sqlite>,
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
                unit_cost: get_decimal(&item_row, "unit_cost")?,
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
            confirmed_at,
            voided_at,
            created_at,
            updated_at,
        }))
    }

    async fn next_stock_log_id_in_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
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
        tx: &mut Transaction<'_, Sqlite>,
    ) -> Result<i64, AppError> {
        let row = sqlx::query("SELECT COALESCE(MAX(id), 0) + 1 AS next_id FROM audit_logs")
            .fetch_one(&mut **tx)
            .await
            .map_err(|err| map_sqlx_error("生成审计日志ID失败", err))?;

        row.try_get("next_id")
            .map_err(|err| map_sqlx_error("读取审计日志ID失败", err))
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn require_pool(pool: Option<&SqlitePool>) -> Result<&SqlitePool, AppError> {
    pool.ok_or_else(|| AppError::internal("SQLite 连接池未初始化"))
}

fn map_sqlx_error(context: &str, err: sqlx::Error) -> AppError {
    AppError::internal(format!("{context}: {err}"))
}

fn parse_user_role(role: &str) -> Result<UserRole, AppError> {
    match role {
        "OWNER" => Ok(UserRole::Owner),
        "ADMIN" => Ok(UserRole::Admin),
        "PURCHASER" => Ok(UserRole::Purchaser),
        "SALES" => Ok(UserRole::Sales),
        _ => Err(AppError::internal(format!("未知用户角色: {role}"))),
    }
}

fn read_uuid_column(row: &SqliteRow, column: &str, context: &str) -> Result<Uuid, AppError> {
    if let Ok(value) = row.try_get::<Uuid, _>(column) {
        return Ok(value);
    }

    if let Ok(text) = row.try_get::<String, _>(column) {
        return Uuid::parse_str(text.trim())
            .map_err(|err| AppError::internal(format!("{context}: {err}")));
    }

    if let Ok(bytes) = row.try_get::<Vec<u8>, _>(column) {
        return Uuid::from_slice(&bytes)
            .map_err(|err| AppError::internal(format!("{context}: {err}")));
    }

    Err(AppError::internal(format!("{context}: 列 {column} 不是合法 UUID")))
}

fn parse_barcode_lookup_status(status: &str) -> Result<BarcodeLookupStatus, AppError> {
    match status {
        "FOUND" => Ok(BarcodeLookupStatus::Found),
        "NOT_FOUND" => Ok(BarcodeLookupStatus::NotFound),
        _ => Err(AppError::internal(format!("未知条码查询状态: {status}"))),
    }
}

fn parse_sales_order_status(status: &str) -> Result<SalesOrderStatus, AppError> {
    match status {
        "DRAFT" => Ok(SalesOrderStatus::Draft),
        "CONFIRMED" => Ok(SalesOrderStatus::Confirmed),
        "RETURNED_PARTIAL" => Ok(SalesOrderStatus::ReturnedPartial),
        "RETURNED_FULL" => Ok(SalesOrderStatus::ReturnedFull),
        "VOIDED" => Ok(SalesOrderStatus::Voided),
        _ => Err(AppError::internal(format!("未知销售单状态: {status}"))),
    }
}

fn parse_purchase_order_status(status: &str) -> Result<PurchaseOrderStatus, AppError> {
    match status {
        "DRAFT" => Ok(PurchaseOrderStatus::Draft),
        "CONFIRMED" => Ok(PurchaseOrderStatus::Confirmed),
        "VOIDED" => Ok(PurchaseOrderStatus::Voided),
        _ => Err(AppError::internal(format!("未知采购单状态: {status}"))),
    }
}

fn parse_stock_check_status(status: &str) -> Result<StockCheckStatus, AppError> {
    match status {
        "DRAFT" => Ok(StockCheckStatus::Draft),
        "COUNTING" => Ok(StockCheckStatus::Counting),
        "CONFIRMED" => Ok(StockCheckStatus::Confirmed),
        _ => Err(AppError::internal(format!("未知盘点单状态: {status}"))),
    }
}

fn map_product_row(row: &sqlx::sqlite::SqliteRow) -> Result<Product, AppError> {
    let category_id: Option<i64> = row.try_get("category_id").unwrap_or(None);
    Ok(Product {
        id: row.try_get("id").map_err(|e| map_sqlx_error("读取产品ID", e))?,
        tenant_id: row.try_get("tenant_id").map_err(|e| map_sqlx_error("读取租户ID", e))?,
        sku: row.try_get("sku").map_err(|e| map_sqlx_error("读取SKU", e))?,
        barcode: row.try_get("barcode").map_err(|e| map_sqlx_error("读取条码", e))?,
        name: row.try_get("name").map_err(|e| map_sqlx_error("读取名称", e))?,
        unit: row.try_get("unit").map_err(|e| map_sqlx_error("读取单位", e))?,
        current_stock: row.try_get("current_stock").map_err(|e| map_sqlx_error("读取库存", e))?,
        cost_price: get_decimal(&row, "cost_price")?,
        retail_price: get_decimal(&row, "retail_price")?,
        last_inbound_unit_cost: get_optional_decimal(&row, "last_inbound_unit_cost")?,
        min_stock_limit: row.try_get("min_stock_limit").map_err(|e| map_sqlx_error("读取最小库存", e))?,
        is_deleted: row.try_get("is_deleted").map_err(|e| map_sqlx_error("读取删除标记", e))?,
        category_id,
        track_batches: row.try_get("track_batches").unwrap_or(false),
        track_serials: row.try_get("track_serials").unwrap_or(false),
    })
}

fn get_decimal(row: &sqlx::sqlite::SqliteRow, column: &str) -> Result<Decimal, AppError> {
    use rust_decimal::prelude::FromPrimitive;
    let val: f64 = row.try_get(column).map_err(|e| map_sqlx_error(&format!("读取列 {column} 失败"), e))?;
    Decimal::from_f64(val).ok_or_else(|| AppError::internal(format!("解析列 {column} 为 Decimal 失败")))
}

fn get_optional_decimal(row: &sqlx::sqlite::SqliteRow, column: &str) -> Result<Option<Decimal>, AppError> {
    use rust_decimal::prelude::FromPrimitive;
    let val_opt: Option<f64> = row.try_get(column).map_err(|e| map_sqlx_error(&format!("读取列 {column} 失败"), e))?;
    match val_opt {
        Some(val) => {
            let d = Decimal::from_f64(val).ok_or_else(|| AppError::internal(format!("解析列 {column} 为 Decimal 失败")))?;
            Ok(Some(d))
        }
        None => Ok(None),
    }
}

fn sales_order_snapshot(order: &SalesOrder) -> Value {
    serde_json::to_value(order).unwrap_or(Value::Null)
}

fn purchase_order_snapshot(order: &PurchaseOrder) -> Value {
    serde_json::to_value(order).unwrap_or(Value::Null)
}

fn parse_datetime(raw: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| AppError::internal(format!("解析时间失败: {}", e)))
}

fn parse_optional_datetime(raw: Option<&str>) -> Result<Option<DateTime<Utc>>, AppError> {
    match raw {
        Some(s) => Ok(Some(parse_datetime(s)?)),
        None => Ok(None),
    }
}


fn stock_check_snapshot(check: &StockCheck) -> Value {
    json!({
        "id": check.id,
        "biz_no": check.biz_no,
        "status": check.status.as_str(),
        "updated_at": check.updated_at
    })
}

fn d_to_f(d: Decimal) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    d.to_f64().unwrap_or(0.0)
}

fn opt_d_to_f(d: Option<Decimal>) -> Option<f64> {
    use rust_decimal::prelude::ToPrimitive;
    d.and_then(|v| v.to_f64())
}

fn get_json(row: &sqlx::sqlite::SqliteRow, column: &str) -> Result<Value, AppError> {
    let s: String = row.try_get(column).map_err(|e| map_sqlx_error(&format!("读取JSON列 {column} 失败"), e))?;
    Ok(serde_json::from_str(&s).unwrap_or(Value::Null))
}

fn is_sql_state(err: &sqlx::Error, _state: &str) -> bool {
    err.to_string().contains("UNIQUE constraint failed")
}
