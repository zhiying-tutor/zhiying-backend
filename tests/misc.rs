mod common;

use axum::http::StatusCode;

use common::TestApp;

#[tokio::test]
async fn health_check_returns_200() {
    let app = TestApp::new().await;
    let (status, body) = app.request("GET", "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "ok");
    assert_eq!(body["data"]["database_url_scheme"], "sqlite");
}

#[tokio::test]
async fn placeholder_my_contents_returns_501() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let (status, body) = app
        .request("GET", "/api/v1/my-contents", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(body["code"], "FEATURE_NOT_IMPLEMENTED");
}

#[tokio::test]
async fn placeholder_public_contents_returns_501() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let (status, body) = app
        .request("GET", "/api/v1/public-contents", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(body["code"], "FEATURE_NOT_IMPLEMENTED");
}
