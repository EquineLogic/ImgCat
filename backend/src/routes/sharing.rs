use crate::AppData;
use crate::ops::models::auth::LoggedInUser;
use crate::ops::models::{
    CopySharedFilePayload, RevokePermissionPayload, SendShareRequestPayload,
    ShareRequestIdPayload, SharedFolderParams,
};
use crate::ops::{OpArgs, OpError, OpSuccess};
use axum::{
    Json,
    extract::{Query, State},
};

pub async fn send_share_request(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<SendShareRequestPayload>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::SendShareRequest {
            filesystem_id: payload.filesystem_id,
            recipient_username: payload.recipient_username,
            access_level: payload.access_level.unwrap_or_else(|| "viewer".into()),
        },
        Some(user.into_ctx()?),
    )
    .await
}

pub async fn cancel_share_request(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<ShareRequestIdPayload>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::CancelShareRequest { id: payload.id },
        Some(user.into_ctx()?),
    )
    .await
}

pub async fn accept_share_request(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<ShareRequestIdPayload>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::AcceptShareRequest { id: payload.id },
        Some(user.into_ctx()?),
    )
    .await
}

pub async fn decline_share_request(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<ShareRequestIdPayload>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::DeclineShareRequest { id: payload.id },
        Some(user.into_ctx()?),
    )
    .await
}

pub async fn list_pending_requests(
    State(app): State<AppData>,
    user: LoggedInUser,
) -> Result<OpSuccess, OpError> {
    app.exec_op(OpArgs::ListPendingRequests, Some(user.into_ctx()?))
        .await
}

pub async fn list_sent_requests(
    State(app): State<AppData>,
    user: LoggedInUser,
) -> Result<OpSuccess, OpError> {
    app.exec_op(OpArgs::ListSentRequests, Some(user.into_ctx()?))
        .await
}

pub async fn revoke_permission(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<RevokePermissionPayload>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::RevokePermission { id: payload.id },
        Some(user.into_ctx()?),
    )
    .await
}

pub async fn list_my_grants(
    State(app): State<AppData>,
    user: LoggedInUser,
) -> Result<OpSuccess, OpError> {
    app.exec_op(OpArgs::ListMyGrants, Some(user.into_ctx()?)).await
}

pub async fn list_shared_with_me(
    State(app): State<AppData>,
    user: LoggedInUser,
) -> Result<OpSuccess, OpError> {
    app.exec_op(OpArgs::ListSharedWithMe, Some(user.into_ctx()?))
        .await
}

pub async fn list_shared_folder(
    State(app): State<AppData>,
    user: LoggedInUser,
    Query(params): Query<SharedFolderParams>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::ListSharedFolder {
            permission_filesystem_id: params.permission_filesystem_id,
            parent_id: params.parent_id,
        },
        Some(user.into_ctx()?),
    )
    .await
}

pub async fn list_shared_files(
    State(app): State<AppData>,
    user: LoggedInUser,
    Query(params): Query<SharedFolderParams>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::ListSharedFiles {
            permission_filesystem_id: params.permission_filesystem_id,
            parent_id: params.parent_id,
        },
        Some(user.into_ctx()?),
    )
    .await
}

pub async fn copy_shared_file(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<CopySharedFilePayload>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::CopySharedFile {
            filesystem_id: payload.filesystem_id,
            parent_id: payload.parent_id,
        },
        Some(user.into_ctx()?),
    )
    .await
}
