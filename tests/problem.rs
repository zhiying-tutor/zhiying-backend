mod common;

use axum::http::StatusCode;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use zhiying_backend::entities::{common::ProblemAnswer, pretest_problem, problem, study_subject};

use common::TestApp;

#[tokio::test]
async fn problems_list_returns_user_problems() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let db = app.db().await;
    let now = Utc::now();
    for i in 0..3 {
        problem::ActiveModel {
            user_id: Set(1),
            content: Set(format!("Problem {}", i)),
            choice_a: Set("A".to_owned()),
            choice_b: Set("B".to_owned()),
            choice_c: Set("C".to_owned()),
            choice_d: Set("D".to_owned()),
            answer: Set(ProblemAnswer::A),
            explanation: Set("E".to_owned()),
            bookmarked: Set(false),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("insert");
    }

    let (status, body) = app
        .request("GET", "/api/v1/problems", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().map(Vec::len), Some(3));
}

#[tokio::test]
async fn problems_list_bookmarked_filter() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let db = app.db().await;
    let now = Utc::now();
    // 1 bookmarked, 2 not
    for i in 0..3 {
        problem::ActiveModel {
            user_id: Set(1),
            content: Set(format!("Problem {}", i)),
            choice_a: Set("A".to_owned()),
            choice_b: Set("B".to_owned()),
            choice_c: Set("C".to_owned()),
            choice_d: Set("D".to_owned()),
            answer: Set(ProblemAnswer::A),
            explanation: Set("E".to_owned()),
            bookmarked: Set(i == 0),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("insert");
    }

    let (status, body) = app
        .request(
            "GET",
            "/api/v1/problems?bookmarked=true",
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().map(Vec::len), Some(1));
    assert_eq!(body["data"][0]["bookmarked"], true);
}

#[tokio::test]
async fn problems_list_wrong_filter() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let db = app.db().await;
    let now = Utc::now();

    // Problem 1: answer=A
    let p1 = problem::ActiveModel {
        user_id: Set(1),
        content: Set("P1".to_owned()),
        choice_a: Set("A".to_owned()),
        choice_b: Set("B".to_owned()),
        choice_c: Set("C".to_owned()),
        choice_d: Set("D".to_owned()),
        answer: Set(ProblemAnswer::A),
        explanation: Set("E".to_owned()),
        bookmarked: Set(false),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

    // Problem 2: answer=B
    let p2 = problem::ActiveModel {
        user_id: Set(1),
        content: Set("P2".to_owned()),
        choice_a: Set("A".to_owned()),
        choice_b: Set("B".to_owned()),
        choice_c: Set("C".to_owned()),
        choice_d: Set("D".to_owned()),
        answer: Set(ProblemAnswer::B),
        explanation: Set("E".to_owned()),
        bookmarked: Set(false),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

    // Create a study subject for pretest_problem
    let subject = study_subject::ActiveModel {
        user_id: Set(1),
        subject: Set("Test".to_owned()),
        status: Set(study_subject::StudySubjectStatus::PretestReady),
        total_stages: Set(0),
        finished_stages: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

    // Pretest problem: P1 answered correctly (A), P2 answered wrong (A instead of B)
    pretest_problem::ActiveModel {
        study_subject_id: Set(subject.id),
        problem_id: Set(p1.id),
        sort_order: Set(0),
        confidence: Set(None),
        chosen_answer: Set(Some(ProblemAnswer::A)), // correct
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

    pretest_problem::ActiveModel {
        study_subject_id: Set(subject.id),
        problem_id: Set(p2.id),
        sort_order: Set(1),
        confidence: Set(None),
        chosen_answer: Set(Some(ProblemAnswer::A)), // wrong (answer is B)
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

    let (status, body) = app
        .request("GET", "/api/v1/problems?wrong=true", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let wrong = body["data"].as_array().expect("array");
    assert_eq!(wrong.len(), 1);
    assert_eq!(wrong[0]["content"], "P2");
}

#[tokio::test]
async fn problems_toggle_bookmark() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let db = app.db().await;
    let now = Utc::now();
    let p = problem::ActiveModel {
        user_id: Set(1),
        content: Set("Q1".to_owned()),
        choice_a: Set("A".to_owned()),
        choice_b: Set("B".to_owned()),
        choice_c: Set("C".to_owned()),
        choice_d: Set("D".to_owned()),
        answer: Set(ProblemAnswer::A),
        explanation: Set("E".to_owned()),
        bookmarked: Set(false),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

    // Toggle on
    let (status, body) = app
        .request(
            "PATCH",
            &format!("/api/v1/problems/{}/bookmark", p.id),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["bookmarked"], true);

    // Toggle off
    let (status, body) = app
        .request(
            "PATCH",
            &format!("/api/v1/problems/{}/bookmark", p.id),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["bookmarked"], false);
}
