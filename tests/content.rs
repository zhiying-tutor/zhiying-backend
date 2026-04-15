mod common;

use axum::http::StatusCode;
use serde_json::json;
use zhiying_backend::entities::{
    code_video, interactive_html, knowledge_explanation, knowledge_video,
};

use common::TestApp;

#[tokio::test]
async fn internal_callback_updates_knowledge_video_status() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("gen_user1", "password123").await;
    let api_key = &app.config.knowledge_video_api_key;

    app.update_user_state("gen_user1", None, 0, 0, 100, 50)
        .await;
    app.insert_knowledge_video(1, knowledge_video::KnowledgeVideoStatus::Queuing)
        .await;

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

    app.update_user_state("gen_user2", None, 0, 0, 90, 10).await;
    app.insert_interactive_html(1, interactive_html::InteractiveHtmlStatus::Queuing)
        .await;

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

    app.insert_code_video(1, code_video::CodeVideoStatus::Finished)
        .await;

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

    app.insert_knowledge_video(1, knowledge_video::KnowledgeVideoStatus::Queuing)
        .await;

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
async fn knowledge_explanation_callback_with_content_and_mindmap() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("gen_user6", "password123").await;
    let api_key = &app.config.knowledge_explanation_api_key;

    app.insert_knowledge_explanation(
        1,
        knowledge_explanation::KnowledgeExplanationStatus::Queuing,
        10,
    )
    .await;

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

#[tokio::test]
async fn content_get_nonexistent_knowledge_video_returns_404() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let (status, body) = app
        .request("GET", "/api/v1/knowledge-videos/999", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "CONTENT_NOT_FOUND");
}

#[tokio::test]
async fn content_get_other_users_code_video_returns_404() {
    let app = TestApp::new().await;
    let _token_alice = app.create_user_and_login("alice", "password123").await;
    let token_bob = app.create_user_and_login("bob", "password123").await;

    app.insert_code_video(1, code_video::CodeVideoStatus::Finished)
        .await;

    // Bob cannot see Alice's code video
    let (status, body) = app
        .request("GET", "/api/v1/code-videos/1", Some(&token_bob), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "CONTENT_NOT_FOUND");
}

#[tokio::test]
async fn content_get_nonexistent_interactive_html_returns_404() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let (status, body) = app
        .request("GET", "/api/v1/interactive-htmls/999", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "CONTENT_NOT_FOUND");
}

#[tokio::test]
async fn content_get_nonexistent_knowledge_explanation_returns_404() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let (status, body) = app
        .request(
            "GET",
            "/api/v1/knowledge-explanations/999",
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "CONTENT_NOT_FOUND");
}

#[tokio::test]
async fn content_retry_non_failed_knowledge_video_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    app.update_user_state("alice", None, 0, 0, 100, 50).await;

    app.insert_knowledge_video(1, knowledge_video::KnowledgeVideoStatus::Finished)
        .await;

    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/knowledge-videos/1",
            Some(&token),
            Some(json!({"retry": true})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_CONTENT_STATUS");
}

#[tokio::test]
async fn content_retry_non_failed_code_video_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    app.insert_code_video(1, code_video::CodeVideoStatus::Generating)
        .await;

    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/code-videos/1",
            Some(&token),
            Some(json!({"retry": true})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_CONTENT_STATUS");
}

#[tokio::test]
async fn code_video_callback_full_lifecycle() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let api_key = &app.config.code_video_api_key;

    app.update_user_state("alice", None, 0, 0, 100, 50).await;
    app.insert_code_video(1, code_video::CodeVideoStatus::Queuing)
        .await;

    // QUEUING -> GENERATING
    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/internal/code-videos/1",
            Some(api_key),
            Some(json!({"status": "GENERATING"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "GENERATING");

    // GENERATING -> FINISHED
    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/internal/code-videos/1",
            Some(api_key),
            Some(json!({"status": "FINISHED", "url": "https://cdn.example.com/cv.mp4"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "FINISHED");

    // User GET confirms
    let (status, body) = app
        .request("GET", "/api/v1/code-videos/1", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["url"], "https://cdn.example.com/cv.mp4");
}

#[tokio::test]
async fn code_video_callback_failed_refunds_diamond() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let api_key = &app.config.code_video_api_key;

    // User has 45 diamonds (simulating 5 already deducted for creation)
    app.update_user_state("alice", None, 0, 0, 100, 45).await;
    app.insert_code_video(1, code_video::CodeVideoStatus::Queuing)
        .await;

    // QUEUING -> GENERATING
    let (status, _) = app
        .request(
            "PATCH",
            "/api/v1/internal/code-videos/1",
            Some(api_key),
            Some(json!({"status": "GENERATING"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // GENERATING -> FAILED
    let (status, _) = app
        .request(
            "PATCH",
            "/api/v1/internal/code-videos/1",
            Some(api_key),
            Some(json!({"status": "FAILED"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Diamond refunded: 45 + 5 = 50
    let (_, me_body) = app.request("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(me_body["data"]["diamond"], 50);
}

#[tokio::test]
async fn interactive_html_callback_full_lifecycle() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let api_key = &app.config.interactive_html_api_key;

    app.update_user_state("alice", None, 0, 0, 100, 50).await;
    app.insert_interactive_html(1, interactive_html::InteractiveHtmlStatus::Queuing)
        .await;

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

    // GENERATING -> FINISHED
    let (status, _) = app
        .request(
            "PATCH",
            "/api/v1/internal/interactive-htmls/1",
            Some(api_key),
            Some(json!({"status": "FINISHED", "url": "https://cdn.example.com/ih.html"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // User GET confirms
    let (status, body) = app
        .request("GET", "/api/v1/interactive-htmls/1", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["url"], "https://cdn.example.com/ih.html");
}

#[tokio::test]
async fn knowledge_explanation_failed_refunds_gold() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    let api_key = &app.config.knowledge_explanation_api_key;

    // User has 90 gold (simulating 10 already deducted)
    app.update_user_state("alice", None, 0, 0, 90, 10).await;
    app.insert_knowledge_explanation(
        1,
        knowledge_explanation::KnowledgeExplanationStatus::Queuing,
        10,
    )
    .await;

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

    // GENERATING -> FAILED
    let (status, _) = app
        .request(
            "PATCH",
            "/api/v1/internal/knowledge-explanations/1",
            Some(api_key),
            Some(json!({"status": "FAILED"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Gold refunded: 90 + 10 = 100
    let (_, me_body) = app.request("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(me_body["data"]["gold"], 100);
}

#[tokio::test]
async fn content_retry_failed_kv_insufficient_diamonds_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    // 0 diamonds
    app.update_user_state("alice", None, 0, 0, 100, 0).await;

    app.insert_knowledge_video(1, knowledge_video::KnowledgeVideoStatus::Failed)
        .await;

    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/knowledge-videos/1",
            Some(&token),
            Some(json!({"retry": true})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INSUFFICIENT_DIAMONDS");
}

#[tokio::test]
async fn content_retry_failed_ih_insufficient_gold_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    // 0 gold
    app.update_user_state("alice", None, 0, 0, 0, 50).await;

    app.insert_interactive_html(1, interactive_html::InteractiveHtmlStatus::Failed)
        .await;

    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/interactive-htmls/1",
            Some(&token),
            Some(json!({"retry": true})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INSUFFICIENT_GOLD");
}

#[tokio::test]
async fn content_retry_failed_ke_insufficient_gold_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;
    // 0 gold
    app.update_user_state("alice", None, 0, 0, 0, 50).await;

    app.insert_knowledge_explanation(
        1,
        knowledge_explanation::KnowledgeExplanationStatus::Failed,
        10,
    )
    .await;

    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/knowledge-explanations/1",
            Some(&token),
            Some(json!({"retry": true})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INSUFFICIENT_GOLD");
}

#[tokio::test]
async fn content_retry_non_failed_interactive_html_returns_400() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    app.insert_interactive_html(1, interactive_html::InteractiveHtmlStatus::Queuing)
        .await;

    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/interactive-htmls/1",
            Some(&token),
            Some(json!({"retry": true})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_CONTENT_STATUS");
}

#[tokio::test]
async fn content_patch_nonexistent_knowledge_video_returns_404() {
    let app = TestApp::new().await;
    let token = app.create_user_and_login("alice", "password123").await;

    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/knowledge-videos/999",
            Some(&token),
            Some(json!({"public": true})),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "CONTENT_NOT_FOUND");
}

#[tokio::test]
async fn content_set_public_non_owner_returns_404() {
    let app = TestApp::new().await;
    app.create_user_and_login("alice", "password123").await;
    let token_bob = app.create_user_and_login("bob", "password123").await;

    app.insert_knowledge_explanation(
        1, // alice
        knowledge_explanation::KnowledgeExplanationStatus::Finished,
        10,
    )
    .await;

    // Bob tries to set Alice's content public
    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/knowledge-explanations/1",
            Some(&token_bob),
            Some(json!({"public": true})),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "CONTENT_NOT_FOUND");
}

#[tokio::test]
async fn internal_callback_cross_service_key_on_ih_rejected() {
    let app = TestApp::new().await;
    app.create_user_and_login("alice", "password123").await;

    app.insert_interactive_html(1, interactive_html::InteractiveHtmlStatus::Queuing)
        .await;

    // Use knowledge_video key on interactive_html endpoint
    let wrong_key = &app.config.knowledge_video_api_key;
    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/internal/interactive-htmls/1",
            Some(wrong_key),
            Some(json!({"status": "GENERATING"})),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "INVALID_API_KEY");
}

#[tokio::test]
async fn internal_callback_nonexistent_resource_returns_404() {
    let app = TestApp::new().await;
    let api_key = &app.config.knowledge_video_api_key;

    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/internal/knowledge-videos/999",
            Some(api_key),
            Some(json!({"status": "GENERATING"})),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "CONTENT_NOT_FOUND");
}
