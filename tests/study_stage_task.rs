mod common;

use axum::http::StatusCode;
use serde_json::json;

use common::TestApp;

#[tokio::test]
async fn study_stage_get_returns_tasks() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (_, stage_ids, task_ids) = app.insert_study_subject_with_plan(1, 1, 3).await;

    let (status, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-stages/{}", stage_ids[0]),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["title"], "Stage 1");
    let tasks = body["data"]["tasks"].as_array().expect("tasks");
    assert_eq!(tasks.len(), 3);
    assert_eq!(tasks[0]["id"], task_ids[0][0]);
    assert_eq!(tasks[0]["status"], "Studying");
    assert_eq!(tasks[1]["status"], "Locked");
}

#[tokio::test]
async fn study_stage_nonexistent_returns_404() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let (status, body) = app
        .request("GET", "/api/v1/study-stages/999", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "STAGE_NOT_FOUND");
}

#[tokio::test]
async fn study_stage_other_users_returns_404() {
    let app = TestApp::new().await;
    app.create_user_and_login("alice", "password123").await;
    let token_bob = app.create_user_and_login("bob", "password123").await;

    let (_, stage_ids, _) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (status, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-stages/{}", stage_ids[0]),
            Some(&token_bob),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "STAGE_NOT_FOUND");
}

#[tokio::test]
async fn study_task_get_returns_data() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (status, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-tasks/{}", task_ids[0][0]),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["title"], "Task 1.1");
    assert_eq!(body["data"]["status"], "Studying");
}

#[tokio::test]
async fn study_task_nonexistent_returns_404() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let (status, body) = app
        .request("GET", "/api/v1/study-tasks/999", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "TASK_NOT_FOUND");
}

#[tokio::test]
async fn study_task_complete_unlocks_next() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (_, stage_ids, task_ids) = app.insert_study_subject_with_plan(1, 1, 2).await;

    // Complete first task
    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/study-tasks/{}/complete", task_ids[0][0]),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // First task is now Finished
    let (_, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-tasks/{}", task_ids[0][0]),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(body["data"]["status"], "Finished");

    // Second task is now Studying
    let (_, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-tasks/{}", task_ids[0][1]),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(body["data"]["status"], "Studying");

    // Stage has finished_tasks=1
    let (_, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-stages/{}", stage_ids[0]),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(body["data"]["finished_tasks"], 1);
}

#[tokio::test]
async fn study_task_complete_locked_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 2).await;

    // Try to complete the second (locked) task
    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/study-tasks/{}/complete", task_ids[0][1]),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_STUDY_TASK_STATUS");
}

#[tokio::test]
async fn study_task_complete_finished_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 2).await;

    // Complete first task
    app.request(
        "POST",
        &format!("/api/v1/study-tasks/{}/complete", task_ids[0][0]),
        Some(&token),
        None,
    )
    .await;

    // Try to complete it again
    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/study-tasks/{}/complete", task_ids[0][0]),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_STUDY_TASK_STATUS");
}

#[tokio::test]
async fn study_task_complete_last_in_stage_finishes_stage_unlocks_next() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    // 2 stages, 1 task each
    let (subject_id, stage_ids, task_ids) = app.insert_study_subject_with_plan(1, 2, 1).await;

    // Complete the only task in stage 1
    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/study-tasks/{}/complete", task_ids[0][0]),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Stage 1 is Finished
    let (_, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-stages/{}", stage_ids[0]),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(body["data"]["status"], "Finished");

    // Stage 2 is now Studying
    let (_, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-stages/{}", stage_ids[1]),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(body["data"]["status"], "Studying");

    // Stage 2's task is now Studying
    let (_, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-tasks/{}", task_ids[1][0]),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(body["data"]["status"], "Studying");

    // Subject finished_stages = 1
    let (_, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-subjects/{subject_id}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(body["data"]["finished_stages"], 1);
    assert_eq!(body["data"]["status"], "Studying");
}

#[tokio::test]
async fn study_task_complete_last_in_last_stage_finishes_subject() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    // 1 stage, 1 task
    let (subject_id, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/study-tasks/{}/complete", task_ids[0][0]),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Subject is Finished
    let (_, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-subjects/{subject_id}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(body["data"]["status"], "Finished");
    assert_eq!(body["data"]["finished_stages"], 1);
}

#[tokio::test]
async fn study_task_kv_locked_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    app.update_user_state("alice", None, 0, 0, 100, 50).await;

    // 1 stage, 2 tasks — second task is locked
    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 2).await;

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/study-tasks/{}/knowledge-video", task_ids[0][1]),
            Some(&token),
            Some(json!({"prompt": "test"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_STUDY_TASK_STATUS");
}

#[tokio::test]
async fn study_task_ih_insufficient_gold_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    // 0 gold
    app.update_user_state("alice", None, 0, 0, 0, 50).await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/study-tasks/{}/interactive-html", task_ids[0][0]),
            Some(&token),
            Some(json!({"prompt": "test"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INSUFFICIENT_GOLD");
}

#[tokio::test]
async fn study_task_kv_insufficient_diamonds_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    // 0 diamonds
    app.update_user_state("alice", None, 0, 0, 100, 0).await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/study-tasks/{}/knowledge-video", task_ids[0][0]),
            Some(&token),
            Some(json!({"prompt": "test"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INSUFFICIENT_DIAMONDS");
}

#[tokio::test]
async fn study_task_explanation_locked_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 2).await;

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/study-tasks/{}/explanation", task_ids[0][1]),
            Some(&token),
            Some(json!({"prompt": "test"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_STUDY_TASK_STATUS");
}
