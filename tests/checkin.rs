mod common;

use axum::http::StatusCode;
use chrono::{Days, Utc};
use serde_json::json;

use common::TestApp;

#[tokio::test]
async fn checkin_basic_flow_and_repeat_guard_work() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("bob", "password123").await;

    let (checkin_status, checkin_body) = app
        .request("POST", "/api/v1/checkins", Some(&token), Some(json!({})))
        .await;

    assert_eq!(checkin_status, StatusCode::CREATED);
    assert_eq!(checkin_body["data"]["gold_reward"], 1);
    assert_eq!(checkin_body["data"]["makeup_applied"], false);
    assert_eq!(checkin_body["data"]["total_checkin"], 1);
    assert_eq!(checkin_body["data"]["streak_checkin"], 1);

    let (list_status, list_body) = app
        .request("GET", "/api/v1/checkins?limit=10", Some(&token), None)
        .await;

    assert_eq!(list_status, StatusCode::OK);
    assert_eq!(list_body["data"].as_array().map(Vec::len), Some(1));
    assert_eq!(list_body["data"][0]["gold_reward"], 1);

    let (repeat_status, repeat_body) = app
        .request("POST", "/api/v1/checkins", Some(&token), Some(json!({})))
        .await;

    assert_eq!(repeat_status, StatusCode::BAD_REQUEST);
    assert_eq!(repeat_body["code"], "ALREADY_CHECKED_IN_TODAY");
}

#[tokio::test]
async fn checkin_without_makeup_resets_streak_after_gap() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("carol", "password123").await;
    let today = Utc::now().date_naive();

    app.update_user_state("carol", today.checked_sub_days(Days::new(2)), 3, 3, 0, 0)
        .await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/checkins",
            Some(&token),
            Some(json!({ "makeup": false })),
        )
        .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["makeup_applied"], false);
    assert_eq!(body["data"]["gold_reward"], 1);
    assert_eq!(body["data"]["streak_checkin"], 1);
    assert_eq!(body["data"]["total_checkin"], 4);
}

#[tokio::test]
async fn checkin_makeup_updates_rewards_and_costs() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("dave", "password123").await;
    let today = Utc::now().date_naive();

    app.update_user_state("dave", today.checked_sub_days(Days::new(3)), 2, 2, 100, 5)
        .await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/checkins",
            Some(&token),
            Some(json!({ "makeup": true })),
        )
        .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["makeup_applied"], true);
    assert_eq!(body["data"]["makeup_days"], 2);
    assert_eq!(body["data"]["gold_cost"], 20);
    assert_eq!(body["data"]["diamond_cost"], 1);
    assert_eq!(body["data"]["gold_reward"], 12);
    assert_eq!(body["data"]["streak_checkin"], 5);
    assert_eq!(body["data"]["total_checkin"], 5);

    let (me_status, me_body) = app.request("GET", "/api/v1/me", Some(&token), None).await;

    assert_eq!(me_status, StatusCode::OK);
    assert_eq!(me_body["data"]["gold"], 92);
    assert_eq!(me_body["data"]["diamond"], 4);
    assert_eq!(me_body["data"]["streak_checkin"], 5);
    assert_eq!(me_body["data"]["total_checkin"], 5);

    let (list_status, list_body) = app
        .request("GET", "/api/v1/checkins?limit=10", Some(&token), None)
        .await;

    assert_eq!(list_status, StatusCode::OK);
    assert_eq!(list_body["data"].as_array().map(Vec::len), Some(3));
}

#[tokio::test]
async fn checkin_makeup_fails_when_gold_is_insufficient() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("erin", "password123").await;
    let today = Utc::now().date_naive();

    app.update_user_state("erin", today.checked_sub_days(Days::new(2)), 1, 1, 5, 2)
        .await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/checkins",
            Some(&token),
            Some(json!({ "makeup": true })),
        )
        .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INSUFFICIENT_GOLD");
}

#[tokio::test]
async fn checkin_makeup_fails_when_diamond_is_insufficient() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("frank", "password123").await;
    let today = Utc::now().date_naive();

    app.update_user_state("frank", today.checked_sub_days(Days::new(2)), 1, 1, 20, 0)
        .await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/checkins",
            Some(&token),
            Some(json!({ "makeup": true })),
        )
        .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INSUFFICIENT_DIAMONDS");
}

#[tokio::test]
async fn checkin_makeup_false_with_gap_resets_streak() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("gina", "password123").await;
    let today = Utc::now().date_naive();

    // Last checkin was 3 days ago, streak was 5
    app.update_user_state("gina", today.checked_sub_days(Days::new(3)), 5, 10, 100, 5)
        .await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/checkins",
            Some(&token),
            Some(json!({ "makeup": false })),
        )
        .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["makeup_applied"], false);
    assert_eq!(body["data"]["streak_checkin"], 1);
    assert_eq!(body["data"]["total_checkin"], 11);
    assert_eq!(body["data"]["gold_reward"], 1);
}

#[tokio::test]
async fn checkin_no_gap_makeup_ignored() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("harry", "password123").await;
    let today = Utc::now().date_naive();

    // Last checkin was yesterday — no gap
    app.update_user_state("harry", today.checked_sub_days(Days::new(1)), 3, 3, 100, 5)
        .await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/checkins",
            Some(&token),
            Some(json!({ "makeup": true })),
        )
        .await;

    assert_eq!(status, StatusCode::CREATED);
    // No makeup applied since there's no gap
    assert_eq!(body["data"]["makeup_applied"], false);
    assert_eq!(body["data"]["makeup_days"], 0);
    assert_eq!(body["data"]["gold_cost"], 0);
    assert_eq!(body["data"]["diamond_cost"], 0);
    assert_eq!(body["data"]["streak_checkin"], 4);
    assert_eq!(body["data"]["gold_reward"], 4);

    // Balance should not be deducted
    let (_, me_body) = app.request("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(me_body["data"]["gold"], 104); // 100 + 4 reward
    assert_eq!(me_body["data"]["diamond"], 5); // unchanged
}

#[tokio::test]
async fn checkin_list_respects_limit() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("iris", "password123").await;
    let today = Utc::now().date_naive();

    // Set user state so we can create multiple checkin records
    // We'll insert 3 checkin records via makeup
    app.update_user_state("iris", today.checked_sub_days(Days::new(3)), 1, 1, 100, 5)
        .await;

    // Makeup will create 2 makeup records + 1 today = 3 total
    app.request(
        "POST",
        "/api/v1/checkins",
        Some(&token),
        Some(json!({ "makeup": true })),
    )
    .await;

    // List with limit=2 should only return 2
    let (status, body) = app
        .request("GET", "/api/v1/checkins?limit=2", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().map(Vec::len), Some(2));

    // List with no limit should return all 3
    let (status, body) = app
        .request("GET", "/api/v1/checkins", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().map(Vec::len), Some(3));
}
