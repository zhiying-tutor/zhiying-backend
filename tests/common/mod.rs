#![allow(dead_code)]

use std::net::{IpAddr, Ipv4Addr};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::Utc;
use http_body_util::BodyExt;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Database, DatabaseConnection, EntityTrait,
    QueryFilter,
};
use serde_json::Value;
use tempfile::TempDir;
use tower::util::ServiceExt;
use zhiying_backend::{
    build_app,
    config::Config,
    entities::{
        common::ProblemAnswer, problem, study_quiz, study_quiz_problem, study_stage, study_subject,
        study_task, user,
    },
};

pub struct TestApp {
    pub app: Router,
    pub config: Config,
    pub _temp_dir: TempDir,
}

impl TestApp {
    pub async fn new() -> Self {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let database_path = temp_dir.path().join("test.db");
        let config = Config {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 3000,
            database_url: format!("sqlite://{}?mode=rwc", database_path.to_string_lossy()),
            jwt_secret: "test-secret".to_owned(),
            jwt_ttl_days: 30,
            cors_allow_origin: "*".to_owned(),
            checkin_reward_sequence: vec![1, 2, 3, 4, 5, 6, 7],
            checkin_makeup_gold_cost_per_day: 10,
            checkin_makeup_diamond_cost: 1,
            knowledge_video_diamond_cost: 5,
            code_video_diamond_cost: 5,
            interactive_html_gold_cost: 10,
            knowledge_explanation_gold_cost: 10,
            knowledge_video_service_url: "http://localhost:9001".to_owned(),
            code_video_service_url: "http://localhost:9002".to_owned(),
            interactive_html_service_url: "http://localhost:9003".to_owned(),
            knowledge_explanation_service_url: "http://localhost:9004".to_owned(),
            knowledge_video_api_key: "sk-test-knowledge-video".to_owned(),
            code_video_api_key: "sk-test-code-video".to_owned(),
            interactive_html_api_key: "sk-test-interactive-html".to_owned(),
            knowledge_explanation_api_key: "sk-test-knowledge-explanation".to_owned(),
            study_subject_diamond_cost: 10,
            pretest_service_url: "http://localhost:9010".to_owned(),
            pretest_api_key: "sk-test-pretest".to_owned(),
            plan_service_url: "http://localhost:9011".to_owned(),
            plan_api_key: "sk-test-plan".to_owned(),
            quiz_service_url: "http://localhost:9012".to_owned(),
            quiz_api_key: "sk-test-quiz".to_owned(),
            study_quiz_free_limit_per_task: 3,
            study_quiz_extra_gold_cost: 20,
        };
        let app = build_app(config.clone())
            .await
            .expect("failed to build test app");

        Self {
            app,
            config,
            _temp_dir: temp_dir,
        }
    }

    pub async fn request(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(path);

        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }

        if body.is_some() {
            request = request.header(header::CONTENT_TYPE, "application/json");
        }

        let request = request
            .body(match body {
                Some(body) => Body::from(body.to_string()),
                None => Body::empty(),
            })
            .expect("failed to build request");

        let response = self
            .app
            .clone()
            .oneshot(request)
            .await
            .expect("request failed");

        let status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("failed to read response body")
            .to_bytes();
        let json = serde_json::from_slice(&bytes).expect("response is not valid json");

        (status, json)
    }

    pub async fn create_user_and_login(&self, username: &str, password: &str) -> String {
        let (create_status, _) = self
            .request(
                "POST",
                "/api/v1/users",
                None,
                Some(serde_json::json!({
                    "username": username,
                    "password": password,
                })),
            )
            .await;

        assert_eq!(create_status, StatusCode::CREATED);

        let (login_status, login_body) = self
            .request(
                "POST",
                "/api/v1/tokens",
                None,
                Some(serde_json::json!({
                    "username": username,
                    "password": password,
                })),
            )
            .await;

        assert_eq!(login_status, StatusCode::OK);

        login_body["data"]["token"]
            .as_str()
            .expect("missing token")
            .to_owned()
    }

    pub async fn db(&self) -> DatabaseConnection {
        Database::connect(&self.config.database_url)
            .await
            .expect("failed to connect test database")
    }

    pub async fn request_with_raw_auth(
        &self,
        method: &str,
        path: &str,
        auth_header: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::AUTHORIZATION, auth_header);

        if body.is_some() {
            request = request.header(header::CONTENT_TYPE, "application/json");
        }

        let request = request
            .body(match body {
                Some(body) => Body::from(body.to_string()),
                None => Body::empty(),
            })
            .expect("failed to build request");

        let response = self
            .app
            .clone()
            .oneshot(request)
            .await
            .expect("request failed");

        let status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("failed to read response body")
            .to_bytes();
        let json = serde_json::from_slice(&bytes).expect("response is not valid json");

        (status, json)
    }

    /// Insert a study subject in Studying status with stages and tasks.
    /// Returns (subject_id, stage_ids, task_ids_per_stage).
    pub async fn insert_study_subject_with_plan(
        &self,
        user_id: i32,
        num_stages: usize,
        tasks_per_stage: usize,
    ) -> (i32, Vec<i32>, Vec<Vec<i32>>) {
        let db = self.db().await;
        let now = Utc::now();

        let subject = study_subject::ActiveModel {
            user_id: Set(user_id),
            subject: Set("Test Subject".to_owned()),
            status: Set(study_subject::StudySubjectStatus::Studying),
            total_stages: Set(num_stages as i32),
            finished_stages: Set(0),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("insert study_subject");

        let mut stage_ids = Vec::new();
        let mut all_task_ids = Vec::new();

        for si in 0..num_stages {
            let stage_status = if si == 0 {
                study_stage::StudyStageStatus::Studying
            } else {
                study_stage::StudyStageStatus::Locked
            };

            let stage = study_stage::ActiveModel {
                study_subject_id: Set(subject.id),
                title: Set(format!("Stage {}", si + 1)),
                description: Set(format!("Description for stage {}", si + 1)),
                sort_order: Set(si as i32),
                status: Set(stage_status),
                total_tasks: Set(tasks_per_stage as i32),
                finished_tasks: Set(0),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(&db)
            .await
            .expect("insert study_stage");

            stage_ids.push(stage.id);

            let mut task_ids = Vec::new();
            for ti in 0..tasks_per_stage {
                let task_status = if si == 0 && ti == 0 {
                    study_task::StudyTaskStatus::Studying
                } else {
                    study_task::StudyTaskStatus::Locked
                };

                let task = study_task::ActiveModel {
                    study_stage_id: Set(stage.id),
                    title: Set(format!("Task {}.{}", si + 1, ti + 1)),
                    description: Set(format!("Description for task {}.{}", si + 1, ti + 1)),
                    sort_order: Set(ti as i32),
                    status: Set(task_status),
                    knowledge_video_id: Set(None),
                    interactive_html_id: Set(None),
                    knowledge_explanation_id: Set(None),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(&db)
                .await
                .expect("insert study_task");

                task_ids.push(task.id);
            }
            all_task_ids.push(task_ids);
        }

        (subject.id, stage_ids, all_task_ids)
    }

    /// Insert a quiz in Ready status with problems. Returns (quiz_id, vec of (sqp_id, problem_id)).
    pub async fn insert_quiz_with_problems(
        &self,
        user_id: i32,
        task_id: i32,
        problems_data: &[(&str, ProblemAnswer)],
        cost: i32,
    ) -> (i32, Vec<(i32, i32)>) {
        let db = self.db().await;
        let now = Utc::now();

        let quiz = study_quiz::ActiveModel {
            study_task_id: Set(task_id),
            status: Set(study_quiz::StudyQuizStatus::Ready),
            cost: Set(cost),
            total_problems: Set(problems_data.len() as i32),
            correct_problems: Set(0),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("insert study_quiz");

        let mut ids = Vec::new();
        for (i, (content, answer)) in problems_data.iter().enumerate() {
            let p = problem::ActiveModel {
                user_id: Set(user_id),
                content: Set(content.to_string()),
                choice_a: Set("A".to_owned()),
                choice_b: Set("B".to_owned()),
                choice_c: Set("C".to_owned()),
                choice_d: Set("D".to_owned()),
                answer: Set(*answer),
                explanation: Set("explanation".to_owned()),
                bookmarked: Set(false),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(&db)
            .await
            .expect("insert problem");

            let sqp = study_quiz_problem::ActiveModel {
                study_quiz_id: Set(quiz.id),
                problem_id: Set(p.id),
                sort_order: Set(i as i32),
                chosen_answer: Set(None),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(&db)
            .await
            .expect("insert study_quiz_problem");

            ids.push((sqp.id, p.id));
        }

        (quiz.id, ids)
    }

    pub async fn update_user_state(
        &self,
        username: &str,
        last_checkin: Option<chrono::NaiveDate>,
        streak_checkin: i32,
        total_checkin: i32,
        gold: i32,
        diamond: i32,
    ) {
        let db = Database::connect(&self.config.database_url)
            .await
            .expect("failed to connect test database");

        let existing = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&db)
            .await
            .expect("failed to query user")
            .expect("user not found");

        let mut active_user: user::ActiveModel = existing.into();
        active_user.last_checkin = Set(last_checkin);
        active_user.streak_checkin = Set(streak_checkin);
        active_user.total_checkin = Set(total_checkin);
        active_user.gold = Set(gold);
        active_user.diamond = Set(diamond);
        active_user.updated_at = Set(Utc::now());
        active_user
            .update(&db)
            .await
            .expect("failed to update user state");
    }
}
