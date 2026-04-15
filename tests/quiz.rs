mod common;

use axum::http::StatusCode;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serde_json::json;
use zhiying_backend::entities::{common::ProblemAnswer, problem, study_quiz, study_quiz_problem};

use common::TestApp;

#[tokio::test]
async fn study_quiz_callback_creates_problems() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let api_key = &app.config.quiz_api_key;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    // Insert a quiz in Queuing state
    let db = app.db().await;
    let now = Utc::now();
    study_quiz::ActiveModel {
        study_task_id: Set(task_ids[0][0]),
        status: Set(study_quiz::StudyQuizStatus::Queuing),
        cost: Set(0),
        total_problems: Set(0),
        correct_problems: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert quiz");

    // Callback: FINISHED with problems
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/internal/study-quizzes/1",
            Some(api_key),
            Some(json!({
                "status": "FINISHED",
                "problems": [
                    {
                        "content": "Quiz Q1",
                        "choice_a": "A", "choice_b": "B", "choice_c": "C", "choice_d": "D",
                        "answer": "A", "explanation": "Explanation 1"
                    },
                    {
                        "content": "Quiz Q2",
                        "choice_a": "A", "choice_b": "B", "choice_c": "C", "choice_d": "D",
                        "answer": "C", "explanation": "Explanation 2"
                    }
                ]
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Verify quiz is Ready with 2 problems
    let (status, body) = app
        .request("GET", "/api/v1/study-quizzes/1", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "Ready");
    assert_eq!(body["data"]["total_problems"], 2);
    let problems = body["data"]["problems"].as_array().expect("problems");
    assert_eq!(problems.len(), 2);
}

#[tokio::test]
async fn study_quiz_callback_failed_refunds_gold() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let api_key = &app.config.quiz_api_key;

    // User has 80 gold (20 already deducted for quiz)
    app.update_user_state("alice", None, 0, 0, 80, 50).await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let db = app.db().await;
    let now = Utc::now();
    study_quiz::ActiveModel {
        study_task_id: Set(task_ids[0][0]),
        status: Set(study_quiz::StudyQuizStatus::Queuing),
        cost: Set(20),
        total_problems: Set(0),
        correct_problems: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert quiz");

    let (status, _) = app
        .request(
            "POST",
            "/api/v1/internal/study-quizzes/1",
            Some(api_key),
            Some(json!({"status": "FAILED"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Gold refunded: 80 + 20 = 100
    let (_, me_body) = app.request("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(me_body["data"]["gold"], 100);
}

#[tokio::test]
async fn study_quiz_list_for_task() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let db = app.db().await;
    let now = Utc::now();
    for _ in 0..2 {
        study_quiz::ActiveModel {
            study_task_id: Set(task_ids[0][0]),
            status: Set(study_quiz::StudyQuizStatus::Ready),
            cost: Set(0),
            total_problems: Set(5),
            correct_problems: Set(3),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("insert quiz");
    }

    let (status, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-tasks/{}/quizzes", task_ids[0][0]),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().map(Vec::len), Some(2));
}

#[tokio::test]
async fn study_quiz_get_detail_with_problems() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (quiz_id, _) = app
        .insert_quiz_with_problems(
            1,
            task_ids[0][0],
            &[("Q1", ProblemAnswer::A), ("Q2", ProblemAnswer::B)],
            0,
        )
        .await;

    let (status, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-quizzes/{quiz_id}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "Ready");
    let problems = body["data"]["problems"].as_array().expect("problems");
    assert_eq!(problems.len(), 2);
    assert_eq!(problems[0]["problem"]["content"], "Q1");
    assert_eq!(problems[0]["chosen_answer"], serde_json::Value::Null);
}

#[tokio::test]
async fn study_quiz_answer_problem() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (quiz_id, sqp_ids) = app
        .insert_quiz_with_problems(1, task_ids[0][0], &[("Q1", ProblemAnswer::A)], 0)
        .await;

    let sqp_id = sqp_ids[0].0;
    let (status, _) = app
        .request(
            "PATCH",
            &format!("/api/v1/study-quizzes/{quiz_id}/problems/{sqp_id}"),
            Some(&token),
            Some(json!({"chosen_answer": "B"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Verify answer stored
    let (_, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-quizzes/{quiz_id}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(body["data"]["problems"][0]["chosen_answer"], "B");
}

#[tokio::test]
async fn study_quiz_answer_non_ready_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let db = app.db().await;
    let now = Utc::now();
    // Quiz in Queuing state
    let quiz = study_quiz::ActiveModel {
        study_task_id: Set(task_ids[0][0]),
        status: Set(study_quiz::StudyQuizStatus::Queuing),
        cost: Set(0),
        total_problems: Set(1),
        correct_problems: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

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

    let sqp = study_quiz_problem::ActiveModel {
        study_quiz_id: Set(quiz.id),
        problem_id: Set(p.id),
        sort_order: Set(0),
        chosen_answer: Set(None),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

    let (status, body) = app
        .request(
            "PATCH",
            &format!("/api/v1/study-quizzes/{}/problems/{}", quiz.id, sqp.id),
            Some(&token),
            Some(json!({"chosen_answer": "A"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_STUDY_QUIZ_STATUS");
}

#[tokio::test]
async fn study_quiz_submit_calculates_correct() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (quiz_id, sqp_ids) = app
        .insert_quiz_with_problems(
            1,
            task_ids[0][0],
            &[("Q1", ProblemAnswer::A), ("Q2", ProblemAnswer::C)],
            0,
        )
        .await;

    // Answer Q1 correctly (A), Q2 incorrectly (B instead of C)
    app.request(
        "PATCH",
        &format!("/api/v1/study-quizzes/{quiz_id}/problems/{}", sqp_ids[0].0),
        Some(&token),
        Some(json!({"chosen_answer": "A"})),
    )
    .await;
    app.request(
        "PATCH",
        &format!("/api/v1/study-quizzes/{quiz_id}/problems/{}", sqp_ids[1].0),
        Some(&token),
        Some(json!({"chosen_answer": "B"})),
    )
    .await;

    // Submit
    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/study-quizzes/{quiz_id}/submit"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["correct_problems"], 1);

    // Verify quiz is now Submitted
    let (_, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-quizzes/{quiz_id}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(body["data"]["status"], "Submitted");
    assert_eq!(body["data"]["correct_problems"], 1);
}

#[tokio::test]
async fn study_quiz_submit_incomplete_returns_400() {
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

    // Only answer Q1
    app.request(
        "PATCH",
        &format!("/api/v1/study-quizzes/{quiz_id}/problems/{}", sqp_ids[0].0),
        Some(&token),
        Some(json!({"chosen_answer": "A"})),
    )
    .await;

    // Submit without answering Q2
    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/study-quizzes/{quiz_id}/submit"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INCOMPLETE_QUIZ_ANSWERS");
}

#[tokio::test]
async fn study_quiz_create_beyond_free_limit_insufficient_gold_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    // 10 gold, but extra quiz costs 20
    app.update_user_state("alice", None, 0, 0, 10, 50).await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    // Insert 3 existing quizzes to exhaust free limit
    let db = app.db().await;
    let now = Utc::now();
    for _ in 0..3 {
        study_quiz::ActiveModel {
            study_task_id: Set(task_ids[0][0]),
            status: Set(study_quiz::StudyQuizStatus::Ready),
            cost: Set(0),
            total_problems: Set(0),
            correct_problems: Set(0),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("insert");
    }

    // Try to create a 4th quiz (beyond free limit, costs 20 gold)
    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/study-tasks/{}/quizzes", task_ids[0][0]),
            Some(&token),
            Some(json!({"prompt": "test"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INSUFFICIENT_GOLD");
}

#[tokio::test]
async fn study_quiz_get_nonexistent_returns_404() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let (status, body) = app
        .request("GET", "/api/v1/study-quizzes/999", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "QUIZ_NOT_FOUND");
}

#[tokio::test]
async fn study_quiz_get_other_user_returns_404() {
    let app = TestApp::new().await;
    app.create_user_and_login("alice", "password123").await;
    let token_bob = app.create_user_and_login("bob", "password123").await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (quiz_id, _) = app
        .insert_quiz_with_problems(1, task_ids[0][0], &[("Q1", ProblemAnswer::A)], 0)
        .await;

    let (status, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-quizzes/{quiz_id}"),
            Some(&token_bob),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "QUIZ_NOT_FOUND");
}

#[tokio::test]
async fn study_quiz_answer_after_submit_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (quiz_id, sqp_ids) = app
        .insert_quiz_with_problems(1, task_ids[0][0], &[("Q1", ProblemAnswer::A)], 0)
        .await;

    // Answer and submit
    app.request(
        "PATCH",
        &format!("/api/v1/study-quizzes/{quiz_id}/problems/{}", sqp_ids[0].0),
        Some(&token),
        Some(json!({"chosen_answer": "A"})),
    )
    .await;
    app.request(
        "POST",
        &format!("/api/v1/study-quizzes/{quiz_id}/submit"),
        Some(&token),
        None,
    )
    .await;

    // Try to answer after submission
    let (status, body) = app
        .request(
            "PATCH",
            &format!("/api/v1/study-quizzes/{quiz_id}/problems/{}", sqp_ids[0].0),
            Some(&token),
            Some(json!({"chosen_answer": "B"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_STUDY_QUIZ_STATUS");
}

#[tokio::test]
async fn study_quiz_submit_already_submitted_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (quiz_id, sqp_ids) = app
        .insert_quiz_with_problems(1, task_ids[0][0], &[("Q1", ProblemAnswer::A)], 0)
        .await;

    // Answer and submit
    app.request(
        "PATCH",
        &format!("/api/v1/study-quizzes/{quiz_id}/problems/{}", sqp_ids[0].0),
        Some(&token),
        Some(json!({"chosen_answer": "A"})),
    )
    .await;
    app.request(
        "POST",
        &format!("/api/v1/study-quizzes/{quiz_id}/submit"),
        Some(&token),
        None,
    )
    .await;

    // Try to submit again
    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/study-quizzes/{quiz_id}/submit"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_STUDY_QUIZ_STATUS");
}

#[tokio::test]
async fn study_quiz_submit_all_correct() {
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

    // Answer both correctly
    app.request(
        "PATCH",
        &format!("/api/v1/study-quizzes/{quiz_id}/problems/{}", sqp_ids[0].0),
        Some(&token),
        Some(json!({"chosen_answer": "A"})),
    )
    .await;
    app.request(
        "PATCH",
        &format!("/api/v1/study-quizzes/{quiz_id}/problems/{}", sqp_ids[1].0),
        Some(&token),
        Some(json!({"chosen_answer": "B"})),
    )
    .await;

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/study-quizzes/{quiz_id}/submit"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["correct_problems"], 2);
}

#[tokio::test]
async fn study_quiz_submit_all_wrong() {
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

    // Answer both incorrectly
    app.request(
        "PATCH",
        &format!("/api/v1/study-quizzes/{quiz_id}/problems/{}", sqp_ids[0].0),
        Some(&token),
        Some(json!({"chosen_answer": "D"})),
    )
    .await;
    app.request(
        "PATCH",
        &format!("/api/v1/study-quizzes/{quiz_id}/problems/{}", sqp_ids[1].0),
        Some(&token),
        Some(json!({"chosen_answer": "D"})),
    )
    .await;

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/study-quizzes/{quiz_id}/submit"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["correct_problems"], 0);

    // Verify via GET
    let (_, body) = app
        .request(
            "GET",
            &format!("/api/v1/study-quizzes/{quiz_id}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(body["data"]["status"], "Submitted");
    assert_eq!(body["data"]["correct_problems"], 0);
    assert_eq!(body["data"]["total_problems"], 2);
}

#[tokio::test]
async fn study_quiz_answer_nonexistent_problem_returns_404() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (quiz_id, _) = app
        .insert_quiz_with_problems(1, task_ids[0][0], &[("Q1", ProblemAnswer::A)], 0)
        .await;

    let (status, body) = app
        .request(
            "PATCH",
            &format!("/api/v1/study-quizzes/{quiz_id}/problems/999"),
            Some(&token),
            Some(json!({"chosen_answer": "A"})),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "STUDY_QUIZ_PROBLEM_NOT_FOUND");
}

#[tokio::test]
async fn study_quiz_callback_wrong_service_key_rejected() {
    let app = TestApp::new().await;
    app.create_user_and_login("alice", "password123").await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let db = app.db().await;
    let now = Utc::now();
    study_quiz::ActiveModel {
        study_task_id: Set(task_ids[0][0]),
        status: Set(study_quiz::StudyQuizStatus::Queuing),
        cost: Set(0),
        total_problems: Set(0),
        correct_problems: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert quiz");

    // Use pretest key on quiz callback
    let wrong_key = &app.config.pretest_api_key;
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/internal/study-quizzes/1",
            Some(wrong_key),
            Some(json!({"status": "GENERATING"})),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "INVALID_API_KEY");
}

#[tokio::test]
async fn study_quiz_create_exactly_at_free_limit() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    app.update_user_state("alice", None, 0, 0, 100, 50).await;

    let (_, _, task_ids) = app.insert_study_subject_with_plan(1, 1, 1).await;

    // Insert 2 existing quizzes (free limit is 3)
    let db = app.db().await;
    let now = Utc::now();
    for _ in 0..2 {
        study_quiz::ActiveModel {
            study_task_id: Set(task_ids[0][0]),
            status: Set(study_quiz::StudyQuizStatus::Ready),
            cost: Set(0),
            total_problems: Set(0),
            correct_problems: Set(0),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("insert");
    }

    // 3rd quiz — still free (service will fail but cost check should pass)
    let (_status, body) = app
        .request(
            "POST",
            &format!("/api/v1/study-tasks/{}/quizzes", task_ids[0][0]),
            Some(&token),
            Some(json!({"prompt": "quiz 3"})),
        )
        .await;
    // The dispatch will fail (no service), but the error should NOT be INSUFFICIENT_GOLD
    assert_ne!(body["code"], "INSUFFICIENT_GOLD");
}
