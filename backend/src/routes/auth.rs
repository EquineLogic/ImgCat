use crate::AppData;
use crate::models::auth::{
    ChangePassword, ChangeUsername, LoggedInUser, RegisterRequest, Session, SetTrashRetention,
    SignInRequest, TrashRetention,
};
use argon2::{
    Argon2, PasswordHasher, PasswordVerifier,
    password_hash::{PasswordHash, SaltString, rand_core::OsRng},
};
use axum::http::{HeaderValue, StatusCode, header::SET_COOKIE};
use axum::{Json, extract::State, response::IntoResponse};
use sqlx;
use sqlx::Row;

pub async fn register(
    State(app): State<AppData>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if let Err(e) = payload.validate() {
        return Err((StatusCode::BAD_REQUEST, e));
    }

    let hashed_password = salt_and_hash_password(&payload.password);

    let mut tx = app.pool.begin().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    let res = sqlx::query("INSERT INTO users (username, name, password) VALUES ($1, $2, $3) ON CONFLICT (username) DO NOTHING")
        .bind(&payload.username)
        .bind(&payload.name)
        .bind(&hashed_password)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {e}")))?;

    if res.rows_affected() == 0 {
        return Err((StatusCode::CONFLICT, "Username already exists".to_string()));
    }

    let session = Session::new(&mut *tx, payload.username)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create session: {e}"),
            )
        })?;

    tx.commit().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    // Make a cookie for sending session token
    // TODO: Add Secure back once not in local development
    let cookie = format!(
        "session_token={}; HttpOnly; SameSite=Lax; Path=/; Max-Age=604800",
        session.token
    );

    // Create response and insert the header
    let mut response = (
        StatusCode::OK,
        Json(LoggedInUser {
            username: session.username,
        }),
    )
        .into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create cookie: {e}"),
            )
        })?,
    );

    Ok(response)
}

pub fn salt_and_hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

pub async fn sign_in(
    State(app): State<AppData>,
    Json(payload): Json<SignInRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut tx = app.pool.begin().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    // get password hash for the given username
    let row = sqlx::query("SELECT password FROM users WHERE username = $1")
        .bind(&payload.username)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "Invalid username or password".to_string(),
            )
        })?;

    let hashed_password: String = row.get("password");

    // verify password
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(&hashed_password).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to parse password hash".to_string(),
        )
    })?;

    argon2
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "Invalid username or password".to_string(),
            )
        })?;

    // create session

    let session = Session::new(&mut *tx, payload.username)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create session: {e}"),
            )
        })?;

    tx.commit().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    // Make a cookie for sending session token
    let cookie = format!(
        "session_token={}; HttpOnly; SameSite=Lax; Path=/; Max-Age=604800",
        session.token
    );

    // Create response and insert the header
    let mut response = (
        StatusCode::OK,
        Json(LoggedInUser {
            username: session.username,
        }),
    )
        .into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create cookie: {e}"),
            )
        })?,
    );

    Ok(response)
}

pub async fn sign_out(
    State(app): State<AppData>,
    user: LoggedInUser,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let row = sqlx::query("DELETE FROM sessions WHERE username = $1")
        .bind(&user.username)
        .execute(&app.pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {e}"),
            )
        })?;

    if row.rows_affected() == 0 {
        return Err((StatusCode::UNAUTHORIZED, "No active session".to_string()));
    }

    // Clear the cookie by setting an expired cookie
    let cookie = "session_token=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0";

    let mut response = (StatusCode::OK, "Signed out successfully").into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(cookie).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create cookie: {e}"),
            )
        })?,
    );

    Ok(response)
}

pub async fn check_auth(user: LoggedInUser) -> Result<impl IntoResponse, (StatusCode, String)> {
    Ok(Json(user))
}

pub async fn change_username(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<ChangeUsername>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if let Err(e) = payload.validate() {
        return Err((StatusCode::BAD_REQUEST, e));
    }

    sqlx::query(
        "UPDATE users SET username = $1
              WHERE username = $2",
    )
    .bind(payload.username)
    .bind(user.username)
    .execute(&app.pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            (StatusCode::CONFLICT, "Username already exists".to_string())
        }
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        ),
    })?;

    Ok(StatusCode::OK)
}

pub async fn change_password(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<ChangePassword>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if let Err(e) = payload.validate() {
        return Err((StatusCode::BAD_REQUEST, e));
    }

    // get password hash for the given username
    let row = sqlx::query("SELECT password FROM users WHERE username = $1")
        .bind(&user.username)
        .fetch_one(&app.pool)
        .await
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "Invalid username or password".to_string(),
            )
        })?;

    let hashed_password: String = row.get("password");

    // verify password
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(&hashed_password).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to parse password hash".to_string(),
        )
    })?;

    argon2
        .verify_password(payload.curr_password.as_bytes(), &parsed_hash)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Incorrect password".to_string()))?;

    // now set new password
    let hashed_password = salt_and_hash_password(&payload.new_password);

    sqlx::query(
        "UPDATE users SET password = $1
              WHERE username = $2",
    )
    .bind(&hashed_password)
    .bind(&user.username)
    .execute(&app.pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    Ok(StatusCode::OK)
}

pub async fn get_trash_retention(
    State(app): State<AppData>,
    user: LoggedInUser,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let row = sqlx::query("SELECT trash_retention_days FROM users WHERE username = $1")
        .bind(&user.username)
        .fetch_one(&app.pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {e}"),
            )
        })?;

    let retention_days: i32 = row.get("trash_retention_days");

    Ok(Json(TrashRetention {
        days: retention_days,
    }))
}

pub async fn set_trash_retention(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<SetTrashRetention>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    const VALID_DAYS: [i32; 6] = [0, 7, 14, 30, 60, 90];

    if !VALID_DAYS.contains(&payload.days) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid retention period".to_string(),
        ));
    }

    sqlx::query(
        "UPDATE users SET trash_retention_days = $1
              WHERE username = $2",
    )
    .bind(payload.days)
    .bind(&user.username)
    .execute(&app.pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    Ok(StatusCode::OK)
}
