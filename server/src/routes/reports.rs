use axum::{
    Json,
    extract::{Extension, Query, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use chrono::{FixedOffset, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use std::collections::{HashMap, HashSet};

use crate::routes::common::{
    DashboardOrdersQuery, DashboardQuery, SalesOrderData, SalesReportExportQuery,
    SalesOrderItemData, SalesReportProductData, SalesReportProductRow, SalesReportQuery,
    TrendQuery, build_sales_report_xlsx_content, ensure_role, escape_csv_field,
    is_report_date_in_range, is_same_report_date, load_product_name_map, parse_report_date,
    postgres_pool_or_none, resolve_report_date, to_sales_order_data,
};
use crate::{
    error::AppError,
    middleware::AuthContext,
    models::{Product, SalesOrder, StockLog},
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};

fn resolve_effective_sell_price(
    log: &StockLog,
    sell_price_map: &HashMap<(String, i64), Decimal>,
) -> Option<Decimal> {
    log.snapshot_sell_price
        .as_ref()
        .cloned()
        .or_else(|| sell_price_map.get(&(log.biz_no.clone(), log.product_id)).cloned())
}

pub async fn get_dashboard_report(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(query): Query<DashboardQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    let timezone = FixedOffset::east_opt(8 * 3600)
        .ok_or_else(|| AppError::internal("时区配置异常").with_request_id(request_id.clone()))?;
    let report_date = resolve_report_date(query.date.as_deref(), &timezone, &request_id)?;

    let products: Vec<Product> = if state.repository.is_postgres() {
        state
            .repository
            .list_products_by_tenant_all(&state.persistence, auth.tenant_id)
            .await?
    } else {
        let products = state.products.lock().map_err(|_| {
            AppError::internal("商品状态锁异常").with_request_id(request_id.clone())
        })?;

        products
            .values()
            .filter(|p| p.tenant_id == auth.tenant_id)
            .cloned()
            .collect()
    };

    let mut low_stock_count = 0_u64;
    let mut product_name_map: HashMap<i64, String> = HashMap::new();
    for product in &products {
        product_name_map.insert(product.id, product.name.clone());
        if !product.is_deleted && product.current_stock < product.min_stock_limit {
            low_stock_count += 1;
        }
    }

    let sales_orders: Vec<SalesOrder> = if state.repository.is_postgres() {
        state
            .repository
            .list_sales_orders_by_tenant(&state.persistence, auth.tenant_id)
            .await?
    } else {
        let sales_orders = state.sales_orders.lock().map_err(|_| {
            AppError::internal("销售单状态锁异常").with_request_id(request_id.clone())
        })?;

        sales_orders
            .values()
            .filter(|order| order.tenant_id == auth.tenant_id)
            .cloned()
            .collect()
    };

    let mut sell_price_map: HashMap<(String, i64), Decimal> = HashMap::new();
    for order in &sales_orders {
        for item in &order.items {
            sell_price_map.insert((order.biz_no.clone(), item.product_id), item.sell_price);
        }
    }

    let stock_logs: Vec<StockLog> = if state.repository.is_postgres() {
        state
            .repository
            .list_stock_logs_by_tenant(&state.persistence, auth.tenant_id)
            .await?
    } else {
        let stock_logs = state.stock_logs.lock().map_err(|_| {
            AppError::internal("库存流水锁异常").with_request_id(request_id.clone())
        })?;

        stock_logs
            .iter()
            .filter(|log| log.tenant_id == auth.tenant_id)
            .cloned()
            .collect()
    };

    let mut total_sales = Decimal::ZERO;
    let mut total_cost = Decimal::ZERO;
    let mut net_sold_qty: HashMap<i64, i32> = HashMap::new();
    let mut sold_order_biz_nos: HashSet<String> = HashSet::new();

    for log in &stock_logs {
        if !is_same_report_date(&log.created_at, report_date, &timezone, &request_id)? {
            continue;
        }

        match log.biz_type.as_str() {
            "OUT_SALE" => {
                let qty = (-log.delta_qty).max(0);
                if qty == 0 {
                    continue;
                }

                if let Some(sell_price) = resolve_effective_sell_price(log, &sell_price_map) {
                    sold_order_biz_nos.insert(log.biz_no.clone());
                    *net_sold_qty.entry(log.product_id).or_insert(0) += qty;
                    total_sales += sell_price * Decimal::from(qty);
                    total_cost += log.snapshot_cost * Decimal::from(qty);
                }
            }
            "RETURN_SALE" => {
                let qty = log.delta_qty.max(0);
                if qty == 0 {
                    continue;
                }

                if let Some(sell_price) = resolve_effective_sell_price(log, &sell_price_map) {
                    *net_sold_qty.entry(log.product_id).or_insert(0) -= qty;
                    total_sales -= sell_price * Decimal::from(qty);
                    total_cost -= log.snapshot_cost * Decimal::from(qty);
                }
            }
            _ => {}
        }
    }

    let total_orders = sold_order_biz_nos.len() as u64;

    let top_selling_item = net_sold_qty
        .iter()
        .filter(|(_, qty)| **qty > 0)
        .max_by(|(pid_a, qty_a), (pid_b, qty_b)| qty_a.cmp(qty_b).then_with(|| pid_b.cmp(pid_a)))
        .and_then(|(pid, _)| product_name_map.get(pid).cloned())
        .unwrap_or_else(|| "暂无".to_string());

    let total_gross_profit = (total_sales - total_cost).round_dp(2);

    let body = ApiResponse::success(
        json!({
            "date": report_date.to_string(),
            "total_sales": total_sales.round_dp(2).to_string(),
            "total_gross_profit": total_gross_profit.to_string(),
            "total_orders": total_orders,
            "low_stock_count": low_stock_count,
            "top_selling_item": top_selling_item
        }),
        request_id.clone(),
    );

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn get_dashboard_orders_drilldown(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(query): Query<DashboardOrdersQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "SALES"], &request_id)?;

    let timezone = FixedOffset::east_opt(8 * 3600)
        .ok_or_else(|| AppError::internal("时区配置异常").with_request_id(request_id.clone()))?;
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let today = Utc::now().with_timezone(&timezone).date_naive();
    let (start_date, end_date) = match (
        query
            .start_date
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty()),
        query
            .end_date
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty()),
    ) {
        (None, None) => (today, today),
        (Some(start), Some(end)) => (
            parse_report_date(start, "start_date", &request_id)?,
            parse_report_date(end, "end_date", &request_id)?,
        ),
        _ => {
            return Err(AppError::bad_request("start_date 与 end_date 需同时提供")
                .with_request_id(request_id));
        }
    };

    if start_date > end_date {
        return Err(
            AppError::bad_request("start_date 不能晚于 end_date").with_request_id(request_id)
        );
    }

    let sales_orders: Vec<SalesOrder> = if state.repository.is_postgres() {
        state
            .repository
            .list_sales_orders_by_tenant(&state.persistence, auth.tenant_id)
            .await?
    } else {
        let sales_orders = state.sales_orders.lock().map_err(|_| {
            AppError::internal("销售单状态锁异常").with_request_id(request_id.clone())
        })?;

        sales_orders
            .values()
            .filter(|order| order.tenant_id == auth.tenant_id)
            .cloned()
            .collect()
    };

    let mut sell_price_map: HashMap<(String, i64), Decimal> = HashMap::new();
    for order in &sales_orders {
        for item in &order.items {
            sell_price_map.insert((order.biz_no.clone(), item.product_id), item.sell_price);
        }
    }

    let stock_logs: Vec<StockLog> = if state.repository.is_postgres() {
        state
            .repository
            .list_stock_logs_by_tenant(&state.persistence, auth.tenant_id)
            .await?
    } else {
        let stock_logs = state.stock_logs.lock().map_err(|_| {
            AppError::internal("库存流水锁异常").with_request_id(request_id.clone())
        })?;

        stock_logs
            .iter()
            .filter(|log| log.tenant_id == auth.tenant_id)
            .cloned()
            .collect()
    };

    #[derive(Default)]
    struct DrilldownItemAggregate {
        product_id: i64,
        sell_price: Decimal,
        sold_qty: i32,
        returned_qty: i32,
    }

    #[derive(Default)]
    struct DrilldownOrderAggregate {
        total_amount: Decimal,
        created_at: String,
        updated_at: String,
        item_aggregates: HashMap<(i64, String), DrilldownItemAggregate>,
    }

    let mut sold_order_biz_nos: HashSet<String> = HashSet::new();
    let mut drilldown_aggregates: HashMap<String, DrilldownOrderAggregate> = HashMap::new();

    for log in &stock_logs {
        if !is_report_date_in_range(
            &log.created_at,
            start_date,
            end_date,
            &timezone,
            &request_id,
        )? {
            continue;
        }

        let Some(sell_price) = resolve_effective_sell_price(log, &sell_price_map) else {
            continue;
        };
        let rounded_sell_price = sell_price.round_dp(4);

        let aggregate = drilldown_aggregates
            .entry(log.biz_no.clone())
            .or_insert_with(|| DrilldownOrderAggregate {
                total_amount: Decimal::ZERO,
                created_at: log.created_at.clone(),
                updated_at: log.created_at.clone(),
                item_aggregates: HashMap::new(),
            });

        let item_key = (log.product_id, rounded_sell_price.to_string());
        let item_aggregate = aggregate
            .item_aggregates
            .entry(item_key)
            .or_insert_with(|| DrilldownItemAggregate {
                product_id: log.product_id,
                sell_price: rounded_sell_price,
                sold_qty: 0,
                returned_qty: 0,
            });

        if log.created_at < aggregate.created_at {
            aggregate.created_at = log.created_at.clone();
        }
        if log.created_at > aggregate.updated_at {
            aggregate.updated_at = log.created_at.clone();
        }

        match log.biz_type.as_str() {
            "OUT_SALE" => {
                let qty = (-log.delta_qty).max(0);
                if qty == 0 {
                    continue;
                }

                aggregate.total_amount += sell_price * Decimal::from(qty);
                item_aggregate.sold_qty += qty;
                sold_order_biz_nos.insert(log.biz_no.clone());
            }
            "RETURN_SALE" => {
                let qty = log.delta_qty.max(0);
                if qty == 0 {
                    continue;
                }

                aggregate.total_amount -= sell_price * Decimal::from(qty);
                item_aggregate.returned_qty += qty;
            }
            _ => {
                continue;
            }
        }
    }

    let mut real_orders_by_biz_no: HashMap<String, SalesOrder> = HashMap::new();
    for order in sales_orders {
        if !sold_order_biz_nos.contains(&order.biz_no) {
            continue;
        }

        match real_orders_by_biz_no.get(&order.biz_no) {
            Some(existing)
                if existing.updated_at > order.updated_at
                    || (existing.updated_at == order.updated_at && existing.id >= order.id) => {}
            _ => {
                real_orders_by_biz_no.insert(order.biz_no.clone(), order);
            }
        }
    }

    let mut matched_orders: Vec<SalesOrderData> = Vec::with_capacity(sold_order_biz_nos.len());
    let mut synthetic_order_id_seed: i64 = -1;
    let product_name_map = load_product_name_map(&state, auth.tenant_id, &request_id).await?;

    for biz_no in &sold_order_biz_nos {
        if let Some(order) = real_orders_by_biz_no.get(biz_no) {
            matched_orders.push(to_sales_order_data(order, &product_name_map));
            continue;
        }

        if let Some(aggregate) = drilldown_aggregates.get(biz_no) {
            let mut synthetic_items = aggregate
                .item_aggregates
                .values()
                .filter_map(|item| {
                    let net_qty = item.sold_qty - item.returned_qty;
                    if net_qty <= 0 {
                        return None;
                    }

                    Some(SalesOrderItemData {
                        product_id: item.product_id,
                        product_name: product_name_map
                            .get(&item.product_id)
                            .cloned()
                            .unwrap_or_else(|| format!("商品#{}", item.product_id)),
                        qty: net_qty,
                        sell_price: item.sell_price.round_dp(4).to_string(),
                        line_amount: (item.sell_price * Decimal::from(net_qty))
                            .round_dp(4)
                            .to_string(),
                        returned_qty: item.returned_qty.max(0),
                        sns: Vec::new(),
                    })
                })
                .collect::<Vec<_>>();

            synthetic_items.sort_by(|a, b| {
                a.product_id
                    .cmp(&b.product_id)
                    .then_with(|| a.sell_price.cmp(&b.sell_price))
            });

            matched_orders.push(SalesOrderData {
                id: synthetic_order_id_seed,
                biz_no: biz_no.clone(),
                customer_id: None,
                status: "OUTBOUND_ONLY".to_string(),
                items: synthetic_items,
                total_amount: aggregate.total_amount.round_dp(4).to_string(),
                remark: Some("由库存流水聚合生成（无销售单主记录）".to_string()),
                confirmed_at: Some(aggregate.created_at.clone()),
                returned_at: None,
                voided_at: None,
                created_at: aggregate.created_at.clone(),
                updated_at: aggregate.updated_at.clone(),
            });
            synthetic_order_id_seed -= 1;
        }
    }

    matched_orders.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| b.id.cmp(&a.id))
    });

    let total = sold_order_biz_nos.len() as u64;
    let start = ((page - 1) * page_size) as usize;
    let end = usize::min(start + page_size as usize, matched_orders.len());

    let mut list = if start >= matched_orders.len() {
        Vec::new()
    } else {
        matched_orders[start..end].to_vec()
    };

    // ── Postgres 路径：批量查当前页订单的 SN 码，按 (outbound_biz_no, product_id) group ──
    if let Some(pool) = state.persistence.postgres.as_ref() {
        let biz_nos: Vec<String> = list.iter().map(|o| o.biz_no.clone()).collect();
        if !biz_nos.is_empty() {
            let rows = sqlx::query(
                r#"
                SELECT outbound_biz_no, product_id, sn
                FROM serial_numbers
                WHERE tenant_id = $1
                  AND outbound_biz_no = ANY($2)
                  AND status = 'SOLD'
                ORDER BY outbound_biz_no, product_id, sn
                "#,
            )
            .bind(auth.tenant_id)
            .bind(&biz_nos)
            .fetch_all(pool)
            .await
            .unwrap_or_default();

            let mut sn_map: HashMap<(String, i64), Vec<String>> = HashMap::new();
            for row in &rows {
                use sqlx::Row;
                let biz_no: String = row.try_get("outbound_biz_no").unwrap_or_default();
                let product_id: i64 = row.try_get("product_id").unwrap_or_default();
                let sn: String = row.try_get("sn").unwrap_or_default();
                sn_map.entry((biz_no, product_id)).or_default().push(sn);
            }

            for order in &mut list {
                for item in &mut order.items {
                    let key = (order.biz_no.clone(), item.product_id);
                    if let Some(sns) = sn_map.get(&key) {
                        item.sns = sns.clone();
                    }
                }
            }
        }
    }

    let body = ApiResponse::success(
        json!({
            "start_date": start_date.to_string(),
            "end_date": end_date.to_string(),
            "list": list,
            "total": total,
            "page": page,
            "page_size": page_size
        }),
        request_id.clone(),
    );

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn get_sales_report(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(query): Query<SalesReportQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    let timezone = FixedOffset::east_opt(8 * 3600)
        .ok_or_else(|| AppError::internal("时区配置异常").with_request_id(request_id.clone()))?;

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let group_by = query
        .group_by
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "product".to_string());

    if group_by != "product" {
        return Err(AppError::bad_request("group_by 仅支持 product").with_request_id(request_id));
    }

    let today = Utc::now().with_timezone(&timezone).date_naive();
    let (start_date, end_date) = match (
        query
            .start_date
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty()),
        query
            .end_date
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty()),
    ) {
        (None, None) => (today, today),
        (Some(start), Some(end)) => (
            parse_report_date(start, "start_date", &request_id)?,
            parse_report_date(end, "end_date", &request_id)?,
        ),
        _ => {
            return Err(AppError::bad_request("start_date 与 end_date 需同时提供")
                .with_request_id(request_id));
        }
    };

    if start_date > end_date {
        return Err(
            AppError::bad_request("start_date 不能晚于 end_date").with_request_id(request_id)
        );
    }

    let products: Vec<Product> = if state.repository.is_postgres() {
        state
            .repository
            .list_products_by_tenant_all(&state.persistence, auth.tenant_id)
            .await?
    } else {
        let products = state.products.lock().map_err(|_| {
            AppError::internal("商品状态锁异常").with_request_id(request_id.clone())
        })?;

        products
            .values()
            .filter(|p| p.tenant_id == auth.tenant_id)
            .cloned()
            .collect()
    };

    let mut product_name_map: HashMap<i64, String> = HashMap::new();
    for product in &products {
        product_name_map.insert(product.id, product.name.clone());
    }

    let sales_orders: Vec<SalesOrder> = if state.repository.is_postgres() {
        state
            .repository
            .list_sales_orders_by_tenant(&state.persistence, auth.tenant_id)
            .await?
    } else {
        let sales_orders = state.sales_orders.lock().map_err(|_| {
            AppError::internal("销售单状态锁异常").with_request_id(request_id.clone())
        })?;

        sales_orders
            .values()
            .filter(|order| order.tenant_id == auth.tenant_id)
            .cloned()
            .collect()
    };

    let mut sell_price_map: HashMap<(String, i64), Decimal> = HashMap::new();
    for order in &sales_orders {
        for item in &order.items {
            sell_price_map.insert((order.biz_no.clone(), item.product_id), item.sell_price);
        }
    }

    let stock_logs: Vec<StockLog> = if state.repository.is_postgres() {
        state
            .repository
            .list_stock_logs_by_tenant(&state.persistence, auth.tenant_id)
            .await?
    } else {
        let stock_logs = state.stock_logs.lock().map_err(|_| {
            AppError::internal("库存流水锁异常").with_request_id(request_id.clone())
        })?;

        stock_logs
            .iter()
            .filter(|log| log.tenant_id == auth.tenant_id)
            .cloned()
            .collect()
    };

    let mut grouped: HashMap<i64, SalesReportProductRow> = HashMap::new();
    for log in &stock_logs {
        if !is_report_date_in_range(
            &log.created_at,
            start_date,
            end_date,
            &timezone,
            &request_id,
        )? {
            continue;
        }

        if log.biz_type != "OUT_SALE" && log.biz_type != "RETURN_SALE" {
            continue;
        }

        let row = grouped
            .entry(log.product_id)
            .or_insert_with(|| SalesReportProductRow {
                product_id: log.product_id,
                product_name: product_name_map
                    .get(&log.product_id)
                    .cloned()
                    .unwrap_or_else(|| format!("商品#{}", log.product_id)),
                total_qty: 0,
                total_sales: Decimal::ZERO,
                total_cost: Decimal::ZERO,
            });

        match log.biz_type.as_str() {
            "OUT_SALE" => {
                let qty = (-log.delta_qty).max(0);
                if qty == 0 {
                    continue;
                }

                if let Some(sell_price) = resolve_effective_sell_price(log, &sell_price_map) {
                    row.total_sales += sell_price * Decimal::from(qty);
                    row.total_cost += log.snapshot_cost * Decimal::from(qty);
                    row.total_qty += qty;
                }
            }
            "RETURN_SALE" => {
                let qty = log.delta_qty.max(0);
                if qty == 0 {
                    continue;
                }

                if let Some(sell_price) = resolve_effective_sell_price(log, &sell_price_map) {
                    row.total_sales -= sell_price * Decimal::from(qty);
                    row.total_cost -= log.snapshot_cost * Decimal::from(qty);
                    row.total_qty -= qty;
                }
            }
            _ => {}
        }
    }

    let mut rows = grouped
        .into_values()
        .filter(|row| {
            !(row.total_qty == 0
                && row.total_sales == Decimal::ZERO
                && row.total_cost == Decimal::ZERO)
        })
        .collect::<Vec<_>>();

    rows.sort_by(|a, b| {
        b.total_sales
            .cmp(&a.total_sales)
            .then_with(|| a.product_id.cmp(&b.product_id))
    });

    let summary_total_sales = rows
        .iter()
        .fold(Decimal::ZERO, |acc, row| acc + row.total_sales)
        .round_dp(2);
    let summary_total_cost = rows
        .iter()
        .fold(Decimal::ZERO, |acc, row| acc + row.total_cost)
        .round_dp(2);
    let summary_total_qty = rows
        .iter()
        .fold(0_i64, |acc, row| acc + i64::from(row.total_qty));

    let total = rows.len() as u64;
    let start = ((page - 1) * page_size) as usize;
    let end = usize::min(start + page_size as usize, rows.len());

    let list = if start >= rows.len() {
        Vec::<SalesReportProductData>::new()
    } else {
        rows[start..end]
            .iter()
            .map(|row| {
                let gross_profit = (row.total_sales - row.total_cost).round_dp(2);
                SalesReportProductData {
                    product_id: row.product_id,
                    product_name: row.product_name.clone(),
                    total_qty: row.total_qty,
                    total_sales: row.total_sales.round_dp(2).to_string(),
                    total_cost: row.total_cost.round_dp(2).to_string(),
                    gross_profit: gross_profit.to_string(),
                }
            })
            .collect::<Vec<_>>()
    };

    let body = ApiResponse::success(
        json!({
            "group_by": group_by,
            "start_date": start_date.to_string(),
            "end_date": end_date.to_string(),
            "list": list,
            "total": total,
            "page": page,
            "page_size": page_size,
            "summary": {
                "total_sales": summary_total_sales.to_string(),
                "total_cost": summary_total_cost.to_string(),
                "total_gross_profit": (summary_total_sales - summary_total_cost).round_dp(2).to_string(),
                "total_qty": summary_total_qty
            }
        }),
        request_id.clone(),
    );

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn export_sales_report_csv(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(query): Query<SalesReportExportQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "PURCHASER"], &request_id)?;

    let format = query
        .format
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "csv".to_string());
    if format != "csv" && format != "xlsx" {
        return Err(AppError::bad_request("format 仅支持 csv 或 xlsx").with_request_id(request_id));
    }

    let group_by = query
        .group_by
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "product".to_string());
    if group_by != "product" {
        return Err(AppError::bad_request("group_by 仅支持 product").with_request_id(request_id));
    }

    let timezone = FixedOffset::east_opt(8 * 3600)
        .ok_or_else(|| AppError::internal("时区配置异常").with_request_id(request_id.clone()))?;
    let today = Utc::now().with_timezone(&timezone).date_naive();
    let (start_date, end_date) = match (
        query
            .start_date
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty()),
        query
            .end_date
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty()),
    ) {
        (None, None) => (today, today),
        (Some(start), Some(end)) => (
            parse_report_date(start, "start_date", &request_id)?,
            parse_report_date(end, "end_date", &request_id)?,
        ),
        _ => {
            return Err(AppError::bad_request("start_date 与 end_date 需同时提供")
                .with_request_id(request_id));
        }
    };
    if start_date > end_date {
        return Err(
            AppError::bad_request("start_date 不能晚于 end_date").with_request_id(request_id)
        );
    }

    let products: Vec<Product> = if state.repository.is_postgres() {
        state
            .repository
            .list_products_by_tenant_all(&state.persistence, auth.tenant_id)
            .await?
    } else {
        let products = state.products.lock().map_err(|_| {
            AppError::internal("商品状态锁异常").with_request_id(request_id.clone())
        })?;

        products
            .values()
            .filter(|p| p.tenant_id == auth.tenant_id)
            .cloned()
            .collect()
    };

    let mut product_name_map: HashMap<i64, String> = HashMap::new();
    for product in &products {
        product_name_map.insert(product.id, product.name.clone());
    }

    let sales_orders: Vec<SalesOrder> = if state.repository.is_postgres() {
        state
            .repository
            .list_sales_orders_by_tenant(&state.persistence, auth.tenant_id)
            .await?
    } else {
        let sales_orders = state.sales_orders.lock().map_err(|_| {
            AppError::internal("销售单状态锁异常").with_request_id(request_id.clone())
        })?;

        sales_orders
            .values()
            .filter(|order| order.tenant_id == auth.tenant_id)
            .cloned()
            .collect()
    };

    let mut sell_price_map: HashMap<(String, i64), Decimal> = HashMap::new();
    for order in &sales_orders {
        for item in &order.items {
            sell_price_map.insert((order.biz_no.clone(), item.product_id), item.sell_price);
        }
    }

    let stock_logs: Vec<StockLog> = if state.repository.is_postgres() {
        state
            .repository
            .list_stock_logs_by_tenant(&state.persistence, auth.tenant_id)
            .await?
    } else {
        let stock_logs = state.stock_logs.lock().map_err(|_| {
            AppError::internal("库存流水锁异常").with_request_id(request_id.clone())
        })?;

        stock_logs
            .iter()
            .filter(|log| log.tenant_id == auth.tenant_id)
            .cloned()
            .collect()
    };

    let mut grouped: HashMap<i64, SalesReportProductRow> = HashMap::new();
    for log in &stock_logs {
        if !is_report_date_in_range(
            &log.created_at,
            start_date,
            end_date,
            &timezone,
            &request_id,
        )? {
            continue;
        }

        if log.biz_type != "OUT_SALE" && log.biz_type != "RETURN_SALE" {
            continue;
        }

        let row = grouped
            .entry(log.product_id)
            .or_insert_with(|| SalesReportProductRow {
                product_id: log.product_id,
                product_name: product_name_map
                    .get(&log.product_id)
                    .cloned()
                    .unwrap_or_else(|| format!("商品#{}", log.product_id)),
                total_qty: 0,
                total_sales: Decimal::ZERO,
                total_cost: Decimal::ZERO,
            });

        match log.biz_type.as_str() {
            "OUT_SALE" => {
                let qty = (-log.delta_qty).max(0);
                if qty == 0 {
                    continue;
                }

                if let Some(sell_price) = resolve_effective_sell_price(log, &sell_price_map) {
                    row.total_sales += sell_price * Decimal::from(qty);
                    row.total_cost += log.snapshot_cost * Decimal::from(qty);
                    row.total_qty += qty;
                }
            }
            "RETURN_SALE" => {
                let qty = log.delta_qty.max(0);
                if qty == 0 {
                    continue;
                }

                if let Some(sell_price) = resolve_effective_sell_price(log, &sell_price_map) {
                    row.total_sales -= sell_price * Decimal::from(qty);
                    row.total_cost -= log.snapshot_cost * Decimal::from(qty);
                    row.total_qty -= qty;
                }
            }
            _ => {}
        }
    }

    let mut rows = grouped
        .into_values()
        .filter(|row| {
            !(row.total_qty == 0
                && row.total_sales == Decimal::ZERO
                && row.total_cost == Decimal::ZERO)
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        b.total_sales
            .cmp(&a.total_sales)
            .then_with(|| a.product_id.cmp(&b.product_id))
    });

    let summary_total_sales = rows
        .iter()
        .fold(Decimal::ZERO, |acc, row| acc + row.total_sales)
        .round_dp(2);
    let summary_total_cost = rows
        .iter()
        .fold(Decimal::ZERO, |acc, row| acc + row.total_cost)
        .round_dp(2);
    let summary_total_qty = rows
        .iter()
        .fold(0_i64, |acc, row| acc + i64::from(row.total_qty));
    let summary_total_gross_profit = (summary_total_sales - summary_total_cost).round_dp(2);

    let (content_type, filename, body_bytes) = if format == "xlsx" {
        (
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            format!("sales_report_{}_{}.xlsx", start_date, end_date),
            build_sales_report_xlsx_content(
                &rows,
                summary_total_qty,
                summary_total_sales,
                summary_total_cost,
                summary_total_gross_profit,
                &request_id,
            )?,
        )
    } else {
        let mut csv_lines = Vec::with_capacity(rows.len() + 2);
        csv_lines.push(
            "product_id,product_name,total_qty,total_sales,total_cost,gross_profit".to_string(),
        );
        for row in &rows {
            let gross_profit = (row.total_sales - row.total_cost).round_dp(2);
            csv_lines.push(format!(
                "{},{},{},{},{},{}",
                row.product_id,
                escape_csv_field(&row.product_name),
                row.total_qty,
                row.total_sales.round_dp(2),
                row.total_cost.round_dp(2),
                gross_profit
            ));
        }
        csv_lines.push(format!(
            "SUMMARY,,{},{},{},{}",
            summary_total_qty, summary_total_sales, summary_total_cost, summary_total_gross_profit
        ));

        let csv_content = format!("\u{feff}{}", csv_lines.join("\n"));
        (
            "text/csv; charset=utf-8",
            format!("sales_report_{}_{}.csv", start_date, end_date),
            csv_content.into_bytes(),
        )
    };

    let mut resp_headers = build_response_headers(&request_id, false);
    let content_type_value = HeaderValue::from_str(content_type).map_err(|_| {
        AppError::internal("导出 Content-Type 构造失败").with_request_id(request_id.to_string())
    })?;
    resp_headers.insert(CONTENT_TYPE, content_type_value);

    let content_disposition = format!("attachment; filename=\"{}\"", filename);
    let content_disposition_value = HeaderValue::from_str(&content_disposition).map_err(|_| {
        AppError::internal("导出文件名构造失败").with_request_id(request_id.to_string())
    })?;
    resp_headers.insert(CONTENT_DISPOSITION, content_disposition_value);

    Ok((StatusCode::OK, resp_headers, body_bytes).into_response())
}

pub async fn get_reports_trend(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(query): Query<TrendQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "SALES"], &request_id)?;

    let timezone = FixedOffset::east_opt(8 * 3600)
        .ok_or_else(|| AppError::internal("时区配置异常").with_request_id(request_id.clone()))?;
    let today = Utc::now().with_timezone(&timezone).date_naive();

    let (start_date, end_date) = match (
        query.start_date.as_deref().map(str::trim).filter(|v| !v.is_empty()),
        query.end_date.as_deref().map(str::trim).filter(|v| !v.is_empty()),
    ) {
        (None, None) => {
            let start = today - chrono::Duration::days(6);
            (start, today)
        }
        (Some(start), Some(end)) => (
            parse_report_date(start, "start_date", &request_id)?,
            parse_report_date(end, "end_date", &request_id)?,
        ),
        _ => {
            return Err(AppError::bad_request("start_date 与 end_date 需同时提供")
                .with_request_id(request_id));
        }
    };

    if start_date > end_date {
        return Err(
            AppError::bad_request("start_date 不能晚于 end_date").with_request_id(request_id)
        );
    }

    // Load sell price map from sales orders
    let sales_orders: Vec<crate::models::SalesOrder> = if state.repository.is_postgres() {
        state
            .repository
            .list_sales_orders_by_tenant(&state.persistence, auth.tenant_id)
            .await?
    } else {
        let guard = state.sales_orders.lock().map_err(|_| {
            AppError::internal("销售单状态锁异常").with_request_id(request_id.clone())
        })?;
        guard
            .values()
            .filter(|o| o.tenant_id == auth.tenant_id)
            .cloned()
            .collect()
    };

    let mut sell_price_map: std::collections::HashMap<(String, i64), rust_decimal::Decimal> =
        std::collections::HashMap::new();
    for order in &sales_orders {
        for item in &order.items {
            sell_price_map.insert((order.biz_no.clone(), item.product_id), item.sell_price);
        }
    }

    // Load stock logs
    let stock_logs: Vec<crate::models::StockLog> = if state.repository.is_postgres() {
        state
            .repository
            .list_stock_logs_by_tenant(&state.persistence, auth.tenant_id)
            .await?
    } else {
        let guard = state.stock_logs.lock().map_err(|_| {
            AppError::internal("库存流水锁异常").with_request_id(request_id.clone())
        })?;
        guard
            .iter()
            .filter(|l| l.tenant_id == auth.tenant_id)
            .cloned()
            .collect()
    };

    // Build per-day aggregates
    use chrono::NaiveDate;
    use std::collections::BTreeMap;

    // key: date string "YYYY-MM-DD"
    struct DayAggregate {
        total_sales: rust_decimal::Decimal,
        total_cost: rust_decimal::Decimal,
        order_nos: std::collections::HashSet<String>,
    }

    let mut days: BTreeMap<NaiveDate, DayAggregate> = BTreeMap::new();

    // Ensure every date in range has an entry (even if zero)
    let mut cur = start_date;
    while cur <= end_date {
        days.insert(cur, DayAggregate {
            total_sales: rust_decimal::Decimal::ZERO,
            total_cost: rust_decimal::Decimal::ZERO,
            order_nos: std::collections::HashSet::new(),
        });
        cur += chrono::Duration::days(1);
    }

    for log in &stock_logs {
        // Parse log date
        let log_date = match chrono::DateTime::parse_from_rfc3339(&log.created_at)
            .ok()
            .map(|dt| dt.with_timezone(&timezone).date_naive())
        {
            Some(d) => d,
            None => continue,
        };

        if log_date < start_date || log_date > end_date {
            continue;
        }

        let sell_price = match resolve_effective_sell_price(log, &sell_price_map) {
            Some(p) => p,
            None => continue,
        };

        let entry = days.entry(log_date).or_insert_with(|| DayAggregate {
            total_sales: rust_decimal::Decimal::ZERO,
            total_cost: rust_decimal::Decimal::ZERO,
            order_nos: std::collections::HashSet::new(),
        });

        match log.biz_type.as_str() {
            "OUT_SALE" => {
                let qty = (-log.delta_qty).max(0);
                if qty == 0 { continue; }
                entry.total_sales += sell_price * rust_decimal::Decimal::from(qty);
                entry.total_cost += log.snapshot_cost * rust_decimal::Decimal::from(qty);
                entry.order_nos.insert(log.biz_no.clone());
            }
            "RETURN_SALE" => {
                let qty = log.delta_qty.max(0);
                if qty == 0 { continue; }
                entry.total_sales -= sell_price * rust_decimal::Decimal::from(qty);
                entry.total_cost -= log.snapshot_cost * rust_decimal::Decimal::from(qty);
            }
            _ => {}
        }
    }

    let day_list: Vec<serde_json::Value> = days
        .into_iter()
        .map(|(date, agg)| {
            let gross_profit = (agg.total_sales - agg.total_cost).round_dp(2);
            serde_json::json!({
                "date": date.to_string(),
                "total_sales": agg.total_sales.round_dp(2).to_string(),
                "total_gross_profit": gross_profit.to_string(),
                "total_orders": agg.order_nos.len() as u64,
            })
        })
        .collect();

    let body = crate::response::ApiResponse::success(
        serde_json::json!({
            "start_date": start_date.to_string(),
            "end_date": end_date.to_string(),
            "days": day_list,
        }),
        request_id.clone(),
    );

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

