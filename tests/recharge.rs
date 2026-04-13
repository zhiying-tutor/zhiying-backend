mod common;

use axum::http::StatusCode;
use serde_json::json;

use common::TestApp;

#[tokio::test]
async fn recharge_add_gold_and_diamond() {
    let app = TestApp::new().await;
    app.create_user_and_login("recharge_user1", "password123")
        .await;
    let api_key = &app.config.recharge_api_key;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/internal/users/1/balance",
            Some(api_key),
            Some(json!({"gold": 100, "diamond": 50})),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["gold"], 100);
    assert_eq!(body["data"]["diamond"], 50);
}

#[tokio::test]
async fn recharge_add_gold_only() {
    let app = TestApp::new().await;
    app.create_user_and_login("recharge_user2", "password123")
        .await;
    let api_key = &app.config.recharge_api_key;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/internal/users/1/balance",
            Some(api_key),
            Some(json!({"gold": 200})),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["gold"], 200);
    assert_eq!(body["data"]["diamond"], 0);
}

#[tokio::test]
async fn recharge_deduct_gold() {
    let app = TestApp::new().await;
    app.create_user_and_login("recharge_user3", "password123")
        .await;
    let api_key = &app.config.recharge_api_key;

    // Give user some gold first
    app.update_user_state("recharge_user3", None, 0, 0, 500, 10)
        .await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/internal/users/1/balance",
            Some(api_key),
            Some(json!({"gold": -200})),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["gold"], 300);
    assert_eq!(body["data"]["diamond"], 10);
}

#[tokio::test]
async fn recharge_insufficient_gold_returns_400() {
    let app = TestApp::new().await;
    app.create_user_and_login("recharge_user4", "password123")
        .await;
    let api_key = &app.config.recharge_api_key;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/internal/users/1/balance",
            Some(api_key),
            Some(json!({"gold": -1})),
        )
        .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INSUFFICIENT_GOLD");
}

#[tokio::test]
async fn recharge_insufficient_diamond_returns_400() {
    let app = TestApp::new().await;
    app.create_user_and_login("recharge_user5", "password123")
        .await;
    let api_key = &app.config.recharge_api_key;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/internal/users/1/balance",
            Some(api_key),
            Some(json!({"diamond": -1})),
        )
        .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INSUFFICIENT_DIAMONDS");
}

#[tokio::test]
async fn recharge_user_not_found_returns_404() {
    let app = TestApp::new().await;
    let api_key = &app.config.recharge_api_key;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/internal/users/999/balance",
            Some(api_key),
            Some(json!({"gold": 100})),
        )
        .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "USER_NOT_FOUND");
}

#[tokio::test]
async fn recharge_empty_body_returns_400() {
    let app = TestApp::new().await;
    app.create_user_and_login("recharge_user6", "password123")
        .await;
    let api_key = &app.config.recharge_api_key;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/internal/users/1/balance",
            Some(api_key),
            Some(json!({})),
        )
        .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_FAILED");
}

#[tokio::test]
async fn recharge_wrong_api_key_returns_401() {
    let app = TestApp::new().await;
    app.create_user_and_login("recharge_user7", "password123")
        .await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/internal/users/1/balance",
            Some("sk-wrong-key"),
            Some(json!({"gold": 100})),
        )
        .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "INVALID_API_KEY");
}

#[tokio::test]
async fn recharge_other_service_key_returns_401() {
    let app = TestApp::new().await;
    app.create_user_and_login("recharge_user8", "password123")
        .await;
    // Use a valid API key but for a different service
    let wrong_service_key = &app.config.knowledge_video_api_key;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/internal/users/1/balance",
            Some(wrong_service_key),
            Some(json!({"gold": 100})),
        )
        .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "INVALID_API_KEY");
}

#[tokio::test]
async fn recharge_accumulates_correctly() {
    let app = TestApp::new().await;
    app.create_user_and_login("recharge_user9", "password123")
        .await;
    let api_key = &app.config.recharge_api_key;

    // First recharge
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/internal/users/1/balance",
            Some(api_key),
            Some(json!({"gold": 100, "diamond": 20})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["gold"], 100);
    assert_eq!(body["data"]["diamond"], 20);

    // Second recharge
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/internal/users/1/balance",
            Some(api_key),
            Some(json!({"gold": 50, "diamond": -10})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["gold"], 150);
    assert_eq!(body["data"]["diamond"], 10);
}
