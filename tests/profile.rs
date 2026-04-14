mod common;

use axum::http::StatusCode;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serde_json::json;
use tower::ServiceExt;
use zhiying_backend::entities::knowledge_explanation;

use common::TestApp;

#[tokio::test]
async fn user_patch_set_public_works() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("gen_user5", "password123").await;

    let db = app.db().await;

    knowledge_explanation::ActiveModel {
        user_id: Set(1),
        status: Set(knowledge_explanation::KnowledgeExplanationStatus::Finished),
        prompt: Set("explain polymorphism".to_owned()),
        content: Set(Some("多态是...".to_owned())),
        mindmap: Set(Some(r#"{"title":"多态"}"#.to_owned())),
        public: Set(false),
        cost: Set(10),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

    // Set public
    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/knowledge-explanations/1",
            Some(&token),
            Some(json!({"public": true})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["public"], true);

    // GET confirms
    let (status, body) = app
        .request(
            "GET",
            "/api/v1/knowledge-explanations/1",
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["public"], true);
    assert_eq!(body["data"]["content"], "多态是...");
    assert_eq!(body["data"]["mindmap"]["title"], "多态");
}

#[tokio::test]
async fn profile_update_birth_year_only() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/me",
            Some(&token),
            Some(json!({"birth_year": 2005})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["birth_year"], 2005);
    assert!(body["data"]["gender"].is_null());
    assert_eq!(body["data"]["introduction"], "");
}

#[tokio::test]
async fn profile_update_gender() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/me",
            Some(&token),
            Some(json!({"gender": "Male"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["gender"], "Male");

    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/me",
            Some(&token),
            Some(json!({"gender": "Female"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["gender"], "Female");
}

#[tokio::test]
async fn profile_introduction_too_long_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let long_intro = "x".repeat(1025);
    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/me",
            Some(&token),
            Some(json!({"introduction": long_intro})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_FAILED");
}

#[tokio::test]
async fn profile_empty_update_succeeds() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let (status, body) = app
        .request("PATCH", "/api/v1/me", Some(&token), Some(json!({})))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["username"], "alice");
}

#[tokio::test]
async fn profile_update_all_fields_at_once() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/me",
            Some(&token),
            Some(json!({
                "birth_year": 2005,
                "gender": "Male",
                "introduction": "你好，世界"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["birth_year"], 2005);
    assert_eq!(body["data"]["gender"], "Male");
    assert_eq!(body["data"]["introduction"], "你好，世界");
}

#[tokio::test]
async fn profile_get_returns_default_values() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (status, body) = app.request("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["username"], "alice");
    assert_eq!(body["data"]["gold"], 0);
    assert_eq!(body["data"]["diamond"], 0);
    assert_eq!(body["data"]["streak_checkin"], 0);
    assert_eq!(body["data"]["total_checkin"], 0);
    assert!(body["data"]["birth_year"].is_null());
    assert!(body["data"]["gender"].is_null());
    assert_eq!(body["data"]["introduction"], "");
}

#[tokio::test]
async fn profile_update_invalid_gender_returns_422() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    // Send raw JSON with invalid gender value — axum's Json<T> deserialization will fail
    let request = axum::http::Request::builder()
        .method("PATCH")
        .uri("/api/v1/me")
        .header(axum::http::header::AUTHORIZATION, format!("Bearer {token}"))
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(
            r#"{"gender": "Invalid"}"#.to_string(),
        ))
        .expect("build request");

    let response = app
        .app
        .clone()
        .oneshot(request)
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
