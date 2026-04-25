#![allow(dead_code)]

use std::collections::BTreeMap;
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
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};
use zhiying_backend::{
    auth::ServiceKind,
    build_app,
    config::Config,
    entities::{
        code_video, common::ProblemAnswer, interactive_html, knowledge_explanation,
        knowledge_video, problem, study_quiz, study_quiz_problem, study_stage, study_subject,
        study_task, user,
    },
};

pub struct TestApp {
    pub app: Router,
    pub config: Config,
    pub mock: MockServer,
    pub _temp_dir: TempDir,
}

impl TestApp {
    pub async fn new() -> Self {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let database_path = temp_dir.path().join("test.db");
        let mock = MockServer::start().await;
        let mock_uri = mock.uri();
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
            knowledge_video_service_url: format!("{mock_uri}/knowledge-videos"),
            code_video_service_url: format!("{mock_uri}/code-videos"),
            interactive_html_service_url: format!("{mock_uri}/interactive-htmls"),
            knowledge_explanation_service_url: format!("{mock_uri}/knowledge-explanations"),
            knowledge_video_api_key: "sk-test-knowledge-video".to_owned(),
            code_video_api_key: "sk-test-code-video".to_owned(),
            interactive_html_api_key: "sk-test-interactive-html".to_owned(),
            knowledge_explanation_api_key: "sk-test-knowledge-explanation".to_owned(),
            study_subject_diamond_costs: BTreeMap::from([(3, 10), (7, 20), (15, 40), (30, 80)]),
            pretest_service_url: format!("{mock_uri}/pretest"),
            pretest_api_key: "sk-test-pretest".to_owned(),
            plan_service_url: format!("{mock_uri}/plan"),
            plan_api_key: "sk-test-plan".to_owned(),
            quiz_service_url: format!("{mock_uri}/quiz"),
            quiz_api_key: "sk-test-quiz".to_owned(),
            study_quiz_free_limit_per_task: 3,
            study_quiz_extra_gold_cost: 20,
            recharge_api_key: "sk-test-recharge".to_owned(),
        };
        let app = build_app(config.clone())
            .await
            .expect("failed to build test app");

        Self {
            app,
            config,
            mock,
            _temp_dir: temp_dir,
        }
    }

    pub async fn mock_pretest_ok(&self) {
        self.mock_microservice_ok("/pretest", &self.config.pretest_api_key)
            .await;
    }

    pub async fn mock_plan_ok(&self) {
        self.mock_microservice_ok("/plan", &self.config.plan_api_key)
            .await;
    }

    pub async fn mock_quiz_ok(&self) {
        self.mock_microservice_ok("/quiz", &self.config.quiz_api_key)
            .await;
    }

    pub async fn mock_content_generation_ok(&self, service: ServiceKind) {
        let (service_path, api_key) = match service {
            ServiceKind::KnowledgeVideo => (
                "/knowledge-videos/generate",
                self.config.knowledge_video_api_key.as_str(),
            ),
            ServiceKind::CodeVideo => (
                "/code-videos/generate",
                self.config.code_video_api_key.as_str(),
            ),
            ServiceKind::InteractiveHtml => (
                "/interactive-htmls/generate",
                self.config.interactive_html_api_key.as_str(),
            ),
            ServiceKind::KnowledgeExplanation => (
                "/knowledge-explanations/generate",
                self.config.knowledge_explanation_api_key.as_str(),
            ),
            _ => panic!("unsupported content generation service kind"),
        };

        self.mock_microservice_ok(service_path, api_key).await;
    }

    async fn mock_microservice_ok(&self, service_path: &str, api_key: &str) {
        Mock::given(method("POST"))
            .and(path(service_path))
            .and(header("Authorization", format!("Bearer {api_key}")))
            .respond_with(ResponseTemplate::new(200))
            .mount(&self.mock)
            .await;
    }

    async fn send_request(&self, request: Request<Body>) -> (StatusCode, Value) {
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

    fn build_request(
        method: &str,
        path: &str,
        auth_header: Option<&str>,
        body: Option<Value>,
    ) -> Request<Body> {
        let mut request = Request::builder().method(method).uri(path);

        if let Some(auth) = auth_header {
            request = request.header(header::AUTHORIZATION, auth);
        }

        if body.is_some() {
            request = request.header(header::CONTENT_TYPE, "application/json");
        }

        request
            .body(match body {
                Some(body) => Body::from(body.to_string()),
                None => Body::empty(),
            })
            .expect("failed to build request")
    }

    pub async fn request(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let auth = token.map(|t| format!("Bearer {t}"));
        let request = Self::build_request(method, path, auth.as_deref(), body);
        self.send_request(request).await
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
        let request = Self::build_request(method, path, Some(auth_header), body);
        self.send_request(request).await
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
            diamond_cost: Set(0),
            language: Set("PYTHON".to_owned()),
            target: Set(String::new()),
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
        streak_checkins: i32,
        total_checkins: i32,
        gold: i32,
        diamond: i32,
    ) {
        let db = self.db().await;

        let existing = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&db)
            .await
            .expect("failed to query user")
            .expect("user not found");

        let mut active_user: user::ActiveModel = existing.into();
        active_user.last_checkin = Set(last_checkin);
        active_user.streak_checkins = Set(streak_checkins);
        active_user.total_checkins = Set(total_checkins);
        active_user.gold = Set(gold);
        active_user.diamond = Set(diamond);
        active_user.updated_at = Set(Utc::now());
        active_user
            .update(&db)
            .await
            .expect("failed to update user state");
    }

    pub async fn insert_knowledge_video(
        &self,
        user_id: i32,
        status: knowledge_video::KnowledgeVideoStatus,
    ) -> i32 {
        let db = self.db().await;
        let now = Utc::now();
        let record = knowledge_video::ActiveModel {
            user_id: Set(user_id),
            status: Set(status),
            prompt: Set("test prompt".to_owned()),
            url: Set(None),
            public: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("insert knowledge_video");
        record.id
    }

    pub async fn insert_code_video(
        &self,
        user_id: i32,
        status: code_video::CodeVideoStatus,
    ) -> i32 {
        let db = self.db().await;
        let now = Utc::now();
        let record = code_video::ActiveModel {
            user_id: Set(user_id),
            status: Set(status),
            prompt: Set("test prompt".to_owned()),
            url: Set(None),
            public: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("insert code_video");
        record.id
    }

    pub async fn insert_interactive_html(
        &self,
        user_id: i32,
        status: interactive_html::InteractiveHtmlStatus,
    ) -> i32 {
        let db = self.db().await;
        let now = Utc::now();
        let record = interactive_html::ActiveModel {
            user_id: Set(user_id),
            status: Set(status),
            prompt: Set("test prompt".to_owned()),
            url: Set(None),
            public: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("insert interactive_html");
        record.id
    }

    pub async fn insert_knowledge_explanation(
        &self,
        user_id: i32,
        status: knowledge_explanation::KnowledgeExplanationStatus,
        cost: i32,
    ) -> i32 {
        let db = self.db().await;
        let now = Utc::now();
        let record = knowledge_explanation::ActiveModel {
            user_id: Set(user_id),
            status: Set(status),
            prompt: Set("test prompt".to_owned()),
            content: Set(None),
            mindmap: Set(None),
            public: Set(false),
            cost: Set(cost),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("insert knowledge_explanation");
        record.id
    }

    pub async fn insert_study_subject(
        &self,
        user_id: i32,
        subject: &str,
        status: study_subject::StudySubjectStatus,
    ) -> i32 {
        let db = self.db().await;
        let now = Utc::now();
        let record = study_subject::ActiveModel {
            user_id: Set(user_id),
            subject: Set(subject.to_owned()),
            status: Set(status),
            total_stages: Set(0),
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
        .expect("insert study_subject");
        record.id
    }
}
