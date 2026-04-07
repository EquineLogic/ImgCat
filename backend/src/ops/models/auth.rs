use crate::AppData;
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum_extra::extract::CookieJar;
use chrono::Utc;
use rand::distr::Alphanumeric;
use rand::{distr::SampleString, rng};
use serde::{Deserialize, Serialize};
use sqlx::Row;
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

    pub async fn new<'c, E>(executor: E, user_id: Uuid, username: String) -> Result<Self, crate::Error>
    where
        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        let token: String = Alphanumeric.sample_string(&mut rng(), Self::TOKEN_LENGTH);

        sqlx::query("INSERT INTO sessions (user_id, token, expires_at) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(&token)
            .bind(Utc::now() + Self::EXPIRY)
            .execute(executor)
            .await
            .map_err(|e| format!("Failed to create session: {e}"))?;

        Ok(Self { user_id, username, token })
    }
}

#[derive(Serialize)]
pub struct LoggedInUser {
    pub user_id: Uuid,
    pub username: String,
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

        let row = sqlx::query(
                "SELECT u.id, u.username FROM sessions s JOIN users u ON s.user_id = u.id WHERE s.token = $1 AND s.expires_at > NOW()"
            )
            .bind(&session_token)
            .fetch_one(&state.pool)
            .await
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    "Invalid or expired session token".to_string(),
                )
            })?;

        let user_id: Uuid = row.get("id");
        let username: String = row.get("username");

        Ok(LoggedInUser { user_id, username })
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
