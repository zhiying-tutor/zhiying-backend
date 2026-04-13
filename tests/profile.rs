mod common;

use axum::http::StatusCode;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serde_json::json;
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
