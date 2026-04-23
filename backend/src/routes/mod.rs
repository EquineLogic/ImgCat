use axum::http::HeaderValue;
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};
use reqwest::header::SET_COOKIE;

use crate::ops::models::SessionType;
use crate::ops::{OpError, OpSuccess};

pub mod auth;
pub mod filesystem;
pub mod ws;

impl IntoResponse for OpSuccess {
    fn into_response(self) -> axum::response::Response {
        let mut sess_token = None;
        match &self {
            Self::CreatedSession { token, token_type, .. } if *token_type == SessionType::Login => sess_token = Some(token.to_string()),
            _ => {}
        }
        let mut resp = (StatusCode::OK, Json(self)).into_response();
        if let Some(token) = sess_token {
            // Cookie helper
            let cookie = format!(
                "session_token={}; HttpOnly; SameSite=Lax; Path=/; Max-Age=604800",
                token
            );

            if let Ok(hv) = HeaderValue::from_str(&cookie) {
                resp.headers_mut().insert(SET_COOKIE, hv);
            }
        }

        resp
    }
}

impl IntoResponse for OpError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Generic(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
            Self::EntityConflict { reason } => (StatusCode::CONFLICT, reason).into_response(),
            Self::UserNotLoggedIn => (
                StatusCode::UNAUTHORIZED,
                "You must be logged in to perform this operation",
            )
                .into_response(),
            Self::EntityNotFound { reason } => (StatusCode::NOT_FOUND, reason).into_response(),
            Self::ValidationFailed { reason } => (StatusCode::BAD_REQUEST, reason).into_response(),
            Self::BadRequest { reason } => (StatusCode::BAD_REQUEST, reason).into_response(),
            Self::TooManyItems => (StatusCode::BAD_REQUEST, "Too many items to process at once").into_response(),
            Self::Unauthorized { reason } => (StatusCode::UNAUTHORIZED, reason).into_response(),
            Self::UserOnlyOp => (StatusCode::BAD_REQUEST, "Operation can only be performed by users (not groups!)").into_response(),
            Self::GroupOnlyOp => (StatusCode::BAD_REQUEST, "Operation can only be performed by groups (not users!)").into_response(),
            Self::OpNeedsPerms { perm } => (StatusCode::FORBIDDEN, format!("This operation needs {perm}!")).into_response(),
        }
    }
}
