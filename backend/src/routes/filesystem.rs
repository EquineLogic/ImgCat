// IMPORTANT: every SELECT/UPDATE query against `filesystem` MUST include
// `AND deleted_at IS NULL` unless it is intentionally operating on soft-deleted
// rows (e.g. trash listing or restore). Missing this filter will leak deleted
// items into the UI.

use crate::AppData;
use crate::ops::models::auth::LoggedInUser;
use crate::ops::models::{
    DeleteFile, DeleteFolder, ListParams, MoveRequest, NewFile, NewFolder, RenameRequest,
    ReorderRequest,
};
use crate::ops::{OpArgs, OpError, OpSuccess};
use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
};
use uuid::Uuid;

pub async fn create_folder(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<NewFolder>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::CreateFolder {
            name: payload.name,
            parent_id: payload.parent_id,
        },
        Some(user.user_id),
    )
    .await
}

pub async fn list_folders(
    State(app): State<AppData>,
    user: LoggedInUser,
    Query(params): Query<ListParams>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::ListFolder {
            parent_id: params.parent_id,
        },
        Some(user.user_id),
    )
    .await
}

pub async fn delete_folder(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<DeleteFolder>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(OpArgs::DeleteFolder { id: payload.id }, Some(user.user_id))
        .await
}

pub async fn rename_folder(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<RenameRequest>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::RenameFolder {
            id: payload.id,
            name: payload.name,
        },
        Some(user.user_id),
    )
    .await
}

pub async fn upload_file(
    State(app): State<AppData>,
    user: LoggedInUser,
    multipart: Multipart,
) -> Result<OpSuccess, OpError> {
    let payload = NewFile::from_multipart(multipart)
        .await
        .map_err(|(_, msg)| OpError::BadRequest { reason: msg })?;

    app.exec_op(
        OpArgs::UploadFile {
            data: payload.data,
            name: payload.name,
            mime_type: payload.mime_type,
            parent_id: payload.parent_id,
        },
        Some(user.user_id),
    )
    .await
}

pub async fn list_files(
    State(app): State<AppData>,
    user: LoggedInUser,
    Query(params): Query<ListParams>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::ListFiles {
            parent_id: params.parent_id,
        },
        Some(user.user_id),
    )
    .await
}

pub async fn get_file(
    State(app): State<AppData>,
    user: LoggedInUser,
    Path(id): Path<Uuid>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(OpArgs::GetFile { id }, Some(user.user_id))
        .await
}

pub async fn rename_file(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<RenameRequest>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::RenameFile {
            id: payload.id,
            name: payload.name,
        },
        Some(user.user_id),
    )
    .await
}

pub async fn delete_file(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<DeleteFolder>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(OpArgs::DeleteFile { id: payload.id }, Some(user.user_id))
        .await
}

pub async fn reorder(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<ReorderRequest>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(OpArgs::Reorder { ids: payload.ids }, Some(user.user_id))
        .await
}

pub async fn move_entry(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<MoveRequest>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::MoveEntry {
            id: payload.id,
            parent_id: payload.parent_id,
        },
        Some(user.user_id),
    )
    .await
}

pub async fn list_trash(
    State(app): State<AppData>,
    user: LoggedInUser,
) -> Result<OpSuccess, OpError> {
    app.exec_op(OpArgs::ListTrash, Some(user.user_id)).await
}

pub async fn restore_entry(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<DeleteFolder>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(OpArgs::RestoreEntry { id: payload.id }, Some(user.user_id))
        .await
}

pub async fn delete_trash_entry(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<DeleteFile>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::DeleteTrashEntry { id: payload.id },
        Some(user.user_id),
    )
    .await
}
