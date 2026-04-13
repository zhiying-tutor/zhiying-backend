mod common;

use axum::http::StatusCode;
use serde_json::json;
use zhiying_backend::auth::encode_token;

use common::TestApp;

#[tokio::test]
async fn auth_and_me_flow_works() {
    let app = TestApp::new().await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/users",
            None,
            Some(json!({
                "username": "alice",
                "password": "password123",
            })),
        )
        .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["username"], "alice");

    let (duplicate_status, duplicate_body) = app
        .request(
            "POST",
            "/api/v1/users",
            None,
            Some(json!({
                "username": "alice",
                "password": "password123",
            })),
        )
        .await;

    assert_eq!(duplicate_status, StatusCode::CONFLICT);
    assert_eq!(duplicate_body["code"], "USERNAME_ALREADY_EXISTS");

    let (login_status, login_body) = app
        .request(
            "POST",
            "/api/v1/tokens",
            None,
            Some(json!({
                "username": "alice",
                "password": "password123",
            })),
        )
        .await;

    assert_eq!(login_status, StatusCode::OK);
    let token = login_body["data"]["token"]
        .as_str()
        .expect("missing token")
        .to_owned();

    let (me_status, me_body) = app.request("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(me_status, StatusCode::OK);
    assert_eq!(me_body["data"]["username"], "alice");

    let (update_status, update_body) = app
        .request(
            "PATCH",
            "/api/v1/me",
            Some(&token),
            Some(json!({
                "birth_year": 2010,
                "introduction": "你好，志英",
            })),
        )
        .await;

    assert_eq!(update_status, StatusCode::OK);
    assert_eq!(update_body["data"]["birth_year"], 2010);
    assert_eq!(update_body["data"]["introduction"], "你好，志英");
}

#[tokio::test]
async fn auth_missing_header_returns_401() {
    let app = TestApp::new().await;
    let (status, body) = app.request("GET", "/api/v1/me", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "MISSING_AUTHORIZATION_HEADER");
}

#[tokio::test]
async fn auth_invalid_format_returns_401() {
    let app = TestApp::new().await;
    let (status, body) = app
        .request_with_raw_auth("GET", "/api/v1/me", "Token abc123", None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "INVALID_AUTHORIZATION_HEADER");
}

#[tokio::test]
async fn auth_expired_token_returns_401() {
    let app = TestApp::new().await;
    // Create a user so user_id=1 exists
    app.create_user_and_login("alice", "password123").await;
    // Generate an expired token (ttl_days = -1)
    let expired = encode_token(1, "alice", "test-secret", -1).expect("encode");
    let (status, body) = app.request("GET", "/api/v1/me", Some(&expired), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "INVALID_OR_EXPIRED_TOKEN");
}

#[tokio::test]
async fn auth_wrong_secret_returns_401() {
    let app = TestApp::new().await;
    app.create_user_and_login("alice", "password123").await;
    let bad = encode_token(1, "alice", "wrong-secret", 30).expect("encode");
    let (status, body) = app.request("GET", "/api/v1/me", Some(&bad), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "INVALID_OR_EXPIRED_TOKEN");
}

#[tokio::test]
async fn auth_wrong_password_returns_401() {
    let app = TestApp::new().await;
    app.create_user_and_login("alice", "password123").await;
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/tokens",
            None,
            Some(json!({"username": "alice", "password": "wrongpassword"})),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "INVALID_CREDENTIALS");
}

#[tokio::test]
async fn auth_nonexistent_user_returns_401() {
    let app = TestApp::new().await;
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/tokens",
            None,
            Some(json!({"username": "nobody", "password": "password123"})),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "INVALID_CREDENTIALS");
}

#[tokio::test]
async fn register_username_too_short_returns_400() {
    let app = TestApp::new().await;
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/users",
            None,
            Some(json!({"username": "ab", "password": "password123"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_FAILED");
}

#[tokio::test]
async fn register_username_too_long_returns_400() {
    let app = TestApp::new().await;
    let long_name = "a".repeat(33);
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/users",
            None,
            Some(json!({"username": long_name, "password": "password123"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_FAILED");
}

#[tokio::test]
async fn register_password_too_short_returns_400() {
    let app = TestApp::new().await;
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/users",
            None,
            Some(json!({"username": "alice", "password": "short"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_FAILED");
}

#[tokio::test]
async fn register_password_too_long_returns_400() {
    let app = TestApp::new().await;
    let long_pw = "x".repeat(73);
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/users",
            None,
            Some(json!({"username": "alice", "password": long_pw})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_FAILED");
}
