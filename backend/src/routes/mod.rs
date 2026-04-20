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
        match self {
            // Filesystem
            Self::Folders { folders } => (StatusCode::OK, Json(folders)).into_response(),
            Self::Files { files } => (StatusCode::OK, Json(files)).into_response(),
            Self::TrashItems { items } => (StatusCode::OK, Json(items)).into_response(),

            // Sharing
            Self::CreatedSession { username, token, token_type } => {
                let mut resp = (StatusCode::OK, Json(serde_json::json!({ "username": username, "token": token, "token_type": token_type }))).into_response();
                if token_type == SessionType::Login {
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
            Self::DeletedSession { token_type, .. } => {
                let mut resp = (StatusCode::OK, "Signed out successfully").into_response();
                if token_type == SessionType::Login {
                    let cookie = "session_token=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0";
                    if let Ok(hv) = HeaderValue::from_str(&cookie) {
                        resp.headers_mut().insert(
                            SET_COOKIE,
                            hv,
                        );
                    }
                }
                resp
            },
            Self::TrashRetention { days } => {
                (StatusCode::OK, Json(serde_json::json!({ "days": days }))).into_response()
            }
            Self::GroupMembers { group_members } => (StatusCode::OK, Json(group_members)).into_response(),
            Self::CreatedGroup { group_id } => (StatusCode::OK, Json(serde_json::json!({ "group_id": group_id }))).into_response(),
            Self::Ok => StatusCode::NO_CONTENT.into_response(),
        }
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
