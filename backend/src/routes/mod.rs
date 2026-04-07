use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};

use crate::ops::{OpError, OpSuccess};

pub mod auth;
pub mod filesystem;
pub mod sharing;

impl IntoResponse for OpSuccess {
    fn into_response(self) -> axum::response::Response {
        match self {
            // Filesystem
            Self::FolderCreated => (StatusCode::CREATED, "The folder has been created").into_response(),
            Self::Folders { folders } => (StatusCode::OK, Json(folders)).into_response(),
            Self::FolderDeleted => StatusCode::NO_CONTENT.into_response(),
            Self::FolderRenamed => StatusCode::NO_CONTENT.into_response(),
            Self::FileUploaded => StatusCode::CREATED.into_response(),
            Self::Files { files } => (StatusCode::OK, Json(files)).into_response(),
            Self::FileData { data, mime_type } => {
                ([(CONTENT_TYPE, mime_type)], data).into_response()
            }
            Self::FileRenamed => StatusCode::NO_CONTENT.into_response(),
            Self::FileDeleted => StatusCode::NO_CONTENT.into_response(),
            Self::Reordered => StatusCode::NO_CONTENT.into_response(),
            Self::EntryMoved => StatusCode::NO_CONTENT.into_response(),
            Self::TrashItems { items } => (StatusCode::OK, Json(items)).into_response(),
            Self::EntryRestored => StatusCode::NO_CONTENT.into_response(),
            Self::TrashEntryDeleted => StatusCode::NO_CONTENT.into_response(),

            // Sharing
            Self::ShareRequestSent => (StatusCode::CREATED, "Share request sent").into_response(),
            Self::ShareRequestCancelled => StatusCode::NO_CONTENT.into_response(),
            Self::ShareRequestAccepted => StatusCode::OK.into_response(),
            Self::ShareRequestDeclined => StatusCode::NO_CONTENT.into_response(),
            Self::PendingRequests { requests } => (StatusCode::OK, Json(requests)).into_response(),
            Self::SentRequests { requests } => (StatusCode::OK, Json(requests)).into_response(),
            Self::PermissionRevoked => StatusCode::NO_CONTENT.into_response(),
            Self::MyGrants { grants } => (StatusCode::OK, Json(grants)).into_response(),
            Self::SharedWithMe { items } => (StatusCode::OK, Json(items)).into_response(),
            Self::SharedFolders { folders } => (StatusCode::OK, Json(folders)).into_response(),
            Self::SharedFiles { files } => (StatusCode::OK, Json(files)).into_response(),
            Self::FileCopied => (StatusCode::CREATED, "File copied to your library").into_response(),

            // Auth — LoggedIn is handled specially in the route handler (cookie)
            Self::LoggedIn { username, .. } => {
                (StatusCode::OK, Json(serde_json::json!({ "username": username }))).into_response()
            }
            Self::SignedOut => (StatusCode::OK, "Signed out successfully").into_response(),
            Self::UsernameChanged => StatusCode::OK.into_response(),
            Self::PasswordChanged => StatusCode::OK.into_response(),
            Self::TrashRetention { days } => {
                (StatusCode::OK, Json(serde_json::json!({ "days": days }))).into_response()
            }
            Self::TrashRetentionSet => StatusCode::OK.into_response(),
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
        }
    }
}
