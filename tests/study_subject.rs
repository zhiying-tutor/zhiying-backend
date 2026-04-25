mod common;

use axum::http::StatusCode;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serde_json::json;
use wiremock::{
    Mock, ResponseTemplate,
    matchers::{body_json, body_partial_json, header, method, path},
};
use zhiying_backend::entities::{common::ProblemAnswer, pretest_problem, problem, study_subject};

use common::TestApp;

#[tokio::test]
async fn study_subject_create_insufficient_diamonds_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    // User has 5 diamonds, total_stages=3 costs 10
    app.update_user_state("alice", None, 0, 0, 0, 5).await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/study-subjects",
            Some(&token),
            Some(json!({
                "subject": "Python basics",
                "total_stages": 3,
                "language": "PYTHON",
                "target": ""
            })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INSUFFICIENT_DIAMONDS");

    // Diamond unchanged and no study_subject was created
    let (_, me_body) = app.request("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(me_body["data"]["diamond"], 5);

    let (_, list_body) = app
        .request("GET", "/api/v1/study-subjects", Some(&token), None)
        .await;
    assert_eq!(list_body["data"].as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn study_subject_list_returns_user_subjects() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    for i in 0..2 {
        app.insert_study_subject(
            1,
            &format!("Subject {i}"),
            study_subject::StudySubjectStatus::PretestQueuing,
        )
        .await;
    }

    let (status, body) = app
        .request("GET", "/api/v1/study-subjects", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().map(Vec::len), Some(2));
}

#[tokio::test]
async fn study_subject_get_by_id_works() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    // Needs specific total_stages/finished_stages, so use manual insert
    let db = app.db().await;
    let now = Utc::now();
    study_subject::ActiveModel {
        user_id: Set(1),
        subject: Set("Calculus".to_owned()),
        status: Set(study_subject::StudySubjectStatus::Studying),
        total_stages: Set(3),
        finished_stages: Set(1),
        diamond_cost: Set(10),
        language: Set("PYTHON".to_owned()),
        target: Set(String::new()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

    let (status, body) = app
        .request("GET", "/api/v1/study-subjects/1", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["subject"], "Calculus");
    assert_eq!(body["data"]["total_stages"], 3);
    assert_eq!(body["data"]["finished_stages"], 1);
}

#[tokio::test]
async fn study_subject_get_other_users_returns_404() {
    let app = TestApp::new().await;
    app.create_user_and_login("alice", "password123").await;
    let token_bob = app.create_user_and_login("bob", "password123").await;

    app.insert_study_subject(
        1, // alice
        "Secret Subject",
        study_subject::StudySubjectStatus::Studying,
    )
    .await;

    let (status, body) = app
        .request("GET", "/api/v1/study-subjects/1", Some(&token_bob), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "STUDY_SUBJECT_NOT_FOUND");
}

#[tokio::test]
async fn study_subject_pretest_callback_creates_problems() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let api_key = &app.config.pretest_api_key;

    app.insert_study_subject(1, "Math", study_subject::StudySubjectStatus::PretestQueuing)
        .await;

    // Callback: FINISHED with problems
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/internal/study-subjects/1",
            Some(api_key),
            Some(json!({
                "status": "FINISHED",
                "problems": [
                    {
                        "content": "What is 1+1?",
                        "choice_a": "1", "choice_b": "2", "choice_c": "3", "choice_d": "4",
                        "answer": "B", "explanation": "Basic arithmetic"
                    },
                    {
                        "content": "What is 2*3?",
                        "choice_a": "5", "choice_b": "6", "choice_c": "7", "choice_d": "8",
                        "answer": "B", "explanation": "Multiplication"
                    }
                ]
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Verify subject is now PretestReady
    let (status, body) = app
        .request("GET", "/api/v1/study-subjects/1", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "PretestReady");

    // Verify pretest has 2 problems
    let (status, body) = app
        .request(
            "GET",
            "/api/v1/study-subjects/1/pretest",
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let problems = body["data"].as_array().expect("array");
    assert_eq!(problems.len(), 2);
    assert_eq!(problems[0]["sort_order"], 0);
    assert_eq!(problems[1]["sort_order"], 1);
    assert_eq!(problems[0]["problem"]["content"], "What is 1+1?");
}

#[tokio::test]
async fn study_subject_pretest_answer_works() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let api_key = &app.config.pretest_api_key;

    app.insert_study_subject(1, "Math", study_subject::StudySubjectStatus::PretestQueuing)
        .await;

    // Create pretest via callback
    app.request(
        "POST",
        "/api/v1/internal/study-subjects/1",
        Some(api_key),
        Some(json!({
            "status": "FINISHED",
            "problems": [{
                "content": "Q1", "choice_a": "A", "choice_b": "B",
                "choice_c": "C", "choice_d": "D", "answer": "A", "explanation": "E"
            }]
        })),
    )
    .await;

    // Get pretest to find pretest_problem_id
    let (_, pretest_body) = app
        .request(
            "GET",
            "/api/v1/study-subjects/1/pretest",
            Some(&token),
            None,
        )
        .await;
    let pp_id = pretest_body["data"][0]["id"].as_i64().expect("id");

    // Answer the pretest problem
    let (status, _) = app
        .request(
            "PATCH",
            &format!("/api/v1/study-subjects/1/pretest/{pp_id}"),
            Some(&token),
            Some(json!({"chosen_answer": "A", "confidence": "VerySure"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Verify answer is stored
    let (_, pretest_body) = app
        .request(
            "GET",
            "/api/v1/study-subjects/1/pretest",
            Some(&token),
            None,
        )
        .await;
    assert_eq!(pretest_body["data"][0]["chosen_answer"], "A");
    assert_eq!(pretest_body["data"][0]["confidence"], "VerySure");
}

#[tokio::test]
async fn study_subject_pretest_answer_not_ready_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    // Subject still in PretestQueuing
    app.insert_study_subject(1, "Math", study_subject::StudySubjectStatus::PretestQueuing)
        .await;

    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/study-subjects/1/pretest/1",
            Some(&token),
            Some(json!({"chosen_answer": "A", "confidence": "NotSure"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_STUDY_SUBJECT_STATUS");
}

#[tokio::test]
async fn study_subject_pretest_answer_invalid_problem_returns_404() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    app.insert_study_subject(1, "Math", study_subject::StudySubjectStatus::PretestReady)
        .await;

    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/study-subjects/1/pretest/999",
            Some(&token),
            Some(json!({"chosen_answer": "A", "confidence": "NotSure"})),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "PROBLEM_NOT_FOUND");
}

#[tokio::test]
async fn study_subject_create_plan_not_ready_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    app.insert_study_subject(1, "Math", study_subject::StudySubjectStatus::PretestQueuing)
        .await;

    let (status, body) = app
        .request("POST", "/api/v1/study-subjects/1/plan", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_STUDY_SUBJECT_STATUS");
}

#[tokio::test]
async fn study_subject_plan_callback_creates_stages_and_tasks() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let api_key = &app.config.plan_api_key;

    // Subject was created with total_stages=2 (user-selected, plan must match)
    let db = app.db().await;
    let now = Utc::now();
    study_subject::ActiveModel {
        user_id: Set(1),
        subject: Set("Python".to_owned()),
        status: Set(study_subject::StudySubjectStatus::PlanQueuing),
        total_stages: Set(2),
        finished_stages: Set(0),
        diamond_cost: Set(10),
        language: Set("PYTHON".to_owned()),
        target: Set(String::new()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

    // Plan callback: FINISHED with stages
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/internal/study-subjects/1",
            Some(api_key),
            Some(json!({
                "status": "FINISHED",
                "stages": [
                    {
                        "title": "Basics",
                        "description": "Python basics",
                        "tasks": [
                            {"title": "Variables", "description": "Learn variables"},
                            {"title": "Functions", "description": "Learn functions"}
                        ]
                    },
                    {
                        "title": "Advanced",
                        "description": "Advanced topics",
                        "tasks": [
                            {"title": "Classes", "description": "Learn OOP"}
                        ]
                    }
                ]
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Verify subject is Studying with 2 stages
    let (status, body) = app
        .request("GET", "/api/v1/study-subjects/1", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "Studying");
    assert_eq!(body["data"]["total_stages"], 2);

    // Stage 1 should be STUDYING with 2 tasks
    let (status, body) = app
        .request("GET", "/api/v1/study-stages/1", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "Studying");
    assert_eq!(body["data"]["title"], "Basics");
    let tasks = body["data"]["tasks"].as_array().expect("tasks");
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0]["status"], "Studying"); // first task unlocked
    assert_eq!(tasks[1]["status"], "Locked"); // second task locked

    // Stage 2 should be LOCKED
    let (status, body) = app
        .request("GET", "/api/v1/study-stages/2", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "Locked");
}

#[tokio::test]
async fn study_subject_plan_callback_failed_refunds_diamond() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let api_key = &app.config.plan_api_key;

    // User has 40 diamonds (10 already deducted)
    app.update_user_state("alice", None, 0, 0, 0, 40).await;

    app.insert_study_subject(1, "Python", study_subject::StudySubjectStatus::PlanQueuing)
        .await;

    let (status, _) = app
        .request(
            "POST",
            "/api/v1/internal/study-subjects/1",
            Some(api_key),
            Some(json!({"status": "FAILED"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Diamond refunded: 40 + 10 = 50
    let (_, me_body) = app.request("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(me_body["data"]["diamond"], 50);
}

#[tokio::test]
async fn study_subject_pretest_callback_failed_refunds_diamond() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let api_key = &app.config.pretest_api_key;

    app.update_user_state("alice", None, 0, 0, 0, 40).await;

    app.insert_study_subject(
        1,
        "Python",
        study_subject::StudySubjectStatus::PretestQueuing,
    )
    .await;

    let (status, _) = app
        .request(
            "POST",
            "/api/v1/internal/study-subjects/1",
            Some(api_key),
            Some(json!({"status": "FAILED"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (_, me_body) = app.request("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(me_body["data"]["diamond"], 50);
}

#[tokio::test]
async fn study_subject_get_nonexistent_returns_404() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (status, body) = app
        .request("GET", "/api/v1/study-subjects/999", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "STUDY_SUBJECT_NOT_FOUND");
}

#[tokio::test]
async fn study_subject_pretest_answer_already_answered_overwrites() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let api_key = &app.config.pretest_api_key;

    app.insert_study_subject(1, "Math", study_subject::StudySubjectStatus::PretestQueuing)
        .await;

    // Create pretest via callback
    app.request(
        "POST",
        "/api/v1/internal/study-subjects/1",
        Some(api_key),
        Some(json!({
            "status": "FINISHED",
            "problems": [{
                "content": "Q1", "choice_a": "A", "choice_b": "B",
                "choice_c": "C", "choice_d": "D", "answer": "A", "explanation": "E"
            }]
        })),
    )
    .await;

    let (_, pretest_body) = app
        .request(
            "GET",
            "/api/v1/study-subjects/1/pretest",
            Some(&token),
            None,
        )
        .await;
    let pp_id = pretest_body["data"][0]["id"].as_i64().expect("id");

    // First answer
    app.request(
        "PATCH",
        &format!("/api/v1/study-subjects/1/pretest/{pp_id}"),
        Some(&token),
        Some(json!({"chosen_answer": "A", "confidence": "VerySure"})),
    )
    .await;

    // Overwrite answer
    let (status, _) = app
        .request(
            "PATCH",
            &format!("/api/v1/study-subjects/1/pretest/{pp_id}"),
            Some(&token),
            Some(json!({"chosen_answer": "C", "confidence": "NotSure"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Verify overwritten
    let (_, pretest_body) = app
        .request(
            "GET",
            "/api/v1/study-subjects/1/pretest",
            Some(&token),
            None,
        )
        .await;
    assert_eq!(pretest_body["data"][0]["chosen_answer"], "C");
    assert_eq!(pretest_body["data"][0]["confidence"], "NotSure");
}

#[tokio::test]
async fn study_subject_create_plan_studying_status_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    // Subject in Studying status (via insert_study_subject_with_plan)
    let (subject_id, _, _) = app.insert_study_subject_with_plan(1, 1, 1).await;

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/study-subjects/{subject_id}/plan"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_STUDY_SUBJECT_STATUS");
}

#[tokio::test]
async fn study_subject_get_pretest_other_user_returns_404() {
    let app = TestApp::new().await;
    app.create_user_and_login("alice", "password123").await;
    let token_bob = app.create_user_and_login("bob", "password123").await;
    let api_key = &app.config.pretest_api_key;

    app.insert_study_subject(
        1, // alice
        "Math",
        study_subject::StudySubjectStatus::PretestQueuing,
    )
    .await;

    // Create pretest
    app.request(
        "POST",
        "/api/v1/internal/study-subjects/1",
        Some(api_key),
        Some(json!({
            "status": "FINISHED",
            "problems": [{
                "content": "Q1", "choice_a": "A", "choice_b": "B",
                "choice_c": "C", "choice_d": "D", "answer": "A", "explanation": "E"
            }]
        })),
    )
    .await;

    // Bob tries to access Alice's pretest
    let (status, body) = app
        .request(
            "GET",
            "/api/v1/study-subjects/1/pretest",
            Some(&token_bob),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "STUDY_SUBJECT_NOT_FOUND");
}

#[tokio::test]
async fn study_subject_callback_wrong_service_key_rejected() {
    let app = TestApp::new().await;
    app.create_user_and_login("alice", "password123").await;

    app.insert_study_subject(1, "Math", study_subject::StudySubjectStatus::PretestQueuing)
        .await;

    // Use quiz key on study-subjects callback
    let wrong_key = &app.config.quiz_api_key;
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/internal/study-subjects/1",
            Some(wrong_key),
            Some(json!({"status": "GENERATING"})),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "INVALID_API_KEY");
}

#[tokio::test]
async fn study_subject_callback_nonexistent_returns_404() {
    let app = TestApp::new().await;
    let api_key = &app.config.pretest_api_key;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/internal/study-subjects/999",
            Some(api_key),
            Some(json!({"status": "GENERATING"})),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "STUDY_SUBJECT_NOT_FOUND");
}

#[tokio::test]
async fn public_config_returns_pricing_in_ascending_order() {
    let app = TestApp::new().await;

    let (status, body) = app.request("GET", "/api/v1/config", None, None).await;
    assert_eq!(status, StatusCode::OK);

    let pricing = body["data"]["study_subject"]["pricing"]
        .as_array()
        .expect("pricing array");
    assert_eq!(pricing.len(), 4);
    assert_eq!(pricing[0]["total_stages"], 3);
    assert_eq!(pricing[0]["diamond_cost"], 10);
    assert_eq!(pricing[1]["total_stages"], 7);
    assert_eq!(pricing[1]["diamond_cost"], 20);
    assert_eq!(pricing[2]["total_stages"], 15);
    assert_eq!(pricing[2]["diamond_cost"], 40);
    assert_eq!(pricing[3]["total_stages"], 30);
    assert_eq!(pricing[3]["diamond_cost"], 80);
}

#[tokio::test]
async fn study_subject_create_with_total_stages_seven_charges_twenty_diamonds() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    app.update_user_state("alice", None, 0, 0, 0, 100).await;

    Mock::given(method("POST"))
        .and(path("/pretest"))
        .and(header(
            "Authorization",
            format!("Bearer {}", app.config.pretest_api_key),
        ))
        .and(body_partial_json(json!({
            "task_id": 1,
            "prompt": "Rust 进阶",
            "total_stages": 7,
            "language": "RUST",
            "target": "能独立写一个 web 服务"
        })))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.mock)
        .await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/study-subjects",
            Some(&token),
            Some(json!({
                "subject": "Rust 进阶",
                "total_stages": 7,
                "language": "RUST",
                "target": "能独立写一个 web 服务"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["diamond_cost"], 20);
    assert_eq!(body["data"]["status"], "PretestQueuing");

    let (_, me_body) = app.request("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(me_body["data"]["diamond"], 80);

    let (_, list_body) = app
        .request("GET", "/api/v1/study-subjects", Some(&token), None)
        .await;
    let items = list_body["data"].as_array().expect("array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["total_stages"], 7);
    assert_eq!(items[0]["diamond_cost"], 20);
    assert_eq!(items[0]["language"], "RUST");
    assert_eq!(items[0]["target"], "能独立写一个 web 服务");
    assert_eq!(items[0]["subject"], "Rust 进阶");
    assert_eq!(items[0]["status"], "PretestQueuing");
}

#[tokio::test]
async fn study_subject_create_dispatch_failure_refunds_twenty_diamonds() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    app.update_user_state("alice", None, 0, 0, 0, 100).await;

    Mock::given(method("POST"))
        .and(path("/pretest"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&app.mock)
        .await;

    let (status, _) = app
        .request(
            "POST",
            "/api/v1/study-subjects",
            Some(&token),
            Some(json!({
                "subject": "Rust 进阶",
                "total_stages": 7,
                "language": "RUST",
                "target": "能独立写一个 web 服务"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);

    let (_, me_body) = app.request("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(me_body["data"]["diamond"], 100);

    let (_, list_body) = app
        .request("GET", "/api/v1/study-subjects", Some(&token), None)
        .await;
    let items = list_body["data"].as_array().expect("array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["diamond_cost"], 20);
    assert_eq!(items[0]["status"], "Failed");
}

#[tokio::test]
async fn study_subject_plan_dispatch_payload_includes_pretest_results() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let db = app.db().await;
    let now = Utc::now();
    study_subject::ActiveModel {
        user_id: Set(1),
        subject: Set("Python".to_owned()),
        status: Set(study_subject::StudySubjectStatus::PretestReady),
        total_stages: Set(7),
        finished_stages: Set(0),
        diamond_cost: Set(20),
        language: Set("PYTHON".to_owned()),
        target: Set("通过课前测定制计划".to_owned()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert subject");

    problem::ActiveModel {
        user_id: Set(1),
        content: Set("什么是所有权？".to_owned()),
        choice_a: Set("变量绑定规则".to_owned()),
        choice_b: Set("垃圾回收".to_owned()),
        choice_c: Set("线程模型".to_owned()),
        choice_d: Set("网络协议".to_owned()),
        answer: Set(ProblemAnswer::A),
        explanation: Set("Rust 通过所有权管理内存".to_owned()),
        bookmarked: Set(false),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert problem");

    pretest_problem::ActiveModel {
        study_subject_id: Set(1),
        problem_id: Set(1),
        sort_order: Set(0),
        confidence: Set(Some(pretest_problem::PretestConfidence::VerySure)),
        chosen_answer: Set(Some(ProblemAnswer::A)),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert pretest problem");

    Mock::given(method("POST"))
        .and(path("/plan"))
        .and(header(
            "Authorization",
            format!("Bearer {}", app.config.plan_api_key),
        ))
        .and(body_json(json!({
            "task_id": 1,
            "prompt": "Python",
            "total_stages": 7,
            "language": "PYTHON",
            "target": "通过课前测定制计划",
            "pretest_results": [{
                "problem_id": 1,
                "content": "什么是所有权？",
                "choice_a": "变量绑定规则",
                "choice_b": "垃圾回收",
                "choice_c": "线程模型",
                "choice_d": "网络协议",
                "answer": "A",
                "chosen_answer": "A",
                "confidence": "VerySure"
            }]
        })))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.mock)
        .await;

    let (status, body) = app
        .request("POST", "/api/v1/study-subjects/1/plan", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "PLAN_QUEUING");
}

#[tokio::test]
async fn study_subject_create_invalid_total_stages_returns_invalid_study_stages() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    app.update_user_state("alice", None, 0, 0, 0, 100).await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/study-subjects",
            Some(&token),
            Some(json!({
                "subject": "Java",
                "total_stages": 99,
                "language": "JAVA",
                "target": ""
            })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "INVALID_STUDY_STAGES");

    // Diamonds untouched, no record persisted
    let (_, me_body) = app.request("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(me_body["data"]["diamond"], 100);
    let (_, list_body) = app
        .request("GET", "/api/v1/study-subjects", Some(&token), None)
        .await;
    assert_eq!(list_body["data"].as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn study_subject_create_missing_total_stages_returns_validation_failed() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    app.update_user_state("alice", None, 0, 0, 0, 100).await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/study-subjects",
            Some(&token),
            Some(json!({"subject": "Python", "language": "PYTHON"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_FAILED");
}

#[tokio::test]
async fn study_subject_create_missing_language_returns_validation_failed() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    app.update_user_state("alice", None, 0, 0, 0, 100).await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/study-subjects",
            Some(&token),
            Some(json!({"subject": "Python", "total_stages": 3})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_FAILED");
}
