use std::fmt::{Display, Formatter};

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::DbErr;
use serde::Serialize;
use thiserror::Error;
use tracing::error;
use validator::ValidationErrors;

#[derive(Debug, Clone, Copy)]
pub enum BusinessError {
    MissingAuthorizationHeader,
    InvalidAuthorizationHeader,
    InvalidOrExpiredToken,
    InvalidApiKey,
    UsernameAlreadyExists,
    InvalidCredentials,
    UserNotFound,
    ContentNotFound,
    InvalidContentStatus,
    AlreadyCheckedInToday,
    InsufficientGold,
    InsufficientDiamonds,
    ServiceUnavailable,
    FeatureNotImplemented,
}

impl BusinessError {
    pub fn code(self) -> &'static str {
        match self {
            Self::MissingAuthorizationHeader => "MISSING_AUTHORIZATION_HEADER",
            Self::InvalidAuthorizationHeader => "INVALID_AUTHORIZATION_HEADER",
            Self::InvalidOrExpiredToken => "INVALID_OR_EXPIRED_TOKEN",
            Self::InvalidApiKey => "INVALID_API_KEY",
            Self::UsernameAlreadyExists => "USERNAME_ALREADY_EXISTS",
            Self::InvalidCredentials => "INVALID_CREDENTIALS",
            Self::UserNotFound => "USER_NOT_FOUND",
            Self::ContentNotFound => "CONTENT_NOT_FOUND",
            Self::InvalidContentStatus => "INVALID_CONTENT_STATUS",
            Self::AlreadyCheckedInToday => "ALREADY_CHECKED_IN_TODAY",
            Self::InsufficientGold => "INSUFFICIENT_GOLD",
            Self::InsufficientDiamonds => "INSUFFICIENT_DIAMONDS",
            Self::ServiceUnavailable => "SERVICE_UNAVAILABLE",
            Self::FeatureNotImplemented => "FEATURE_NOT_IMPLEMENTED",
        }
    }

    pub fn message_zh(self) -> &'static str {
        match self {
            Self::MissingAuthorizationHeader => "缺少认证信息",
            Self::InvalidAuthorizationHeader => "认证信息格式不正确",
            Self::InvalidOrExpiredToken => "登录状态无效或已过期",
            Self::InvalidApiKey => "无效的服务密钥",
            Self::UsernameAlreadyExists => "用户名已存在",
            Self::InvalidCredentials => "用户名或密码错误",
            Self::UserNotFound => "用户不存在",
            Self::ContentNotFound => "内容不存在",
            Self::InvalidContentStatus => "当前状态不允许此操作",
            Self::AlreadyCheckedInToday => "今天已经签到过了",
            Self::InsufficientGold => "金币不足",
            Self::InsufficientDiamonds => "钻石不足",
            Self::ServiceUnavailable => "生成服务暂时不可用，请稍后再试",
            Self::FeatureNotImplemented => "该功能暂未实现",
        }
    }

    pub fn status_code(self) -> StatusCode {
        match self {
            Self::MissingAuthorizationHeader
            | Self::InvalidAuthorizationHeader
            | Self::InvalidOrExpiredToken
            | Self::InvalidApiKey
            | Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::UserNotFound | Self::ContentNotFound => StatusCode::NOT_FOUND,
            Self::UsernameAlreadyExists => StatusCode::CONFLICT,
            Self::FeatureNotImplemented => StatusCode::NOT_IMPLEMENTED,
            Self::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::AlreadyCheckedInToday
            | Self::InsufficientGold
            | Self::InsufficientDiamonds
            | Self::InvalidContentStatus => StatusCode::BAD_REQUEST,
        }
    }
}

impl Display for BusinessError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Business(BusinessError),
    #[error("VALIDATION_FAILED")]
    ValidationFailed,
    #[error("{context}")]
    Internal { context: String },
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    success: bool,
    code: String,
    message: String,
}

impl AppError {
    pub fn business(error: BusinessError) -> Self {
        Self::Business(error)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            context: message.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Business(error) => (
                error.status_code(),
                error.code().to_owned(),
                error.message_zh().to_owned(),
            ),
            Self::ValidationFailed => (
                StatusCode::BAD_REQUEST,
                "VALIDATION_FAILED".to_owned(),
                "请求参数不合法".to_owned(),
            ),
            Self::Internal { context } => {
                error!(error = %context, "internal application error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_SERVER_ERROR".to_owned(),
                    "服务器开小差了，请稍后再试".to_owned(),
                )
            }
            Self::Database(err) => {
                error!(error = %err, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_SERVER_ERROR".to_owned(),
                    "服务器开小差了，请稍后再试".to_owned(),
                )
            }
        };

        let body = ErrorBody {
            success: false,
            code,
            message,
        };

        (status, Json(body)).into_response()
    }
}

impl From<ValidationErrors> for AppError {
    fn from(_: ValidationErrors) -> Self {
        Self::ValidationFailed
    }
}
