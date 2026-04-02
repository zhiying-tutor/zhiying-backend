use std::{env, net::IpAddr};

use crate::error::AppError;

#[derive(Clone, Debug)]
pub struct Config {
    pub host: IpAddr,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_ttl_days: i64,
    pub cors_allow_origin: String,
    pub checkin_reward_sequence: Vec<i32>,
    pub checkin_makeup_gold_cost_per_day: i32,
    pub checkin_makeup_diamond_cost: i32,

    // Content generation costs
    pub knowledge_video_diamond_cost: i32,
    pub code_video_diamond_cost: i32,
    pub interactive_html_gold_cost: i32,
    pub knowledge_explanation_gold_cost: i32,

    // Microservice URLs
    pub knowledge_video_service_url: String,
    pub code_video_service_url: String,
    pub interactive_html_service_url: String,
    pub knowledge_explanation_service_url: String,

    // Microservice API keys
    pub knowledge_video_api_key: String,
    pub code_video_api_key: String,
    pub interactive_html_api_key: String,
    pub knowledge_explanation_api_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let host = env::var("APP_HOST")
            .unwrap_or_else(|_| "0.0.0.0".to_owned())
            .parse()
            .map_err(|_| AppError::internal("APP_HOST is invalid"))?;

        let port = env::var("APP_PORT")
            .unwrap_or_else(|_| "3000".to_owned())
            .parse()
            .map_err(|_| AppError::internal("APP_PORT is invalid"))?;

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://./zhiying.db?mode=rwc".to_owned());

        let jwt_secret =
            env::var("JWT_SECRET").unwrap_or_else(|_| "change-me-in-production".to_owned());

        let jwt_ttl_days = env::var("JWT_TTL_DAYS")
            .unwrap_or_else(|_| "30".to_owned())
            .parse()
            .map_err(|_| AppError::internal("JWT_TTL_DAYS is invalid"))?;

        let cors_allow_origin = env::var("CORS_ALLOW_ORIGIN").unwrap_or_else(|_| "*".to_owned());

        let checkin_reward_sequence = env::var("CHECKIN_REWARD_SEQUENCE")
            .unwrap_or_else(|_| "1,2,3,4,6,8,10".to_owned())
            .split(',')
            .map(|part| {
                part.trim()
                    .parse::<i32>()
                    .map_err(|_| AppError::internal("CHECKIN_REWARD_SEQUENCE is invalid"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        if checkin_reward_sequence.is_empty() {
            return Err(AppError::internal("CHECKIN_REWARD_SEQUENCE is empty"));
        }

        let checkin_makeup_gold_cost_per_day = env::var("CHECKIN_MAKEUP_GOLD_COST_PER_DAY")
            .unwrap_or_else(|_| "50".to_owned())
            .parse()
            .map_err(|_| AppError::internal("CHECKIN_MAKEUP_GOLD_COST_PER_DAY is invalid"))?;

        let checkin_makeup_diamond_cost = env::var("CHECKIN_MAKEUP_DIAMOND_COST")
            .unwrap_or_else(|_| "1".to_owned())
            .parse()
            .map_err(|_| AppError::internal("CHECKIN_MAKEUP_DIAMOND_COST is invalid"))?;

        let knowledge_video_diamond_cost = env::var("KNOWLEDGE_VIDEO_DIAMOND_COST")
            .unwrap_or_else(|_| "5".to_owned())
            .parse()
            .map_err(|_| AppError::internal("KNOWLEDGE_VIDEO_DIAMOND_COST is invalid"))?;

        let code_video_diamond_cost = env::var("CODE_VIDEO_DIAMOND_COST")
            .unwrap_or_else(|_| "5".to_owned())
            .parse()
            .map_err(|_| AppError::internal("CODE_VIDEO_DIAMOND_COST is invalid"))?;

        let interactive_html_gold_cost = env::var("INTERACTIVE_HTML_GOLD_COST")
            .unwrap_or_else(|_| "10".to_owned())
            .parse()
            .map_err(|_| AppError::internal("INTERACTIVE_HTML_GOLD_COST is invalid"))?;

        let knowledge_explanation_gold_cost = env::var("KNOWLEDGE_EXPLANATION_GOLD_COST")
            .unwrap_or_else(|_| "10".to_owned())
            .parse()
            .map_err(|_| AppError::internal("KNOWLEDGE_EXPLANATION_GOLD_COST is invalid"))?;

        let knowledge_video_service_url = env::var("KNOWLEDGE_VIDEO_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8001".to_owned());
        let code_video_service_url = env::var("CODE_VIDEO_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8002".to_owned());
        let interactive_html_service_url = env::var("INTERACTIVE_HTML_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8003".to_owned());
        let knowledge_explanation_service_url = env::var("KNOWLEDGE_EXPLANATION_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8004".to_owned());

        let knowledge_video_api_key = env::var("KNOWLEDGE_VIDEO_API_KEY")
            .unwrap_or_else(|_| "sk-knowledge-video-dev".to_owned());
        let code_video_api_key =
            env::var("CODE_VIDEO_API_KEY").unwrap_or_else(|_| "sk-code-video-dev".to_owned());
        let interactive_html_api_key = env::var("INTERACTIVE_HTML_API_KEY")
            .unwrap_or_else(|_| "sk-interactive-html-dev".to_owned());
        let knowledge_explanation_api_key = env::var("KNOWLEDGE_EXPLANATION_API_KEY")
            .unwrap_or_else(|_| "sk-knowledge-explanation-dev".to_owned());

        Ok(Self {
            host,
            port,
            database_url,
            jwt_secret,
            jwt_ttl_days,
            cors_allow_origin,
            checkin_reward_sequence,
            checkin_makeup_gold_cost_per_day,
            checkin_makeup_diamond_cost,
            knowledge_video_diamond_cost,
            code_video_diamond_cost,
            interactive_html_gold_cost,
            knowledge_explanation_gold_cost,
            knowledge_video_service_url,
            code_video_service_url,
            interactive_html_service_url,
            knowledge_explanation_service_url,
            knowledge_video_api_key,
            code_video_api_key,
            interactive_html_api_key,
            knowledge_explanation_api_key,
        })
    }
}
