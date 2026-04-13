use std::net::{IpAddr, Ipv4Addr};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{Days, Utc};
use http_body_util::BodyExt;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Database, EntityTrait, QueryFilter,
};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::util::ServiceExt;
use zhiying_backend::{build_app, config::Config, entities::user};

struct TestApp {
    app: Router,
    config: Config,
    _temp_dir: TempDir,
}

impl TestApp {
    async fn new() -> Self {
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

    async fn request(
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

    async fn create_user_and_login(&self, username: &str, password: &str) -> String {
        let (create_status, _) = self
            .request(
                "POST",
                "/api/v1/users",
                None,
                Some(json!({
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
                Some(json!({
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

    async fn update_user_state(
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

#[tokio::test]
async fn auth_and_me_flow_works() {
    let app = TestApp::new().await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/users",
            None,
            Some(json!({
                "username": "alice",
                "password": "password123",
            })),
        )
        .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["username"], "alice");

    let (duplicate_status, duplicate_body) = app
        .request(
            "POST",
            "/api/v1/users",
            None,
            Some(json!({
                "username": "alice",
                "password": "password123",
            })),
        )
        .await;

    assert_eq!(duplicate_status, StatusCode::CONFLICT);
    assert_eq!(duplicate_body["code"], "USERNAME_ALREADY_EXISTS");

    let (login_status, login_body) = app
        .request(
            "POST",
            "/api/v1/tokens",
            None,
            Some(json!({
                "username": "alice",
                "password": "password123",
            })),
        )
        .await;

    assert_eq!(login_status, StatusCode::OK);
    let token = login_body["data"]["token"]
        .as_str()
        .expect("missing token")
        .to_owned();

    let (me_status, me_body) = app.request("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(me_status, StatusCode::OK);
    assert_eq!(me_body["data"]["username"], "alice");

    let (update_status, update_body) = app
        .request(
            "PATCH",
            "/api/v1/me",
            Some(&token),
            Some(json!({
                "birth_year": 2010,
                "introduction": "你好，志英",
            })),
        )
        .await;

    assert_eq!(update_status, StatusCode::OK);
    assert_eq!(update_body["data"]["birth_year"], 2010);
    assert_eq!(update_body["data"]["introduction"], "你好，志英");
}

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

// --- Content generation tests ---

// Note: POST creation tests would require a running microservice to confirm queuing.
// These tests cover the internal callback endpoints, user GET/PATCH, and balance checks.

#[tokio::test]
async fn internal_callback_updates_knowledge_video_status() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("gen_user1", "password123").await;
    let api_key = &app.config.knowledge_video_api_key;

    // Give user some diamonds
    app.update_user_state("gen_user1", None, 0, 0, 100, 50)
        .await;

    // Directly insert a knowledge_video record in QUEUING state
    let db = Database::connect(&app.config.database_url)
        .await
        .expect("connect");
    use zhiying_backend::entities::knowledge_video;
    knowledge_video::ActiveModel {
        user_id: Set(1),
        status: Set(knowledge_video::KnowledgeVideoStatus::Queuing),
        prompt: Set("test prompt".to_owned()),
        url: Set(None),
        public: Set(false),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

    // Callback: QUEUING -> GENERATING
    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/internal/knowledge-videos/1",
            Some(api_key),
            Some(json!({"status": "GENERATING"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "GENERATING");

    // Callback: GENERATING -> FINISHED
    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/internal/knowledge-videos/1",
            Some(api_key),
            Some(json!({"status": "FINISHED", "url": "https://cdn.example.com/v1.mp4"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "FINISHED");

    // User can GET the finished resource
    let (status, body) = app
        .request("GET", "/api/v1/knowledge-videos/1", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["url"], "https://cdn.example.com/v1.mp4");
}

#[tokio::test]
async fn internal_callback_failed_triggers_refund() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("gen_user2", "password123").await;
    let api_key = &app.config.interactive_html_api_key;

    // Give user some gold
    app.update_user_state("gen_user2", None, 0, 0, 100, 10)
        .await;

    // Directly insert an interactive_html record
    let db = Database::connect(&app.config.database_url)
        .await
        .expect("connect");
    use zhiying_backend::entities::interactive_html;
    interactive_html::ActiveModel {
        user_id: Set(1),
        status: Set(interactive_html::InteractiveHtmlStatus::Queuing),
        prompt: Set("build a tree".to_owned()),
        url: Set(None),
        public: Set(false),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

    // Deduct the cost manually (simulating what create handler would do)
    app.update_user_state("gen_user2", None, 0, 0, 90, 10).await;

    // QUEUING -> GENERATING
    let (status, _) = app
        .request(
            "PATCH",
            "/api/v1/internal/interactive-htmls/1",
            Some(api_key),
            Some(json!({"status": "GENERATING"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // GENERATING -> FAILED (should refund)
    let (status, _) = app
        .request(
            "PATCH",
            "/api/v1/internal/interactive-htmls/1",
            Some(api_key),
            Some(json!({"status": "FAILED"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Check user gold was refunded
    let (_, me_body) = app.request("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(me_body["data"]["gold"], 100); // 90 + 10 refund
}

#[tokio::test]
async fn internal_callback_invalid_transition_rejected() {
    let app = TestApp::new().await;
    app.create_user_and_login("gen_user3", "password123").await;
    let api_key = &app.config.code_video_api_key;

    let db = Database::connect(&app.config.database_url)
        .await
        .expect("connect");
    use zhiying_backend::entities::code_video;
    code_video::ActiveModel {
        user_id: Set(1),
        status: Set(code_video::CodeVideoStatus::Finished),
        prompt: Set("test".to_owned()),
        url: Set(Some("https://example.com/v.mp4".to_owned())),
        public: Set(false),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

    // FINISHED -> GENERATING is invalid
    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/internal/code-videos/1",
            Some(api_key),
            Some(json!({"status": "GENERATING"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_CONTENT_STATUS");
}

#[tokio::test]
async fn internal_callback_wrong_api_key_rejected() {
    let app = TestApp::new().await;
    app.create_user_and_login("gen_user4", "password123").await;

    let db = Database::connect(&app.config.database_url)
        .await
        .expect("connect");
    use zhiying_backend::entities::knowledge_video;
    knowledge_video::ActiveModel {
        user_id: Set(1),
        status: Set(knowledge_video::KnowledgeVideoStatus::Queuing),
        prompt: Set("test".to_owned()),
        url: Set(None),
        public: Set(false),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

    // Use code_video api_key on knowledge_video endpoint
    let wrong_key = &app.config.code_video_api_key;
    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/internal/knowledge-videos/1",
            Some(wrong_key),
            Some(json!({"status": "GENERATING"})),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "INVALID_API_KEY");

    // Use completely invalid key
    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/internal/knowledge-videos/1",
            Some("sk-nonexistent"),
            Some(json!({"status": "GENERATING"})),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "INVALID_API_KEY");
}

#[tokio::test]
async fn user_patch_set_public_works() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("gen_user5", "password123").await;

    let db = Database::connect(&app.config.database_url)
        .await
        .expect("connect");
    use zhiying_backend::entities::knowledge_explanation;
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
async fn knowledge_explanation_callback_with_content_and_mindmap() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("gen_user6", "password123").await;
    let api_key = &app.config.knowledge_explanation_api_key;

    let db = Database::connect(&app.config.database_url)
        .await
        .expect("connect");
    use zhiying_backend::entities::knowledge_explanation;
    knowledge_explanation::ActiveModel {
        user_id: Set(1),
        status: Set(knowledge_explanation::KnowledgeExplanationStatus::Queuing),
        prompt: Set("explain interfaces".to_owned()),
        content: Set(None),
        mindmap: Set(None),
        public: Set(false),
        cost: Set(10),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert");

    // QUEUING -> GENERATING
    let (status, _) = app
        .request(
            "PATCH",
            "/api/v1/internal/knowledge-explanations/1",
            Some(api_key),
            Some(json!({"status": "GENERATING"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // GENERATING -> FINISHED with content and mindmap
    let mindmap = r#"{"title":"接口","children":[{"title":"定义","children":[]}]}"#;
    let (status, _) = app
        .request(
            "PATCH",
            "/api/v1/internal/knowledge-explanations/1",
            Some(api_key),
            Some(json!({
                "status": "FINISHED",
                "content": "接口是一种抽象类型...",
                "mindmap": mindmap
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Verify via user GET
    let (status, body) = app
        .request(
            "GET",
            "/api/v1/knowledge-explanations/1",
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["content"], "接口是一种抽象类型...");
    assert_eq!(body["data"]["mindmap"]["title"], "接口");
}
