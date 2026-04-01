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
