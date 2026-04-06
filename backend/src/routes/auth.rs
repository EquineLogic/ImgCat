use crate::AppData;
use crate::ops::models::auth::{
    ChangePassword, ChangeUsername, LoggedInUser, RegisterRequest, SetTrashRetention, SignInRequest,
};
use crate::ops::{OpArgs, OpError, OpSuccess};
use axum::http::{HeaderValue, StatusCode, header::SET_COOKIE};
use axum::{Json, extract::State, response::IntoResponse};

// Cookie helper
fn session_cookie(token: &str) -> String {
    format!(
        "session_token={}; HttpOnly; SameSite=Lax; Path=/; Max-Age=604800",
        token
    )
}

pub async fn register(
    State(app): State<AppData>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, OpError> {
    if let Err(e) = payload.validate() {
        return Err(OpError::ValidationFailed { reason: e });
    }

    let result = app
        .exec_op(
            OpArgs::Register {
                username: payload.username,
                password: payload.password,
                name: payload.name,
            },
            None,
        )
        .await?;

    match result {
        OpSuccess::LoggedIn { username, token } => {
            let cookie = session_cookie(&token);
            let mut response = (
                StatusCode::OK,
                Json(serde_json::json!({ "username": username })),
            )
                .into_response();
            response.headers_mut().insert(
                SET_COOKIE,
                HeaderValue::from_str(&cookie)
                    .map_err(|e| OpError::Generic(format!("Failed to create cookie: {e}").into()))?,
            );
            Ok(response)
        }
        other => Ok(other.into_response()),
    }
}

pub async fn sign_in(
    State(app): State<AppData>,
    Json(payload): Json<SignInRequest>,
) -> Result<impl IntoResponse, OpError> {
    let result = app
        .exec_op(
            OpArgs::SignIn {
                username: payload.username,
                password: payload.password,
            },
            None,
        )
        .await?;

    match result {
        OpSuccess::LoggedIn { username, token } => {
            let cookie = session_cookie(&token);
            let mut response = (
                StatusCode::OK,
                Json(serde_json::json!({ "username": username })),
            )
                .into_response();
            response.headers_mut().insert(
                SET_COOKIE,
                HeaderValue::from_str(&cookie)
                    .map_err(|e| OpError::Generic(format!("Failed to create cookie: {e}").into()))?,
            );
            Ok(response)
        }
        other => Ok(other.into_response()),
    }
}

pub async fn check_auth(
    State(app): State<AppData>,
    user: LoggedInUser,
) -> Result<OpSuccess, OpError> {
    app.exec_op(OpArgs::CheckAuth, Some(user.username)).await
}

pub async fn sign_out(
    State(app): State<AppData>,
    user: LoggedInUser,
) -> Result<impl IntoResponse, OpError> {
    app.exec_op(OpArgs::SignOut, Some(user.username)).await?;

    let cookie = "session_token=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0";
    let mut response = (StatusCode::OK, "Signed out successfully").into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(cookie)
            .map_err(|e| OpError::Generic(format!("Failed to create cookie: {e}").into()))?,
    );
    Ok(response)
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
        Some(user.username),
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
        Some(user.username),
    )
    .await
}

pub async fn get_trash_retention(
    State(app): State<AppData>,
    user: LoggedInUser,
) -> Result<OpSuccess, OpError> {
    app.exec_op(OpArgs::GetTrashRetention, Some(user.username))
        .await
}

pub async fn set_trash_retention(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<SetTrashRetention>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::SetTrashRetention { days: payload.days },
        Some(user.username),
    )
    .await
}
