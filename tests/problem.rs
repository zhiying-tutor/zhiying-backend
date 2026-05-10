mod common;

use axum::http::StatusCode;
use serde_json::json;
use zhiying_backend::entities::common::ProblemAnswer;

use common::TestApp;

#[tokio::test]
async fn me_mistakes_lists_only_wrong_answers() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (quiz_id, sqp_ids) = app
        .insert_quiz_with_problems(
            1,
            task_ids[0][0],
            &[
                ("Q1", ProblemAnswer::A),
                ("Q2", ProblemAnswer::B),
                ("Q3", ProblemAnswer::C),
            ],
            0,
        )
        .await;

    // Q1 correct (A), Q2 wrong (A vs B), Q3 not answered
    for (sqp_id, chosen) in [(sqp_ids[0].0, "A"), (sqp_ids[1].0, "A")] {
        app.request(
            "PATCH",
            &format!("/api/v1/study-quizzes/{quiz_id}/problems/{sqp_id}"),
            Some(&token),
            Some(json!({ "chosen_answer": chosen })),
        )
        .await;
    }

    let (status, body) = app
        .request("GET", "/api/v1/me/mistakes", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let arr = body["data"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["content"], "Q2");
    assert_eq!(arr[0]["source"]["quiz_id"], quiz_id);
}

#[tokio::test]
async fn me_mistakes_hides_when_marked_unless_include_hidden() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (quiz_id, sqp_ids) = app
        .insert_quiz_with_problems(
            1,
            task_ids[0][0],
            &[("Q1", ProblemAnswer::A), ("Q2", ProblemAnswer::B)],
            0,
        )
        .await;

    // Both wrong
    for sqp_id in [sqp_ids[0].0, sqp_ids[1].0] {
        app.request(
            "PATCH",
            &format!("/api/v1/study-quizzes/{quiz_id}/problems/{sqp_id}"),
            Some(&token),
            Some(json!({ "chosen_answer": "D" })),
        )
        .await;
    }

    // Hide first
    let (status, _) = app
        .request(
            "PATCH",
            &format!("/api/v1/quiz-problems/{}/mistake-visibility", sqp_ids[0].0),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (_, body) = app
        .request("GET", "/api/v1/me/mistakes", Some(&token), None)
        .await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);

    let (_, body) = app
        .request(
            "GET",
            "/api/v1/me/mistakes?include_hidden=true",
            Some(&token),
            None,
        )
        .await;
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn me_bookmarks_returns_only_bookmarked() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (_, sqp_ids) = app
        .insert_quiz_with_problems(
            1,
            task_ids[0][0],
            &[("Q1", ProblemAnswer::A), ("Q2", ProblemAnswer::B)],
            0,
        )
        .await;

    let (status, body) = app
        .request(
            "PATCH",
            &format!("/api/v1/quiz-problems/{}/bookmark", sqp_ids[0].0),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["bookmarked"], true);

    let (_, body) = app
        .request("GET", "/api/v1/me/bookmarks", Some(&token), None)
        .await;
    let arr = body["data"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["content"], "Q1");
}

#[tokio::test]
async fn quiz_problem_toggle_other_user_returns_404() {
    let app = TestApp::new().await;
    app.create_user_and_login("alice", "password123").await;
    let token_bob = app.create_user_and_login("bob", "password123").await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;
    let (_, sqp_ids) = app
        .insert_quiz_with_problems(1, task_ids[0][0], &[("Q1", ProblemAnswer::A)], 0)
        .await;

    let (status, body) = app
        .request(
            "PATCH",
            &format!("/api/v1/quiz-problems/{}/bookmark", sqp_ids[0].0),
            Some(&token_bob),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "STUDY_QUIZ_PROBLEM_NOT_FOUND");
}

#[tokio::test]
async fn me_mistakes_empty_for_new_user() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (status, body) = app
        .request("GET", "/api/v1/me/mistakes", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().map(Vec::len), Some(0));
}
