use super::*;

#[tokio::test]
async fn refresh_token_for_nonexistent_user_returns_401() {
    let state = make_test_state();

    let (tenant_id, ghost_user_id) = {
        let users = state.users.lock().expect("lock users");
        let admin = users.get("admin").expect("admin exists");
        (admin.tenant_id, uuid::Uuid::new_v4())
    };

    let ghost_refresh_token = crate::middleware::generate_token(
        &state.config.jwt_secret,
        tenant_id,
        ghost_user_id,
        "OWNER",
        "refresh",
        state.config.refresh_token_exp_secs,
    )
    .expect("generate ghost refresh token");

    let app = build_router(state);
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/refresh")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "refresh_token": ghost_refresh_token }).to_string(),
        ))
        .expect("build refresh request");

    let resp = app.oneshot(req).await.expect("send refresh request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn refresh_token_should_return_current_user_role() {
    let state = make_test_state();

    let (tenant_id, sales_user_id) = {
        let users_arc = state.users.clone();
        let mut users = users_arc.lock().expect("lock users");
        let owner = users.get("admin").expect("admin exists").clone();
        let sales_user_id = uuid::Uuid::new_v4();
        users.insert(
            "sales_refresh".to_string(),
            crate::models::User {
                id: sales_user_id,
                tenant_id: owner.tenant_id,
                username: "sales_refresh".to_string(),
                name: "销售刷新".to_string(),
                role: crate::models::UserRole::Sales,
                password_hash: crate::models::hash_password("sales123"),
            },
        );
        (owner.tenant_id, sales_user_id)
    };

    let forged_refresh_token = crate::middleware::generate_token(
        &state.config.jwt_secret,
        tenant_id,
        sales_user_id,
        "OWNER",
        "refresh",
        state.config.refresh_token_exp_secs,
    )
    .expect("generate forged refresh token");

    let app = build_router(state);

    let refresh_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/refresh")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "refresh_token": forged_refresh_token }).to_string(),
        ))
        .expect("build refresh request");
    let refresh_resp = app
        .clone()
        .oneshot(refresh_req)
        .await
        .expect("send refresh request");
    assert_eq!(refresh_resp.status(), StatusCode::OK);
    let refresh_body = response_json(refresh_resp).await;

    assert_eq!(refresh_body["data"]["user_info"]["role"], "SALES");

    let new_access_token = refresh_body["data"]["access_token"]
        .as_str()
        .expect("new access token")
        .to_string();

    let export_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/reports/sales/export?group_by=product&format=csv")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", new_access_token),
        )
        .body(Body::empty())
        .expect("build export request");
    let export_resp = app.oneshot(export_req).await.expect("send export request");
    assert_eq!(export_resp.status(), StatusCode::FORBIDDEN);
    let export_body = response_json(export_resp).await;
    assert_eq!(export_body["code"], 4030);
}

#[tokio::test]
async fn protected_route_with_nonexistent_user_token_returns_401() {
    let state = make_test_state();

    let tenant_id = {
        let users = state.users.lock().expect("lock users");
        users.get("admin").expect("admin exists").tenant_id
    };

    let forged_token = crate::middleware::generate_token(
        &state.config.jwt_secret,
        tenant_id,
        uuid::Uuid::new_v4(),
        "OWNER",
        "access",
        state.config.access_token_exp_secs,
    )
    .expect("generate token");

    let app = build_router(state);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/products?page=1&page_size=20")
        .header(header::AUTHORIZATION, format!("Bearer {}", forged_token))
        .body(Body::empty())
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn auth_middleware_should_use_current_user_role_instead_of_token_role() {
    let state = make_test_state();

    let (tenant_id, sales_user_id) = {
        let users_arc = state.users.clone();
        let mut users = users_arc.lock().expect("lock users");
        let owner = users.get("admin").expect("admin exists").clone();
        let sales_user_id = uuid::Uuid::new_v4();
        users.insert(
            "sales_shadow".to_string(),
            crate::models::User {
                id: sales_user_id,
                tenant_id: owner.tenant_id,
                username: "sales_shadow".to_string(),
                name: "销售影子".to_string(),
                role: crate::models::UserRole::Sales,
                password_hash: crate::models::hash_password("sales123"),
            },
        );
        (owner.tenant_id, sales_user_id)
    };

    let elevated_token = crate::middleware::generate_token(
        &state.config.jwt_secret,
        tenant_id,
        sales_user_id,
        "OWNER",
        "access",
        state.config.access_token_exp_secs,
    )
    .expect("generate token");

    let app = build_router(state);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/reports/sales/export?group_by=product&format=csv")
        .header(header::AUTHORIZATION, format!("Bearer {}", elevated_token))
        .body(Body::empty())
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4030);
}

#[tokio::test]
async fn user_management_owner_can_list_and_create_users() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    // 1. List users
    let req = Request::builder()
        .uri("/api/v1/users")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 200);
    assert!(body["data"]["list"].as_array().unwrap().len() >= 1);

    // 2. Create user
    let payload = json!({
        "username": "new_user",
        "name": "新员工",
        "role": "SALES",
        "password": "password123"
    })
    .to_string();

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/users")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-user-create-001")
        .body(Body::from(payload))
        .expect("build req");

    let resp = app.oneshot(req).await.expect("send req");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 200);
    assert_eq!(body["data"]["username"], "new_user");
}

#[tokio::test]
async fn user_management_non_owner_cannot_list_users() {
    let state = make_test_state();

    // Setup a SALES user
    let tenant_id = {
        let users = state.users.lock().unwrap();
        users.get("admin").unwrap().tenant_id
    };
    let sales_user = crate::models::User {
        id: uuid::Uuid::new_v4(),
        tenant_id,
        username: "sales_test".to_string(),
        name: "销售员".to_string(),
        role: crate::models::UserRole::Sales,
        password_hash: crate::models::hash_password("sales123"),
    };
    state
        .users
        .lock()
        .unwrap()
        .insert(sales_user.username.clone(), sales_user);

    let app = build_router(state);

    // Login as SALES
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({"username":"sales_test","password":"sales123"}).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = response_json(resp).await;
    let token = body["data"]["access_token"].as_str().unwrap().to_string();

    // Try to list users
    let req = Request::builder()
        .uri("/api/v1/users")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4030);
}

#[tokio::test]
async fn user_management_create_user_with_duplicate_username_returns_4090() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload = json!({
        "username": "admin", // Duplicate
        "name": "另一个管理员",
        "role": "OWNER",
        "password": "password123"
    })
    .to_string();

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/users")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-user-create-dup")
        .body(Body::from(payload))
        .expect("build req");

    let resp = app.oneshot(req).await.expect("send req");
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4090);
}
