use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::FileUrl;

// ── Response types ─────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ShareRequestEntry {
    pub id: Uuid,
    pub filesystem_id: Uuid,
    pub entry_name: String,
    pub entry_type: String,
    pub sender_username: String,
    pub recipient_username: String,
    pub access_level: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub url: Option<FileUrl>,
}

#[derive(Serialize)]
pub struct PermissionEntry {
    pub id: Uuid,
    pub filesystem_id: Uuid,
    pub entry_name: String,
    pub entry_type: String,
    pub grantee_username: String,
    pub granted_by: Option<String>,
    pub access_level: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub url: Option<FileUrl>,
}

#[derive(sqlx::FromRow)]
pub struct ShareRequestRow {
    pub id: Uuid,
    pub filesystem_id: Uuid,
    pub entry_name: String,
    pub entry_type: String,
    pub sender_username: String,
    pub recipient_username: String,
    pub access_level: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub s3_fileid: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct PermissionRow {
    pub id: Uuid,
    pub filesystem_id: Uuid,
    pub entry_name: String,
    pub entry_type: String,
    pub grantee_username: String,
    pub granted_by: Option<String>,
    pub access_level: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub s3_fileid: Option<String>,
}

// ── Request types ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SendShareRequestPayload {
    pub filesystem_id: Uuid,
    pub recipient_username: String,
    pub access_level: Option<String>,
}

#[derive(Deserialize)]
pub struct ShareRequestIdPayload {
    pub id: Uuid,
}

#[derive(Deserialize)]
pub struct RevokePermissionPayload {
    pub id: Uuid,
}

#[derive(Deserialize)]
pub struct SharedFolderParams {
    pub permission_filesystem_id: Uuid,
    pub parent_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct CopySharedFilePayload {
    pub filesystem_id: Uuid,
    pub parent_id: Option<Uuid>,
}
