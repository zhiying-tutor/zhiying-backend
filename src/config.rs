use std::{collections::BTreeMap, env, net::IpAddr};

use crate::error::AppError;

#[derive(Clone, Debug)]
pub struct Config {
    pub host: IpAddr,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_ttl_days: i64,
    pub cors_allow_origin: String,
    pub register_bonus_diamonds: i32,
    pub checkin_reward_sequence: Vec<i32>,
    pub checkin_makeup_gold_cost_per_day: i32,
    pub checkin_makeup_diamond_cost: i32,

    // Content generation costs
    pub knowledge_video_diamond_cost: i32,
    pub code_video_diamond_cost: i32,
    pub interactive_html_gold_cost: i32,
    pub knowledge_explanation_gold_cost: i32,

    // Microservice exchanges (RabbitMQ)
    pub knowledge_video_exchange: String,
    pub code_video_exchange: String,
    pub interactive_html_exchange: String,
    pub knowledge_explanation_exchange: String,

    // Microservice API keys (used by callbacks back to this server)
    pub knowledge_video_api_key: String,
    pub code_video_api_key: String,
    pub interactive_html_api_key: String,
    pub knowledge_explanation_api_key: String,

    // Study subject: total_stages → diamond_cost
    pub study_subject_diamond_costs: BTreeMap<i32, i32>,
    pub pretest_exchange: String,
    pub pretest_api_key: String,
    pub plan_exchange: String,
    pub plan_api_key: String,
    pub quiz_exchange: String,
    pub quiz_api_key: String,
    pub study_quiz_free_limit_per_task: i32,
    pub study_quiz_extra_gold_cost: i32,

    // Recharge
    pub recharge_api_key: String,

    // RabbitMQ
    pub rabbitmq_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let host = env::var("APP_HOST")
            .unwrap_or_else(|_| "0.0.0.0".to_owned())
            .parse()
            .map_err(|_| AppError::internal("APP_HOST is invalid"))?;

        let port = env::var("APP_PORT")
            .unwrap_or_else(|_| "9000".to_owned())
            .parse()
            .map_err(|_| AppError::internal("APP_PORT is invalid"))?;

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://zhiying-backend.db?mode=rwc".to_owned());

        let jwt_secret =
            env::var("JWT_SECRET").unwrap_or_else(|_| "change-me-in-production".to_owned());

        let jwt_ttl_days = env::var("JWT_TTL_DAYS")
            .unwrap_or_else(|_| "30".to_owned())
            .parse()
            .map_err(|_| AppError::internal("JWT_TTL_DAYS is invalid"))?;

        let cors_allow_origin = env::var("CORS_ALLOW_ORIGIN").unwrap_or_else(|_| "*".to_owned());

        let register_bonus_diamonds = env::var("REGISTER_BONUS_DIAMONDS")
            .unwrap_or_else(|_| "80".to_owned())
            .parse()
            .map_err(|_| AppError::internal("REGISTER_BONUS_DIAMONDS is invalid"))?;

        if register_bonus_diamonds < 0 {
            return Err(AppError::internal(
                "REGISTER_BONUS_DIAMONDS must be non-negative",
            ));
        }

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

        let knowledge_video_exchange = env::var("KNOWLEDGE_VIDEO_EXCHANGE")
            .unwrap_or_else(|_| "zhiying.knowledge_video".to_owned());
        let code_video_exchange =
            env::var("CODE_VIDEO_EXCHANGE").unwrap_or_else(|_| "zhiying.code_video".to_owned());
        let interactive_html_exchange = env::var("INTERACTIVE_HTML_EXCHANGE")
            .unwrap_or_else(|_| "zhiying.interactive_html".to_owned());
        let knowledge_explanation_exchange = env::var("KNOWLEDGE_EXPLANATION_EXCHANGE")
            .unwrap_or_else(|_| "zhiying.knowledge_explanation".to_owned());

        let knowledge_video_api_key = env::var("KNOWLEDGE_VIDEO_API_KEY")
            .unwrap_or_else(|_| "sk-knowledge-video-dev".to_owned());
        let code_video_api_key =
            env::var("CODE_VIDEO_API_KEY").unwrap_or_else(|_| "sk-code-video-dev".to_owned());
        let interactive_html_api_key = env::var("INTERACTIVE_HTML_API_KEY")
            .unwrap_or_else(|_| "sk-interactive-html-dev".to_owned());
        let knowledge_explanation_api_key = env::var("KNOWLEDGE_EXPLANATION_API_KEY")
            .unwrap_or_else(|_| "sk-knowledge-explanation-dev".to_owned());

        let study_subject_diamond_costs = parse_study_subject_diamond_costs(
            &env::var("STUDY_SUBJECT_DIAMOND_COSTS")
                .unwrap_or_else(|_| "3:10,7:20,15:40,30:80".to_owned()),
        )?;

        let pretest_exchange =
            env::var("PRETEST_EXCHANGE").unwrap_or_else(|_| "zhiying.pretest".to_owned());
        let pretest_api_key =
            env::var("PRETEST_API_KEY").unwrap_or_else(|_| "sk-pretest-dev".to_owned());

        let plan_exchange = env::var("PLAN_EXCHANGE").unwrap_or_else(|_| "zhiying.plan".to_owned());
        let plan_api_key = env::var("PLAN_API_KEY").unwrap_or_else(|_| "sk-plan-dev".to_owned());

        let quiz_exchange = env::var("QUIZ_EXCHANGE").unwrap_or_else(|_| "zhiying.quiz".to_owned());
        let quiz_api_key = env::var("QUIZ_API_KEY").unwrap_or_else(|_| "sk-quiz-dev".to_owned());

        let study_quiz_free_limit_per_task = env::var("STUDY_QUIZ_FREE_LIMIT_PER_TASK")
            .unwrap_or_else(|_| "3".to_owned())
            .parse()
            .map_err(|_| AppError::internal("STUDY_QUIZ_FREE_LIMIT_PER_TASK is invalid"))?;

        let study_quiz_extra_gold_cost = env::var("STUDY_QUIZ_EXTRA_GOLD_COST")
            .unwrap_or_else(|_| "20".to_owned())
            .parse()
            .map_err(|_| AppError::internal("STUDY_QUIZ_EXTRA_GOLD_COST is invalid"))?;

        let recharge_api_key =
            env::var("RECHARGE_API_KEY").unwrap_or_else(|_| "sk-recharge-dev".to_owned());

        let rabbitmq_url = env::var("RABBITMQ_URL")
            .unwrap_or_else(|_| "amqp://dev:dev@localhost:5672/%2f".to_owned());

        Ok(Self {
            host,
            port,
            database_url,
            jwt_secret,
            jwt_ttl_days,
            cors_allow_origin,
            register_bonus_diamonds,
            checkin_reward_sequence,
            checkin_makeup_gold_cost_per_day,
            checkin_makeup_diamond_cost,
            knowledge_video_diamond_cost,
            code_video_diamond_cost,
            interactive_html_gold_cost,
            knowledge_explanation_gold_cost,
            knowledge_video_exchange,
            code_video_exchange,
            interactive_html_exchange,
            knowledge_explanation_exchange,
            knowledge_video_api_key,
            code_video_api_key,
            interactive_html_api_key,
            knowledge_explanation_api_key,
            study_subject_diamond_costs,
            pretest_exchange,
            pretest_api_key,
            plan_exchange,
            plan_api_key,
            quiz_exchange,
            quiz_api_key,
            study_quiz_free_limit_per_task,
            study_quiz_extra_gold_cost,
            recharge_api_key,
            rabbitmq_url,
        })
    }
}

fn parse_study_subject_diamond_costs(raw: &str) -> Result<BTreeMap<i32, i32>, AppError> {
    let mut map = BTreeMap::new();
    for entry in raw.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (key, value) = entry
            .split_once(':')
            .ok_or_else(|| AppError::internal("STUDY_SUBJECT_DIAMOND_COSTS is invalid"))?;
        let key: i32 = key
            .trim()
            .parse()
            .map_err(|_| AppError::internal("STUDY_SUBJECT_DIAMOND_COSTS is invalid"))?;
        let value: i32 = value
            .trim()
            .parse()
            .map_err(|_| AppError::internal("STUDY_SUBJECT_DIAMOND_COSTS is invalid"))?;
        if key <= 0 || value <= 0 {
            return Err(AppError::internal(
                "STUDY_SUBJECT_DIAMOND_COSTS must contain positive numbers",
            ));
        }
        if map.insert(key, value).is_some() {
            return Err(AppError::internal(
                "STUDY_SUBJECT_DIAMOND_COSTS contains duplicate total_stages",
            ));
        }
    }
    if map.is_empty() {
        return Err(AppError::internal("STUDY_SUBJECT_DIAMOND_COSTS is empty"));
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::parse_study_subject_diamond_costs;

    #[test]
    fn parses_default_value() {
        let map = parse_study_subject_diamond_costs("3:10,7:20,15:40,30:80").unwrap();
        assert_eq!(map.len(), 4);
        assert_eq!(map[&3], 10);
        assert_eq!(map[&30], 80);
        let keys: Vec<_> = map.keys().copied().collect();
        assert_eq!(keys, vec![3, 7, 15, 30]);
    }

    #[test]
    fn parses_custom_value_ignoring_whitespace() {
        let map = parse_study_subject_diamond_costs("  5 : 12 , 9: 24 ").unwrap();
        assert_eq!(map[&5], 12);
        assert_eq!(map[&9], 24);
    }

    #[test]
    fn rejects_duplicate_keys() {
        assert!(parse_study_subject_diamond_costs("3:10,3:20").is_err());
    }

    #[test]
    fn rejects_non_positive() {
        assert!(parse_study_subject_diamond_costs("0:10").is_err());
        assert!(parse_study_subject_diamond_costs("3:0").is_err());
        assert!(parse_study_subject_diamond_costs("-3:10").is_err());
    }

    #[test]
    fn rejects_non_numeric() {
        assert!(parse_study_subject_diamond_costs("a:10").is_err());
        assert!(parse_study_subject_diamond_costs("3:b").is_err());
        assert!(parse_study_subject_diamond_costs("3-10").is_err());
    }

    #[test]
    fn rejects_empty() {
        assert!(parse_study_subject_diamond_costs("").is_err());
        assert!(parse_study_subject_diamond_costs(" , ").is_err());
    }
}
