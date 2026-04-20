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
use chrono::DateTime;

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
    pub user_type: String,
    pub username: String,
    pub session_id: Uuid,
    pub group: Option<(Uuid, String)>
}

impl LoggedInUser {
    fn get_group_id(parts: &mut axum::http::request::Parts) -> Option<Uuid> {
        parts.headers.get("X-Group")?.to_str().ok()?.parse().ok() // take x-group, convert to str if possible and parse to uuid if possible
    }
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
        }

        let row: Row = sqlx::query_as(
            "SELECT u.id, u.user_type, u.username, s.id AS session_id FROM sessions s JOIN users u ON s.user_id = u.id WHERE s.token = $1 AND s.expires_at > NOW()"
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

        let group_id = Self::get_group_id(parts);
        let group_data = match group_id {
            Some(id) => {
                #[derive(sqlx::FromRow)]
                struct GroupData {
                    username: String
                }
                let row: GroupData = sqlx::query_as(
                    "SELECT u.username FROM group_members m 
            JOIN users u ON u.id = m.group_id 
            WHERE m.user_id = $1 AND m.group_id = $2 AND m.state = 'accepted'"
                )
                .bind(&id)
                .fetch_optional(&state.pool)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        e.to_string(),
                    )
                })?
                .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Could not find selected group in users group list".to_string()))?;

                Some((id, row.username))
            },
            None => None,
        };

        Ok(LoggedInUser { user_id: row.id, username: row.username, user_type: row.user_type, session_id: row.session_id, group: group_data })
    }
}

impl LoggedInUser {
    pub fn into_ctx(self) -> Result<OpCtx, OpError> {
        match self.user_type.as_str() {
            "user" => Ok(OpCtx::User { id: self.user_id, username: self.username, group: self.group }),
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
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "group_member_state", rename_all = "snake_case")]
pub enum GroupMemberState {
    PendingInvite,
    Accepted,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct GroupMember {
    pub id: Uuid,

    // Source Group
    pub group_id: Uuid,
    pub group_type: String, // Defaults to 'group'
    pub group_name: String,
    pub group_username: String,

    // Target User
    pub user_id: Uuid,
    pub user_type: String,  // Defaults to 'user'

    // Sender
    pub sender_id: Option<Uuid>,
    pub sender_type: Option<String>, // Defaults to 'user'
    pub sender_username: Option<String>,

    // Metadata
    pub perms: Vec<String>,
    pub state: GroupMemberState,
    pub created_at: DateTime<Utc>,
}
