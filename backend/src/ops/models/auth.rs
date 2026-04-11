use crate::AppData;
use crate::ops::{OpCtx, OpError};
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum_extra::extract::CookieJar;
use chrono::Utc;
use rand::distr::Alphanumeric;
use rand::{distr::SampleString, rng};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

fn is_valid_password(password: &str) -> bool {
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_symbol = password.chars().any(|c| !c.is_alphanumeric());
    let is_long_enough = password.len() >= 8;
    has_uppercase && has_lowercase && has_digit && has_symbol && is_long_enough
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub name: String,
}

impl RegisterRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.username.len() < 5 {
            return Err("Username must be at least 5 characters".to_string());
        }
        if self.name.len() < 5 {
            return Err("Display name must be at least 5 characters".to_string());
        }
        if self.password.len() < 8 {
            return Err("Password must be at least 8 characters".to_string());
        }
        if !is_valid_password(&self.password) {
            return Err("Password must contain at least one uppercase letter, one lowercase letter, one digit, and one special character".to_string());
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct SignInRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct Session {
    pub user_id: Uuid,
    pub username: String,
    pub token: String,
}

impl Session {
    const TOKEN_LENGTH: usize = 256;
    const EXPIRY: Duration = Duration::from_secs(60 * 60 * 24);

    pub async fn new<'c, E>(executor: E, user_id: Uuid, username: String, underlying_group_member: Option<Uuid>) -> Result<Self, crate::Error>
    where
        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        let token: String = Alphanumeric.sample_string(&mut rng(), Self::TOKEN_LENGTH);

        sqlx::query("INSERT INTO sessions (user_id, token, expires_at, underlying_group_member) VALUES ($1, $2, $3, $4)")
            .bind(user_id)
            .bind(&token)
            .bind(Utc::now() + Self::EXPIRY)
            .bind(underlying_group_member)
            .execute(executor)
            .await
            .map_err(|e| format!("Failed to create session: {e}"))?;

        Ok(Self { user_id, username, token })
    }
}

#[derive(Serialize)]
pub struct LoggedInUser {
    pub user_id: Uuid,
    pub user_type: String,
    pub username: String,
    pub session_id: Uuid,
    pub underlying_group_member: Option<Uuid>
}

impl FromRequestParts<AppData> for LoggedInUser {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppData,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to parse cookies: {e}"),
                )
            })?;

        let session_token = jar
            .get("session_token")
            .map(|c| c.value())
            .ok_or((StatusCode::UNAUTHORIZED, "No session token".to_string()))?;

        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            username: String,
            user_type: String,
            session_id: Uuid,
            underlying_group_member: Option<Uuid>
        }

        let row: Row = sqlx::query_as(
                "SELECT u.id, u.user_type, u.username, s.id AS session_id, s.underlying_group_member FROM sessions s JOIN users u ON s.user_id = u.id WHERE s.token = $1 AND s.expires_at > NOW()"
            )
            .bind(&session_token)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    e.to_string(),
                )
            })?
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Invalid authorization provided".to_string()))?;

        Ok(LoggedInUser { user_id: row.id, username: row.username, user_type: row.user_type, session_id: row.session_id, underlying_group_member: row.underlying_group_member })
    }
}

impl LoggedInUser {
    pub fn into_ctx(&self) -> Result<OpCtx, OpError> {
        match self.user_type.as_str() {
            "user" => Ok(OpCtx::User(self.user_id)),
            "group" => {
                let Some(underlying_group_member) = self.underlying_group_member else {
                    return Err(OpError::Generic("Session is a group session but no underlying_group_member found".into()));
                };
                Ok(OpCtx::Group { id: self.user_id, underlying_group_member })
            },
            _ => Err(OpError::Generic("Invalid user type provided".into()))
        }
    }
}

#[derive(Deserialize)]
pub struct ChangeUsername {
    pub username: String,
}

impl ChangeUsername {
    pub fn validate(&self) -> Result<(), String> {
        if self.username.len() < 5 {
            return Err("Username must be at least 5 characters".to_string());
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct ChangePassword {
    pub curr_password: String,
    pub new_password: String,
}

impl ChangePassword {
    pub fn validate(&self) -> Result<(), String> {
        if !is_valid_password(&self.new_password) {
            return Err("Password must contain at least one uppercase letter, one lowercase letter, one digit, and one special character".to_string());
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct SetTrashRetention {
    pub days: i32,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    Login,
    GroupSession
}