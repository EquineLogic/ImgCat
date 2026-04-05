use axum::response::IntoResponse;
use axum::http::StatusCode;

use crate::ops::OpError;

pub mod auth;
pub mod filesystem;

impl IntoResponse for OpError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Generic(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
            Self::EntityConflict { reason } => (StatusCode::CONFLICT, reason).into_response(),
            Self::UserNotLoggedIn => (StatusCode::UNAUTHORIZED, "You must be logged in to perform this operation").into_response()
        }
    }
}