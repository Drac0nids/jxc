use axum::{
    Json,
    extract::{Extension, Query, State},
    http::{HeaderMap, StatusCode, header::{CONTENT_DISPOSITION, CONTENT_TYPE}, HeaderValue},
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::collections::{HashMap};
use chrono::{FixedOffset, Utc};
use rust_decimal::{Decimal};

use crate::{
    error::AppError,
    middleware::{AuthContext},
    models::{Product, StockLog, SalesOrder},
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};
use crate::routes::common::{
    postgres_pool_or_none, ensure_role, parse_report_date, resolve_report_date,
    is_same_report_date, is_report_date_in_range, escape_csv_field,
    build_sales_report_xlsx_content,
    DashboardQuery, SalesReportQuery, SalesReportExportQuery, SalesReportProductRow, SalesReportProductData
};

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
            .list_products_by_tenant_all(postgres_pool_or_none(&state), auth.tenant_id)
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
            .list_sales_orders_by_tenant(postgres_pool_or_none(&state), auth.tenant_id)
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
    let mut total_orders = 0_u64;
    for order in &sales_orders {
        for item in &order.items {
            sell_price_map.insert((order.biz_no.clone(), item.product_id), item.sell_price);
        }

        if let Some(confirmed_at) = order.confirmed_at.as_deref()
            && is_same_report_date(confirmed_at, report_date, &timezone, &request_id)?
        {
            total_orders += 1;
        }
    }

    let stock_logs: Vec<StockLog> = if state.repository.is_postgres() {
        state
            .repository
            .list_stock_logs_by_tenant(postgres_pool_or_none(&state), auth.tenant_id)
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

                if let Some(sell_price) = sell_price_map
                    .get(&(log.biz_no.clone(), log.product_id))
                    .cloned()
                {
                    total_sales += sell_price * Decimal::from(qty);
                }
                total_cost += log.snapshot_cost * Decimal::from(qty);
                *net_sold_qty.entry(log.product_id).or_insert(0) += qty;
            }
            "RETURN_SALE" => {
                let qty = log.delta_qty.max(0);
                if qty == 0 {
                    continue;
                }

                if let Some(sell_price) = sell_price_map
                    .get(&(log.biz_no.clone(), log.product_id))
                    .cloned()
                {
                    total_sales -= sell_price * Decimal::from(qty);
                }
                total_cost -= log.snapshot_cost * Decimal::from(qty);
                *net_sold_qty.entry(log.product_id).or_insert(0) -= qty;
            }
            _ => {}
        }
    }

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
            .list_products_by_tenant_all(postgres_pool_or_none(&state), auth.tenant_id)
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
            .list_sales_orders_by_tenant(postgres_pool_or_none(&state), auth.tenant_id)
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
            .list_stock_logs_by_tenant(postgres_pool_or_none(&state), auth.tenant_id)
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

                if let Some(sell_price) = sell_price_map
                    .get(&(log.biz_no.clone(), log.product_id))
                    .cloned()
                {
                    row.total_sales += sell_price * Decimal::from(qty);
                }
                row.total_cost += log.snapshot_cost * Decimal::from(qty);
                row.total_qty += qty;
            }
            "RETURN_SALE" => {
                let qty = log.delta_qty.max(0);
                if qty == 0 {
                    continue;
                }

                if let Some(sell_price) = sell_price_map
                    .get(&(log.biz_no.clone(), log.product_id))
                    .cloned()
                {
                    row.total_sales -= sell_price * Decimal::from(qty);
                }
                row.total_cost -= log.snapshot_cost * Decimal::from(qty);
                row.total_qty -= qty;
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
            .list_products_by_tenant_all(postgres_pool_or_none(&state), auth.tenant_id)
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
            .list_sales_orders_by_tenant(postgres_pool_or_none(&state), auth.tenant_id)
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
            .list_stock_logs_by_tenant(postgres_pool_or_none(&state), auth.tenant_id)
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

                if let Some(sell_price) = sell_price_map
                    .get(&(log.biz_no.clone(), log.product_id))
                    .cloned()
                {
                    row.total_sales += sell_price * Decimal::from(qty);
                }
                row.total_cost += log.snapshot_cost * Decimal::from(qty);
                row.total_qty += qty;
            }
            "RETURN_SALE" => {
                let qty = log.delta_qty.max(0);
                if qty == 0 {
                    continue;
                }

                if let Some(sell_price) = sell_price_map
                    .get(&(log.biz_no.clone(), log.product_id))
                    .cloned()
                {
                    row.total_sales -= sell_price * Decimal::from(qty);
                }
                row.total_cost -= log.snapshot_cost * Decimal::from(qty);
                row.total_qty -= qty;
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
