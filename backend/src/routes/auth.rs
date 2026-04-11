use crate::AppData;
use crate::ops::models::auth::{
    ChangePassword, ChangeUsername, LoggedInUser, RegisterRequest, SetTrashRetention, SignInRequest,
};
use crate::ops::{OpArgs, OpError, OpSuccess};
use axum::{Json, extract::State, response::IntoResponse};

pub async fn register(
    State(app): State<AppData>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, OpError> {
    if let Err(e) = payload.validate() {
        return Err(OpError::ValidationFailed { reason: e });
    }

    app
    .exec_op(
        OpArgs::CreateUser {
            username: payload.username,
            password: payload.password,
            name: payload.name,
        },
        None,
    )
    .await
}

pub async fn sign_in(
    State(app): State<AppData>,
    Json(payload): Json<SignInRequest>,
) -> Result<impl IntoResponse, OpError> {
    app
        .exec_op(
            OpArgs::CreateLoginSession {
                username: payload.username,
                password: payload.password,
            },
            None,
        )
        .await
}

pub async fn check_auth(
    State(_app): State<AppData>,
    user: LoggedInUser,
) -> Json<LoggedInUser> {
    Json(user) // LoggedInUser extractor already verified auth and has the username
}

pub async fn sign_out(
    State(app): State<AppData>,
    user: LoggedInUser,
) -> Result<OpSuccess, OpError> {
    app.exec_op(OpArgs::DeleteSession { id: user.session_id }, Some(user.into_ctx()?)).await
}

pub async fn change_username(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<ChangeUsername>,
) -> Result<OpSuccess, OpError> {
    if let Err(e) = payload.validate() {
        return Err(OpError::ValidationFailed { reason: e });
    }

    app.exec_op(
        OpArgs::ChangeUsername {
            new_username: payload.username,
        },
        Some(user.into_ctx()?),
    )
    .await
}

pub async fn change_password(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<ChangePassword>,
) -> Result<OpSuccess, OpError> {
    if let Err(e) = payload.validate() {
        return Err(OpError::ValidationFailed { reason: e });
    }

    app.exec_op(
        OpArgs::ChangePassword {
            curr_password: payload.curr_password,
            new_password: payload.new_password,
        },
        Some(user.into_ctx()?),
    )
    .await
}

pub async fn get_trash_retention(
    State(app): State<AppData>,
    user: LoggedInUser,
) -> Result<OpSuccess, OpError> {
    app.exec_op(OpArgs::GetTrashRetention, Some(user.into_ctx()?))
        .await
}

pub async fn set_trash_retention(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<SetTrashRetention>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::SetTrashRetention { days: payload.days },
        Some(user.into_ctx()?),
    )
    .await
}
