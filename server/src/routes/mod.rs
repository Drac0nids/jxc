pub mod common;
pub mod auth;
pub mod users;
pub mod products;
pub mod inventory;
pub mod purchase_orders;
pub mod sales_orders;
pub mod stock_checks;
pub mod reports;
pub mod audit;

use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, patch},
};
use serde_json::json;

use crate::{
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh_token))
}

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/logout", post(auth::logout))
        .route("/users", get(users::list_users).post(users::create_user))
        .route("/users/:id/role", patch(users::update_user_role))
        .route("/users/:id/reset-password", post(users::reset_user_password))
        .route("/products/scan", get(products::scan_product))
        .route("/inventory/logs", get(audit::list_stock_logs))
        .route("/inventory/alerts/low-stock", get(inventory::list_low_stock_alerts))
        .route("/reports/dashboard", get(reports::get_dashboard_report))
        .route("/reports/sales", get(reports::get_sales_report))
        .route("/reports/sales/export", get(reports::export_sales_report_csv))
        .route("/audit/logs", get(audit::list_audit_logs))
        .route("/products", get(products::list_products).post(products::create_product))
        .route(
            "/products/:id",
            get(products::get_product).put(products::update_product).delete(products::delete_product),
        )
        .route("/purchase-orders", post(purchase_orders::create_purchase_order))
        .route("/purchase-orders/:id", get(purchase_orders::get_purchase_order))
        .route("/purchase-orders/:id/confirm", post(purchase_orders::confirm_purchase_order))
        .route("/purchase-orders/:id/void", post(purchase_orders::void_purchase_order))
        .route("/sales-orders", post(sales_orders::create_sales_order))
        .route("/sales-orders/:id", get(sales_orders::get_sales_order))
        .route("/sales-orders/:id/confirm", post(sales_orders::confirm_sales_order))
        .route("/sales-orders/:id/void", post(sales_orders::void_sales_order))
        .route("/sales-orders/:id/return", post(sales_orders::return_sales_order))
        .route("/inventory/stock-checks", post(stock_checks::create_stock_check))
        .route("/inventory/stock-checks/:id", get(stock_checks::get_stock_check))
        .route("/inventory/stock-checks/:id/start", post(stock_checks::start_stock_check))
        .route(
            "/inventory/stock-checks/:id/confirm",
            post(stock_checks::confirm_stock_check),
        )
        .route("/inventory/inbound", post(inventory::inbound))
        .route("/inventory/outbound", post(inventory::outbound))
}

pub async fn health(headers: HeaderMap) -> Response {
    let request_id = resolve_request_id(&headers);
    let body = ApiResponse::success(
        json!({
            "status": "ok",
            "service": "jxc-server"
        }),
        request_id.clone(),
    );

    (
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response()
}

#[cfg(test)]
mod tests;
