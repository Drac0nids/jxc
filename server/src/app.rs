use axum::{
    Router,
    http::{
        HeaderName, Method,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
    middleware,
    routing::get,
};
use tower_http::cors::{Any, CorsLayer};

use crate::{middleware::auth_middleware, routes, state::AppState};

pub fn build_router(state: AppState) -> Router {
    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            AUTHORIZATION,
            CONTENT_TYPE,
            HeaderName::from_static("x-request-id"),
            HeaderName::from_static("x-idempotency-key"),
            HeaderName::from_static("x-client-type"),
        ])
        .expose_headers([
            HeaderName::from_static("x-request-id"),
            HeaderName::from_static("x-idempotent-replay"),
        ]);

    let public_routes = routes::public_routes();
    let protected_routes = routes::protected_routes().layer(middleware::from_fn_with_state(
        state.clone(),
        auth_middleware,
    ));

    Router::new()
        .route("/health", get(routes::health))
        .nest("/api/v1", public_routes.merge(protected_routes))
        .layer(cors_layer)
        .with_state(state)
}
