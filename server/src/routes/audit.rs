use axum::{
    Json,
    extract::{Extension, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::FixedOffset;
use serde_json::json;
use std::cmp::Reverse;
use uuid::Uuid;

use crate::routes::common::{
    AuditLogData, AuditLogQuery, StockLogData, StockLogQuery, ensure_role, is_report_date_in_range,
    load_product_name_map, parse_report_date, postgres_pool_or_none,
};
use crate::{
    error::AppError,
    middleware::AuthContext,
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};

pub async fn list_audit_logs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(query): Query<AuditLogQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN"], &request_id)?;

    let timezone = FixedOffset::east_opt(8 * 3600)
        .ok_or_else(|| AppError::internal("时区配置异常").with_request_id(request_id.clone()))?;
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let action_filter = query
        .action
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned);
    let target_type_filter = query
        .target_type
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned);
    let request_id_filter = query
        .request_id
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned);
    let operator_id_filter = query
        .operator_id
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|raw| {
            Uuid::parse_str(raw).map_err(|_| {
                AppError::bad_request("operator_id 格式错误，必须为 UUID")
                    .with_request_id(request_id.clone())
            })
        })
        .transpose()?;

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
        (None, None) => (None, None),
        (Some(start), Some(end)) => (
            Some(parse_report_date(start, "start_date", &request_id)?),
            Some(parse_report_date(end, "end_date", &request_id)?),
        ),
        _ => {
            return Err(AppError::bad_request("start_date 与 end_date 需同时提供")
                .with_request_id(request_id));
        }
    };

    if let (Some(start), Some(end)) = (start_date, end_date)
        && start > end
    {
        return Err(
            AppError::bad_request("start_date 不能晚于 end_date").with_request_id(request_id)
        );
    }

    let audit_logs = if state.repository.is_postgres() {
        state
            .repository
            .list_audit_logs_by_tenant(postgres_pool_or_none(&state), auth.tenant_id)
            .await?
    } else {
        let audit_logs = state.audit_logs.lock().map_err(|_| {
            AppError::internal("审计日志锁异常").with_request_id(request_id.clone())
        })?;

        audit_logs
            .iter()
            .filter(|log| log.tenant_id == auth.tenant_id)
            .cloned()
            .collect::<Vec<_>>()
    };

    let mut filtered = audit_logs
        .iter()
        .filter(|log| log.tenant_id == auth.tenant_id)
        .filter(|log| {
            if let Some(ref action) = action_filter {
                return log.action.eq_ignore_ascii_case(action);
            }
            true
        })
        .filter(|log| {
            if let Some(ref target_type) = target_type_filter {
                return log.target_type.eq_ignore_ascii_case(target_type);
            }
            true
        })
        .filter(|log| {
            if let Some(operator_id) = operator_id_filter {
                return log.operator_id == operator_id;
            }
            true
        })
        .filter(|log| {
            if let Some(ref rid) = request_id_filter {
                return log.request_id == *rid;
            }
            true
        })
        .filter_map(|log| {
            let in_range = match (start_date, end_date) {
                (Some(start), Some(end)) => {
                    is_report_date_in_range(&log.created_at, start, end, &timezone, &request_id)
                        .unwrap_or(false)
                }
                _ => true,
            };

            if !in_range {
                return None;
            }

            Some(AuditLogData {
                id: log.id,
                operator_id: log.operator_id.to_string(),
                action: log.action.clone(),
                target_type: log.target_type.clone(),
                target_id: log.target_id.clone(),
                before_data: log.before_data.clone(),
                after_data: log.after_data.clone(),
                request_id: log.request_id.clone(),
                created_at: log.created_at.clone(),
            })
        })
        .collect::<Vec<_>>();

    filtered.sort_by_key(|entry| Reverse(entry.id));

    let total = filtered.len() as u64;
    let start = ((page - 1) * page_size) as usize;
    let end = usize::min(start + page_size as usize, filtered.len());
    let list = if start >= filtered.len() {
        Vec::new()
    } else {
        filtered[start..end].to_vec()
    };

    let body = ApiResponse::success(
        json!({
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

pub async fn list_stock_logs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(query): Query<StockLogQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER"], &request_id)?;

    let timezone = FixedOffset::east_opt(8 * 3600)
        .ok_or_else(|| AppError::internal("时区配置异常").with_request_id(request_id.clone()))?;

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let biz_type_filter = query
        .biz_type
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned);
    let biz_no_filter = query
        .biz_no
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_ascii_lowercase());
    let product_id_filter = query.product_id;
    if let Some(product_id) = product_id_filter
        && product_id <= 0
    {
        return Err(AppError::bad_request("product_id 必须为正整数").with_request_id(request_id));
    }
    let operator_id_filter = query
        .operator_id
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|raw| {
            Uuid::parse_str(raw).map_err(|_| {
                AppError::bad_request("operator_id 格式错误，必须为 UUID")
                    .with_request_id(request_id.clone())
            })
        })
        .transpose()?;

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
        (None, None) => (None, None),
        (Some(start), Some(end)) => (
            Some(parse_report_date(start, "start_date", &request_id)?),
            Some(parse_report_date(end, "end_date", &request_id)?),
        ),
        _ => {
            return Err(AppError::bad_request("start_date 与 end_date 需同时提供")
                .with_request_id(request_id));
        }
    };

    if let (Some(start), Some(end)) = (start_date, end_date)
        && start > end
    {
        return Err(
            AppError::bad_request("start_date 不能晚于 end_date").with_request_id(request_id)
        );
    }

    let stock_logs = if state.repository.is_postgres() {
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
            .collect::<Vec<_>>()
    };

    let mut filtered = stock_logs
        .iter()
        .filter(|log| log.tenant_id == auth.tenant_id)
        .filter(|log| {
            if let Some(ref biz_type) = biz_type_filter {
                return log.biz_type.eq_ignore_ascii_case(biz_type);
            }
            true
        })
        .filter(|log| {
            if let Some(ref biz_no) = biz_no_filter {
                return log.biz_no.to_ascii_lowercase().contains(biz_no);
            }
            true
        })
        .filter(|log| {
            if let Some(product_id) = product_id_filter {
                return log.product_id == product_id;
            }
            true
        })
        .filter(|log| {
            if let Some(operator_id) = operator_id_filter {
                return log.operator_id == operator_id;
            }
            true
        })
        .filter_map(|log| {
            let in_range = match (start_date, end_date) {
                (Some(start), Some(end)) => {
                    is_report_date_in_range(&log.created_at, start, end, &timezone, &request_id)
                        .unwrap_or(false)
                }
                _ => true,
            };

            if !in_range {
                return None;
            }

            Some(StockLogData {
                id: log.id,
                product_id: log.product_id,
                product_name: None,
                biz_type: log.biz_type.clone(),
                biz_no: log.biz_no.clone(),
                delta_qty: log.delta_qty,
                snapshot_stock: log.snapshot_stock,
                snapshot_cost: log.snapshot_cost.round_dp(4).to_string(),
                snapshot_sell_price: log.snapshot_sell_price.map(|v| v.round_dp(4).to_string()),
                snapshot_inbound_unit_cost: log
                    .snapshot_inbound_unit_cost
                    .map(|v| v.round_dp(4).to_string()),
                operator_id: log.operator_id.to_string(),
                operator_name: None,
                created_at: log.created_at.clone(),
            })
        })
        .collect::<Vec<_>>();

    filtered.sort_by_key(|entry| Reverse(entry.id));

    // 商品名称映射
    let product_name_map = load_product_name_map(&state, auth.tenant_id, &request_id)
        .await
        .unwrap_or_default();

    // 操作人姓名映射
    let users = state
        .repository
        .list_users_by_tenant(postgres_pool_or_none(&state), auth.tenant_id)
        .await
        .unwrap_or_default();
    let user_name_map: std::collections::HashMap<String, String> = users
        .into_iter()
        .map(|u| (u.id.to_string(), u.name.clone()))
        .collect();

    let total = filtered.len() as u64;
    let start = ((page - 1) * page_size) as usize;
    let end = usize::min(start + page_size as usize, filtered.len());
    let list: Vec<StockLogData> = if start >= filtered.len() {
        Vec::new()
    } else {
        filtered[start..end]
            .iter()
            .map(|log| StockLogData {
                product_name: product_name_map.get(&log.product_id).cloned(),
                operator_name: user_name_map.get(&log.operator_id).cloned(),
                ..log.clone()
            })
            .collect()
    };

    let body = ApiResponse::success(
        json!({
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
