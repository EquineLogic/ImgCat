pub mod models;

use crate::AppData;
use argon2::{
    Argon2, PasswordHasher, PasswordVerifier,
    password_hash::{PasswordHash, SaltString, rand_core::OsRng},
};
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use models::*;
use sqlx::Row;
use std::error::Error;
use uuid::Uuid;

// ── Inputs ──────────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "op")]
pub enum OpArgs {
    // Filesystem
    CreateFolder { name: String, parent_id: Option<Uuid> },
    ListFolder { parent_id: Option<Uuid> },
    DeleteFolder { id: Uuid },
    RenameFolder { id: Uuid, name: String },
    UploadFile { data: Vec<u8>, name: String, mime_type: String, parent_id: Option<Uuid> },
    ListFiles { parent_id: Option<Uuid> },
    GetFile { id: Uuid },
    RenameFile { id: Uuid, name: String },
    DeleteFile { id: Uuid },
    Reorder { ids: Vec<Uuid> },
    MoveEntry { id: Uuid, parent_id: Option<Uuid> },
    ListTrash,
    RestoreEntry { id: Uuid },
    DeleteTrashEntry { id: Uuid },

    // Sharing
    SendShareRequest { filesystem_id: Uuid, recipient_username: String, access_level: String },
    CancelShareRequest { id: Uuid },
    AcceptShareRequest { id: Uuid },
    DeclineShareRequest { id: Uuid },
    ListPendingRequests,
    ListSentRequests,
    RevokePermission { id: Uuid },
    ListMyGrants,
    ListSharedWithMe,
    ListSharedFolder { permission_filesystem_id: Uuid, parent_id: Option<Uuid> },
    ListSharedFiles { permission_filesystem_id: Uuid, parent_id: Option<Uuid> },
    GetSharedFile { id: Uuid },
    CopySharedFile { filesystem_id: Uuid, parent_id: Option<Uuid> },

    // Auth
    Register { username: String, password: String, name: String },
    SignIn { username: String, password: String },
    CheckAuth,
    SignOut,
    ChangeUsername { new_username: String },
    ChangePassword { curr_password: String, new_password: String },
    GetTrashRetention,
    SetTrashRetention { days: i32 },
}

// ── Outputs ─────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
#[serde(tag = "op")]
pub enum OpSuccess {
    // Filesystem
    FolderCreated,
    Folders { folders: Vec<Folder> },
    FolderDeleted,
    FolderRenamed,
    FileUploaded,
    Files { files: Vec<FileEntry> },
    #[serde(skip)]
    FileData { data: Vec<u8>, mime_type: String },
    FileRenamed,
    FileDeleted,
    Reordered,
    EntryMoved,
    TrashItems { items: Vec<TrashEntry> },
    EntryRestored,
    TrashEntryDeleted,

    // Sharing
    ShareRequestSent,
    ShareRequestCancelled,
    ShareRequestAccepted,
    ShareRequestDeclined,
    PendingRequests { requests: Vec<ShareRequestEntry> },
    SentRequests { requests: Vec<ShareRequestEntry> },
    PermissionRevoked,
    MyGrants { grants: Vec<PermissionEntry> },
    SharedWithMe { items: Vec<PermissionEntry> },
    SharedFolders { folders: Vec<Folder> },
    SharedFiles { files: Vec<FileEntry> },
    // GetSharedFile reuses FileData
    FileCopied,

    // Auth
    LoggedIn { username: String, token: String },
    AuthChecked { username: String },
    SignedOut,
    UsernameChanged,
    PasswordChanged,
    TrashRetention { days: i32 },
    TrashRetentionSet,
}

// ── Errors ──────────────────────────────────────────────────────────────

pub enum OpError {
    Generic(crate::Error),
    EntityConflict { reason: &'static str },
    EntityNotFound { reason: &'static str },
    UserNotLoggedIn,
    ValidationFailed { reason: String },
    BadRequest { reason: String },
    TooManyItems,
    Unauthorized { reason: String },
}

impl<T: Error + Send + Sync + 'static> From<T> for OpError {
    fn from(value: T) -> Self {
        Self::Generic(value.into())
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn salt_and_hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

// ── Execution ───────────────────────────────────────────────────────────

impl AppData {
    pub async fn exec_op(
        &self,
        op: OpArgs,
        username: Option<String>,
    ) -> Result<OpSuccess, OpError> {
        match op {
            // ─── Filesystem ─────────────────────────────────────────

            OpArgs::CreateFolder { name, parent_id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let row = sqlx::query(
                    "INSERT INTO filesystem (name, type, owner_username, parent_id, sort_order)
                    VALUES ($1, 'folder', $2, $3,
                        COALESCE((SELECT MAX(sort_order) FROM filesystem WHERE parent_id IS NOT DISTINCT FROM $3 AND owner_username = $2 AND deleted_at IS NULL), 0) + 1
                    ) ON CONFLICT (parent_id, name) WHERE deleted_at IS NULL DO NOTHING"
                )
                .bind(name)
                .bind(username)
                .bind(parent_id)
                .execute(&self.pool)
                .await?;

                if row.rows_affected() == 0 {
                    return Err(OpError::EntityConflict {
                        reason: "A folder with that name already exists",
                    });
                }

                Ok(OpSuccess::FolderCreated)
            }

            OpArgs::ListFolder { parent_id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let rows: Vec<Folder> = sqlx::query_as(
                    "SELECT id, name FROM filesystem WHERE owner_username = $1 AND type = 'folder' AND parent_id IS NOT DISTINCT FROM $2 AND deleted_at IS NULL ORDER BY sort_order"
                )
                .bind(username)
                .bind(parent_id)
                .fetch_all(&self.pool)
                .await?;

                Ok(OpSuccess::Folders { folders: rows })
            }

            OpArgs::DeleteFolder { id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let res = sqlx::query(
                    "UPDATE filesystem SET deleted_at = NOW(), updated_at = NOW()
                            WHERE owner_username = $2
                            AND deleted_at IS NULL
                            AND path <@ (
                                SELECT path FROM filesystem
                                WHERE id = $1 AND owner_username = $2 AND type = 'folder' AND deleted_at IS NULL
                            )",
                )
                .bind(id)
                .bind(&username)
                .execute(&self.pool)
                .await?;

                if res.rows_affected() == 0 {
                    return Err(OpError::EntityNotFound {
                        reason: "Folder not found",
                    });
                }

                Ok(OpSuccess::FolderDeleted)
            }

            OpArgs::RenameFolder { id, name } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let res = sqlx::query(
                    "UPDATE filesystem SET name = $1, updated_at = NOW() WHERE id = $2 AND owner_username = $3 AND type = 'folder' AND deleted_at IS NULL",
                )
                .bind(&name)
                .bind(id)
                .bind(&username)
                .execute(&self.pool)
                .await?;

                if res.rows_affected() == 0 {
                    return Err(OpError::EntityNotFound {
                        reason: "Folder not found",
                    });
                }

                Ok(OpSuccess::FolderRenamed)
            }

            OpArgs::UploadFile { data, name, mime_type, parent_id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let s3_key = Uuid::new_v4().to_string();

                self.s3
                    .put_object()
                    .bucket(&self.bucket)
                    .key(&s3_key)
                    .body(data.clone().into())
                    .content_type(&mime_type)
                    .send()
                    .await
                    .map_err(|e| OpError::Generic(format!("S3 upload error: {e:?}").into()))?;

                let mut tx = self.pool.begin().await?;

                let f_row = sqlx::query(
                    "INSERT INTO files (owner_username, s3_fileid, size_bytes, mime_type) VALUES ($1, $2, $3, $4) RETURNING id"
                )
                .bind(&username)
                .bind(&s3_key)
                .bind(data.len() as i64)
                .bind(&mime_type)
                .fetch_one(&mut *tx)
                .await?;

                let f_id: Uuid = f_row.get("id");

                let fs_row = sqlx::query(
                    "INSERT INTO filesystem (name, type, owner_username, parent_id, file_id, sort_order)
                     VALUES ($1, 'file_link', $2, $3, $4,
                        COALESCE((SELECT MAX(sort_order) FROM filesystem WHERE parent_id IS NOT DISTINCT FROM $3 AND owner_username = $2 AND deleted_at IS NULL), 0) + 1
                     )"
                )
                .bind(&name)
                .bind(&username)
                .bind(parent_id)
                .bind(f_id)
                .execute(&mut *tx)
                .await?;

                if fs_row.rows_affected() == 0 {
                    return Err(OpError::EntityConflict {
                        reason: "A file with that name already exists",
                    });
                }

                tx.commit().await?;
                Ok(OpSuccess::FileUploaded)
            }

            OpArgs::ListFiles { parent_id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let rows: Vec<FileRow> = sqlx::query_as(
                    "SELECT fs.id, fs.name, f.size_bytes, f.mime_type, f.s3_fileid
                          FROM filesystem fs
                          JOIN files f ON fs.file_id = f.id
                          WHERE fs.owner_username = $1 AND fs.type = 'file_link'
                          AND fs.parent_id IS NOT DISTINCT FROM $2
                          AND fs.deleted_at IS NULL
                          ORDER BY fs.sort_order",
                )
                .bind(&username)
                .bind(parent_id)
                .fetch_all(&self.pool)
                .await?;

                let mut files: Vec<FileEntry> = Vec::with_capacity(rows.len());
                for row in rows {
                    let furl = FileUrl::new(self, row.s3_fileid).await
                        .map_err(|e| OpError::Generic(e))?;
                    files.push(FileEntry {
                        id: row.id,
                        name: row.name,
                        mime_type: row.mime_type,
                        size_bytes: row.size_bytes,
                        url: furl,
                    });
                }

                Ok(OpSuccess::Files { files })
            }

            OpArgs::GetFile { id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let row = sqlx::query(
                    "SELECT f.s3_fileid, f.mime_type
                          FROM filesystem fs
                          JOIN files f on fs.file_id = f.id
                          WHERE fs.id = $1 AND fs.owner_username = $2 AND fs.type = 'file_link'",
                )
                .bind(&id)
                .bind(&username)
                .fetch_one(&self.pool)
                .await?;

                let s3_fileid: String = row.get("s3_fileid");
                let mime_type: String = row.get("mime_type");

                let obj = self
                    .s3
                    .get_object()
                    .bucket(&self.bucket)
                    .key(s3_fileid)
                    .send()
                    .await
                    .map_err(|e| OpError::Generic(format!("S3 error: {e:?}").into()))?;

                let data = obj.body.collect().await
                    .map_err(|e| OpError::Generic(format!("S3 error: {e:?}").into()))?;

                Ok(OpSuccess::FileData {
                    data: data.into_bytes().to_vec(),
                    mime_type,
                })
            }

            OpArgs::RenameFile { id, name } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let res = sqlx::query(
                    "UPDATE filesystem SET name = $1, updated_at = NOW()
                     WHERE id = $2 AND owner_username = $3 AND type = 'file_link' AND deleted_at IS NULL",
                )
                .bind(&name)
                .bind(id)
                .bind(&username)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    if let sqlx::Error::Database(ref db_err) = e {
                        if db_err.is_unique_violation() {
                            return OpError::EntityConflict {
                                reason: "A file with that name already exists",
                            };
                        }
                    }
                    OpError::Generic(e.into())
                })?;

                if res.rows_affected() == 0 {
                    return Err(OpError::EntityNotFound {
                        reason: "File not found",
                    });
                }

                Ok(OpSuccess::FileRenamed)
            }

            OpArgs::DeleteFile { id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let res = sqlx::query(
                    "UPDATE filesystem SET deleted_at = NOW(), updated_at = NOW()
                     WHERE id = $1 AND owner_username = $2 AND type = 'file_link' AND deleted_at IS NULL",
                )
                .bind(id)
                .bind(&username)
                .execute(&self.pool)
                .await?;

                if res.rows_affected() == 0 {
                    return Err(OpError::EntityNotFound {
                        reason: "File not found",
                    });
                }

                Ok(OpSuccess::FileDeleted)
            }

            OpArgs::Reorder { ids } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let mut tx = self.pool.begin().await?;

                for (i, id) in ids.iter().enumerate() {
                    sqlx::query(
                        "UPDATE filesystem SET sort_order = $1 WHERE id = $2 AND owner_username = $3 AND deleted_at IS NULL"
                    )
                    .bind((i + 1) as i32)
                    .bind(id)
                    .bind(&username)
                    .execute(&mut *tx)
                    .await?;
                }

                tx.commit().await?;
                Ok(OpSuccess::Reordered)
            }

            OpArgs::MoveEntry { id, parent_id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let res = sqlx::query(
                    "UPDATE filesystem
                     SET parent_id = $1,
                         sort_order = COALESCE((SELECT MAX(sort_order) FROM filesystem WHERE parent_id IS NOT DISTINCT FROM $1 AND owner_username = $3 AND deleted_at IS NULL), 0) + 1,
                         updated_at = NOW()
                     WHERE id = $2 AND owner_username = $3 AND deleted_at IS NULL
                       AND id <> $1
                       AND ($1 IS NULL OR EXISTS (
                           SELECT 1 FROM filesystem p
                           WHERE p.id = $1
                             AND p.owner_username = $3
                             AND p.type = 'folder'
                             AND p.deleted_at IS NULL
                             AND NOT (p.path <@ (SELECT path FROM filesystem WHERE id = $2 AND deleted_at IS NULL))
                       ))",
                )
                .bind(parent_id)
                .bind(id)
                .bind(&username)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    if let sqlx::Error::Database(ref db_err) = e {
                        if db_err.is_unique_violation() {
                            return OpError::EntityConflict {
                                reason: "An item with that name already exists in the target folder",
                            };
                        }
                    }
                    OpError::Generic(e.into())
                })?;

                if res.rows_affected() == 0 {
                    return Err(OpError::EntityNotFound {
                        reason: "Entry or target not found",
                    });
                }

                Ok(OpSuccess::EntryMoved)
            }

            OpArgs::ListTrash => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let rows: Vec<TrashRow> = sqlx::query_as(
                    "SELECT fs.id, fs.name, fs.type::text AS kind, fs.deleted_at, f.mime_type, f.s3_fileid
                       FROM filesystem fs
                       LEFT JOIN files f ON fs.file_id = f.id
                      WHERE fs.owner_username = $1
                        AND fs.deleted_at IS NOT NULL
                        AND NOT EXISTS (
                            SELECT 1 FROM filesystem anc
                            WHERE anc.owner_username = $1
                              AND anc.deleted_at IS NOT NULL
                              AND anc.id <> fs.id
                              AND fs.path <@ anc.path
                        )
                      ORDER BY fs.deleted_at DESC, fs.name",
                )
                .bind(&username)
                .fetch_all(&self.pool)
                .await?;

                let mut items: Vec<TrashEntry> = Vec::with_capacity(rows.len());
                for r in rows {
                    let furl = match r.s3_fileid {
                        Some(s3_fileid) => Some(FileUrl::new(self, s3_fileid).await
                            .map_err(|e| OpError::Generic(e))?),
                        None => None,
                    };

                    items.push(TrashEntry {
                        id: r.id,
                        name: r.name,
                        kind: r.kind,
                        mime_type: r.mime_type,
                        deleted_at: r.deleted_at,
                        url: furl,
                    });
                }

                Ok(OpSuccess::TrashItems { items })
            }

            OpArgs::RestoreEntry { id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let res = sqlx::query(
                    "UPDATE filesystem SET deleted_at = NULL, updated_at = NOW()
                     WHERE owner_username = $2
                       AND deleted_at IS NOT NULL
                       AND path <@ (SELECT path FROM filesystem WHERE id = $1 AND owner_username = $2 AND deleted_at IS NOT NULL)
                       AND deleted_at = (SELECT deleted_at FROM filesystem WHERE id = $1 AND owner_username = $2)",
                )
                .bind(id)
                .bind(&username)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    if let sqlx::Error::Database(ref db_err) = e {
                        if db_err.is_unique_violation() {
                            return OpError::EntityConflict {
                                reason: "A non-deleted item with that name already exists in the target folder. Rename it first.",
                            };
                        }
                    }
                    OpError::Generic(e.into())
                })?;

                if res.rows_affected() == 0 {
                    return Err(OpError::EntityNotFound {
                        reason: "Trash item not found",
                    });
                }

                Ok(OpSuccess::EntryRestored)
            }

            OpArgs::DeleteTrashEntry { id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let mut tx = self.pool.begin().await?;

                let file_rows = sqlx::query(
                    "SELECT fs.file_id, f.s3_fileid FROM filesystem fs
                       JOIN files f ON fs.file_id = f.id
                       WHERE fs.owner_username = $1
                         AND fs.deleted_at IS NOT NULL
                         AND fs.path <@ (SELECT path FROM filesystem WHERE id = $2 AND owner_username = $1 AND deleted_at IS NOT NULL)
                         AND fs.type = 'file_link'",
                )
                .bind(&username)
                .bind(id)
                .fetch_all(&mut *tx)
                .await?;

                let s3_keys: Vec<String> = file_rows.iter().map(|r| r.get("s3_fileid")).collect();
                let file_ids: Vec<Uuid> = file_rows.iter().map(|r| r.get("file_id")).collect();

                if s3_keys.len() > 1000 {
                    return Err(OpError::TooManyItems);
                }

                let fs_res = sqlx::query(
                    "DELETE FROM filesystem WHERE owner_username = $1
                       AND deleted_at IS NOT NULL
                       AND path <@ (SELECT path FROM filesystem WHERE id = $2 AND owner_username = $1 AND deleted_at IS NOT NULL)",
                )
                .bind(&username)
                .bind(id)
                .execute(&mut *tx)
                .await?;

                if fs_res.rows_affected() == 0 {
                    return Err(OpError::EntityNotFound {
                        reason: "Trash item not found",
                    });
                }

                if !file_ids.is_empty() {
                    sqlx::query("DELETE FROM files WHERE owner_username = $1 AND id = ANY($2)")
                        .bind(&username)
                        .bind(&file_ids)
                        .execute(&mut *tx)
                        .await?;
                }

                tx.commit().await?;

                // Best-effort S3 cleanup
                if !s3_keys.is_empty() {
                    let objects: Vec<ObjectIdentifier> = s3_keys
                        .into_iter()
                        .filter_map(|k| ObjectIdentifier::builder().key(k).build().ok())
                        .collect();

                    if let Ok(delete) = Delete::builder().set_objects(Some(objects)).build() {
                        match self
                            .s3
                            .delete_objects()
                            .bucket(&self.bucket)
                            .delete(delete)
                            .send()
                            .await
                        {
                            Ok(resp) => {
                                if !resp.errors().is_empty() {
                                    log::warn!("S3 partial failures: {:?}", resp.errors());
                                }
                            }
                            Err(e) => log::warn!("S3 delete_objects failed: {e:?}"),
                        }
                    }
                }

                Ok(OpSuccess::TrashEntryDeleted)
            }

            // ─── Sharing ────────────────────────────────────────────

            OpArgs::SendShareRequest { filesystem_id, recipient_username, access_level } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                if username == recipient_username {
                    return Err(OpError::ValidationFailed {
                        reason: "Cannot share with yourself".into(),
                    });
                }

                // Verify caller owns the entry
                let entry_exists = sqlx::query(
                    "SELECT 1 FROM filesystem WHERE id = $1 AND owner_username = $2 AND deleted_at IS NULL"
                )
                .bind(filesystem_id)
                .bind(&username)
                .fetch_optional(&self.pool)
                .await?;

                if entry_exists.is_none() {
                    return Err(OpError::EntityNotFound { reason: "Entry not found" });
                }

                // Verify recipient exists
                let recipient_exists = sqlx::query("SELECT 1 FROM users WHERE username = $1")
                    .bind(&recipient_username)
                    .fetch_optional(&self.pool)
                    .await?;

                if recipient_exists.is_none() {
                    return Err(OpError::EntityNotFound { reason: "User not found" });
                }

                // Validate access_level
                if !["viewer", "editor"].contains(&access_level.as_str()) {
                    return Err(OpError::ValidationFailed {
                        reason: "Invalid access level".into(),
                    });
                }

                let res = sqlx::query(
                    "INSERT INTO share_requests (filesystem_id, sender_username, recipient_username, access_level)
                     VALUES ($1, $2, $3, $4::access_level)
                     ON CONFLICT (filesystem_id, recipient_username) DO NOTHING"
                )
                .bind(filesystem_id)
                .bind(&username)
                .bind(&recipient_username)
                .bind(&access_level)
                .execute(&self.pool)
                .await?;

                if res.rows_affected() == 0 {
                    return Err(OpError::EntityConflict {
                        reason: "A share request already exists for this item and user",
                    });
                }

                Ok(OpSuccess::ShareRequestSent)
            }

            OpArgs::CancelShareRequest { id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let res = sqlx::query(
                    "DELETE FROM share_requests WHERE id = $1 AND sender_username = $2"
                )
                .bind(id)
                .bind(&username)
                .execute(&self.pool)
                .await?;

                if res.rows_affected() == 0 {
                    return Err(OpError::EntityNotFound { reason: "Share request not found" });
                }

                Ok(OpSuccess::ShareRequestCancelled)
            }

            OpArgs::AcceptShareRequest { id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let mut tx = self.pool.begin().await?;

                // Fetch and delete the request (must be addressed to caller)
                let row = sqlx::query(
                    "DELETE FROM share_requests WHERE id = $1 AND recipient_username = $2
                     RETURNING filesystem_id, sender_username, access_level::text"
                )
                .bind(id)
                .bind(&username)
                .fetch_optional(&mut *tx)
                .await?;

                let Some(row) = row else {
                    return Err(OpError::EntityNotFound { reason: "Share request not found" });
                };

                let filesystem_id: Uuid = row.get("filesystem_id");
                let sender: String = row.get("sender_username");
                let access_level: String = row.get("access_level");

                // Insert permission
                sqlx::query(
                    "INSERT INTO permissions (filesystem_id, grantee_username, access_level, granted_by)
                     VALUES ($1, $2, $3::access_level, $4)
                     ON CONFLICT (filesystem_id, grantee_username) DO UPDATE SET access_level = $3::access_level"
                )
                .bind(filesystem_id)
                .bind(&username)
                .bind(&access_level)
                .bind(&sender)
                .execute(&mut *tx)
                .await?;

                tx.commit().await?;
                Ok(OpSuccess::ShareRequestAccepted)
            }

            OpArgs::DeclineShareRequest { id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let res = sqlx::query(
                    "DELETE FROM share_requests WHERE id = $1 AND recipient_username = $2"
                )
                .bind(id)
                .bind(&username)
                .execute(&self.pool)
                .await?;

                if res.rows_affected() == 0 {
                    return Err(OpError::EntityNotFound { reason: "Share request not found" });
                }

                Ok(OpSuccess::ShareRequestDeclined)
            }

            OpArgs::ListPendingRequests => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let rows: Vec<ShareRequestRow> = sqlx::query_as(
                    "SELECT sr.id, sr.filesystem_id, fs.name AS entry_name, fs.type::text AS entry_type,
                            sr.sender_username, sr.recipient_username, sr.access_level::text AS access_level,
                            sr.created_at, f.s3_fileid
                     FROM share_requests sr
                     JOIN filesystem fs ON sr.filesystem_id = fs.id
                     LEFT JOIN files f ON fs.file_id = f.id
                     WHERE sr.recipient_username = $1 AND fs.deleted_at IS NULL
                     ORDER BY sr.created_at DESC"
                )
                .bind(&username)
                .fetch_all(&self.pool)
                .await?;

                let mut requests = Vec::with_capacity(rows.len());
                for r in rows {
                    let url = match r.s3_fileid {
                        Some(s3id) => Some(FileUrl::new(self, s3id).await.map_err(OpError::Generic)?),
                        None => None,
                    };
                    requests.push(ShareRequestEntry {
                        id: r.id, filesystem_id: r.filesystem_id,
                        entry_name: r.entry_name, entry_type: r.entry_type,
                        sender_username: r.sender_username, recipient_username: r.recipient_username,
                        access_level: r.access_level, created_at: r.created_at, url,
                    });
                }

                Ok(OpSuccess::PendingRequests { requests })
            }

            OpArgs::ListSentRequests => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let rows: Vec<ShareRequestRow> = sqlx::query_as(
                    "SELECT sr.id, sr.filesystem_id, fs.name AS entry_name, fs.type::text AS entry_type,
                            sr.sender_username, sr.recipient_username, sr.access_level::text AS access_level,
                            sr.created_at, f.s3_fileid
                     FROM share_requests sr
                     JOIN filesystem fs ON sr.filesystem_id = fs.id
                     LEFT JOIN files f ON fs.file_id = f.id
                     WHERE sr.sender_username = $1 AND fs.deleted_at IS NULL
                     ORDER BY sr.created_at DESC"
                )
                .bind(&username)
                .fetch_all(&self.pool)
                .await?;

                let mut requests = Vec::with_capacity(rows.len());
                for r in rows {
                    let url = match r.s3_fileid {
                        Some(s3id) => Some(FileUrl::new(self, s3id).await.map_err(OpError::Generic)?),
                        None => None,
                    };
                    requests.push(ShareRequestEntry {
                        id: r.id, filesystem_id: r.filesystem_id,
                        entry_name: r.entry_name, entry_type: r.entry_type,
                        sender_username: r.sender_username, recipient_username: r.recipient_username,
                        access_level: r.access_level, created_at: r.created_at, url,
                    });
                }

                Ok(OpSuccess::SentRequests { requests })
            }

            OpArgs::RevokePermission { id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let res = sqlx::query(
                    "DELETE FROM permissions WHERE id = $1 AND granted_by = $2"
                )
                .bind(id)
                .bind(&username)
                .execute(&self.pool)
                .await?;

                if res.rows_affected() == 0 {
                    return Err(OpError::EntityNotFound { reason: "Permission not found" });
                }

                Ok(OpSuccess::PermissionRevoked)
            }

            OpArgs::ListMyGrants => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let rows: Vec<PermissionRow> = sqlx::query_as(
                    "SELECT p.id, p.filesystem_id, fs.name AS entry_name, fs.type::text AS entry_type,
                            p.grantee_username, p.granted_by, p.access_level::text AS access_level,
                            p.created_at, f.s3_fileid
                     FROM permissions p
                     JOIN filesystem fs ON p.filesystem_id = fs.id
                     LEFT JOIN files f ON fs.file_id = f.id
                     WHERE p.granted_by = $1 AND fs.deleted_at IS NULL
                     ORDER BY p.created_at DESC"
                )
                .bind(&username)
                .fetch_all(&self.pool)
                .await?;

                let mut grants = Vec::with_capacity(rows.len());
                for r in rows {
                    let url = match r.s3_fileid {
                        Some(s3id) => Some(FileUrl::new(self, s3id).await.map_err(OpError::Generic)?),
                        None => None,
                    };
                    grants.push(PermissionEntry {
                        id: r.id, filesystem_id: r.filesystem_id,
                        entry_name: r.entry_name, entry_type: r.entry_type,
                        grantee_username: r.grantee_username, granted_by: r.granted_by,
                        access_level: r.access_level, created_at: r.created_at, url,
                    });
                }

                Ok(OpSuccess::MyGrants { grants })
            }

            OpArgs::ListSharedWithMe => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let rows: Vec<PermissionRow> = sqlx::query_as(
                    "SELECT p.id, p.filesystem_id, fs.name AS entry_name, fs.type::text AS entry_type,
                            p.grantee_username, p.granted_by, p.access_level::text AS access_level,
                            p.created_at, f.s3_fileid
                     FROM permissions p
                     JOIN filesystem fs ON p.filesystem_id = fs.id
                     LEFT JOIN files f ON fs.file_id = f.id
                     WHERE p.grantee_username = $1 AND fs.deleted_at IS NULL
                     ORDER BY p.created_at DESC"
                )
                .bind(&username)
                .fetch_all(&self.pool)
                .await?;

                let mut items = Vec::with_capacity(rows.len());
                for r in rows {
                    let url = match r.s3_fileid {
                        Some(s3id) => Some(FileUrl::new(self, s3id).await.map_err(OpError::Generic)?),
                        None => None,
                    };
                    items.push(PermissionEntry {
                        id: r.id, filesystem_id: r.filesystem_id,
                        entry_name: r.entry_name, entry_type: r.entry_type,
                        grantee_username: r.grantee_username, granted_by: r.granted_by,
                        access_level: r.access_level, created_at: r.created_at, url,
                    });
                }

                Ok(OpSuccess::SharedWithMe { items })
            }

            OpArgs::ListSharedFolder { permission_filesystem_id, parent_id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                // Verify permission exists and get the shared entry's path
                let perm_row = sqlx::query(
                    "SELECT fs.path::text FROM permissions p
                     JOIN filesystem fs ON p.filesystem_id = fs.id
                     WHERE p.filesystem_id = $1 AND p.grantee_username = $2 AND fs.deleted_at IS NULL"
                )
                .bind(permission_filesystem_id)
                .bind(&username)
                .fetch_optional(&self.pool)
                .await?;

                let Some(perm_row) = perm_row else {
                    return Err(OpError::Unauthorized { reason: "No permission for this item".into() });
                };

                let shared_path: String = perm_row.get("path");

                // The parent to list: if None, use the shared folder itself
                let listing_parent = parent_id.unwrap_or(permission_filesystem_id);

                let rows: Vec<Folder> = sqlx::query_as(
                    "SELECT fs.id, fs.name FROM filesystem fs
                     WHERE fs.parent_id = $1 AND fs.type = 'folder' AND fs.deleted_at IS NULL
                       AND fs.path <@ $2::ltree
                     ORDER BY fs.sort_order"
                )
                .bind(listing_parent)
                .bind(&shared_path)
                .fetch_all(&self.pool)
                .await?;

                Ok(OpSuccess::SharedFolders { folders: rows })
            }

            OpArgs::ListSharedFiles { permission_filesystem_id, parent_id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                // Verify permission exists and get the shared entry's path
                let perm_row = sqlx::query(
                    "SELECT fs.path::text FROM permissions p
                     JOIN filesystem fs ON p.filesystem_id = fs.id
                     WHERE p.filesystem_id = $1 AND p.grantee_username = $2 AND fs.deleted_at IS NULL"
                )
                .bind(permission_filesystem_id)
                .bind(&username)
                .fetch_optional(&self.pool)
                .await?;

                let Some(perm_row) = perm_row else {
                    return Err(OpError::Unauthorized { reason: "No permission for this item".into() });
                };

                let shared_path: String = perm_row.get("path");

                let listing_parent = parent_id.unwrap_or(permission_filesystem_id);

                let rows: Vec<FileRow> = sqlx::query_as(
                    "SELECT fs.id, fs.name, f.size_bytes, f.mime_type, f.s3_fileid
                     FROM filesystem fs
                     JOIN files f ON fs.file_id = f.id
                     WHERE fs.parent_id = $1 AND fs.type = 'file_link' AND fs.deleted_at IS NULL
                       AND fs.path <@ $2::ltree
                     ORDER BY fs.sort_order"
                )
                .bind(listing_parent)
                .bind(&shared_path)
                .fetch_all(&self.pool)
                .await?;

                let mut files = Vec::with_capacity(rows.len());
                for row in rows {
                    let furl = FileUrl::new(self, row.s3_fileid).await.map_err(OpError::Generic)?;
                    files.push(FileEntry {
                        id: row.id, name: row.name, mime_type: row.mime_type,
                        size_bytes: row.size_bytes, url: furl,
                    });
                }

                Ok(OpSuccess::SharedFiles { files })
            }

            OpArgs::GetSharedFile { id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let row = sqlx::query(
                    "SELECT f.s3_fileid, f.mime_type
                     FROM filesystem fs
                     JOIN files f ON fs.file_id = f.id
                     WHERE fs.id = $1 AND fs.type = 'file_link' AND fs.deleted_at IS NULL
                       AND EXISTS (
                           SELECT 1 FROM permissions p
                           JOIN filesystem fs_shared ON p.filesystem_id = fs_shared.id
                           WHERE p.grantee_username = $2
                             AND fs_shared.deleted_at IS NULL
                             AND fs.path <@ fs_shared.path
                       )"
                )
                .bind(id)
                .bind(&username)
                .fetch_optional(&self.pool)
                .await?;

                let Some(row) = row else {
                    return Err(OpError::EntityNotFound { reason: "File not found or no permission" });
                };

                let s3_fileid: String = row.get("s3_fileid");
                let mime_type: String = row.get("mime_type");

                let obj = self.s3
                    .get_object()
                    .bucket(&self.bucket)
                    .key(s3_fileid)
                    .send()
                    .await
                    .map_err(|e| OpError::Generic(format!("S3 error: {e:?}").into()))?;

                let data = obj.body.collect().await
                    .map_err(|e| OpError::Generic(format!("S3 error: {e:?}").into()))?;

                Ok(OpSuccess::FileData {
                    data: data.into_bytes().to_vec(),
                    mime_type,
                })
            }

            OpArgs::CopySharedFile { filesystem_id, parent_id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                // Verify the file is shared with the caller
                let row = sqlx::query(
                    "SELECT fs.name, f.s3_fileid, f.size_bytes, f.mime_type
                     FROM filesystem fs
                     JOIN files f ON fs.file_id = f.id
                     WHERE fs.id = $1 AND fs.type = 'file_link' AND fs.deleted_at IS NULL
                       AND EXISTS (
                           SELECT 1 FROM permissions p
                           JOIN filesystem fs_shared ON p.filesystem_id = fs_shared.id
                           WHERE p.grantee_username = $2
                             AND fs_shared.deleted_at IS NULL
                             AND fs.path <@ fs_shared.path
                       )"
                )
                .bind(filesystem_id)
                .bind(&username)
                .fetch_optional(&self.pool)
                .await?;

                let Some(row) = row else {
                    return Err(OpError::EntityNotFound { reason: "File not found or no permission" });
                };

                let name: String = row.get("name");
                let s3_fileid: String = row.get("s3_fileid");
                let size_bytes: i64 = row.get("size_bytes");
                let mime_type: String = row.get("mime_type");

                // Create a new files row pointing to the same S3 object
                let mut tx = self.pool.begin().await?;

                let f_row = sqlx::query(
                    "INSERT INTO files (owner_username, s3_fileid, size_bytes, mime_type) VALUES ($1, $2, $3, $4) RETURNING id"
                )
                .bind(&username)
                .bind(&s3_fileid)
                .bind(size_bytes)
                .bind(&mime_type)
                .fetch_one(&mut *tx)
                .await?;

                let f_id: Uuid = f_row.get("id");

                sqlx::query(
                    "INSERT INTO filesystem (name, type, owner_username, parent_id, file_id, sort_order)
                     VALUES ($1, 'file_link', $2, $3, $4,
                        COALESCE((SELECT MAX(sort_order) FROM filesystem WHERE parent_id IS NOT DISTINCT FROM $3 AND owner_username = $2 AND deleted_at IS NULL), 0) + 1
                     )"
                )
                .bind(&name)
                .bind(&username)
                .bind(parent_id)
                .bind(f_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    if let sqlx::Error::Database(ref db_err) = e {
                        if db_err.is_unique_violation() {
                            return OpError::EntityConflict {
                                reason: "A file with that name already exists in the target folder",
                            };
                        }
                    }
                    OpError::Generic(e.into())
                })?;

                tx.commit().await?;
                Ok(OpSuccess::FileCopied)
            }

            // ─── Auth ───────────────────────────────────────────────

            OpArgs::Register { username, password, name } => {
                let hashed_password = salt_and_hash_password(&password);

                let mut tx = self.pool.begin().await?;

                let res = sqlx::query(
                    "INSERT INTO users (username, name, password) VALUES ($1, $2, $3) ON CONFLICT (username) DO NOTHING"
                )
                .bind(&username)
                .bind(&name)
                .bind(&hashed_password)
                .execute(&mut *tx)
                .await?;

                if res.rows_affected() == 0 {
                    return Err(OpError::EntityConflict {
                        reason: "Username already exists",
                    });
                }

                let session = models::auth::Session::new(&mut *tx, username).await
                    .map_err(|e| OpError::Generic(e))?;

                tx.commit().await?;

                Ok(OpSuccess::LoggedIn {
                    username: session.username,
                    token: session.token,
                })
            }

            OpArgs::SignIn { username, password } => {
                let mut tx = self.pool.begin().await?;

                let row = sqlx::query("SELECT password FROM users WHERE username = $1")
                    .bind(&username)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|_| OpError::Unauthorized {
                        reason: "Invalid username or password".into(),
                    })?;

                let hashed_password: String = row.get("password");

                let argon2 = Argon2::default();
                let parsed_hash = PasswordHash::new(&hashed_password)
                    .map_err(|e| OpError::Generic(format!("Failed to parse password hash: {e}").into()))?;

                argon2
                    .verify_password(password.as_bytes(), &parsed_hash)
                    .map_err(|_| OpError::Unauthorized {
                        reason: "Invalid username or password".into(),
                    })?;

                let session = models::auth::Session::new(&mut *tx, username).await
                    .map_err(|e| OpError::Generic(e))?;

                tx.commit().await?;

                Ok(OpSuccess::LoggedIn {
                    username: session.username,
                    token: session.token,
                })
            }

            OpArgs::CheckAuth => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                Ok(OpSuccess::AuthChecked { username })
            }

            OpArgs::SignOut => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let row = sqlx::query("DELETE FROM sessions WHERE username = $1")
                    .bind(&username)
                    .execute(&self.pool)
                    .await?;

                if row.rows_affected() == 0 {
                    return Err(OpError::Unauthorized {
                        reason: "No active session".into(),
                    });
                }

                Ok(OpSuccess::SignedOut)
            }

            OpArgs::ChangeUsername { new_username } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                sqlx::query("UPDATE users SET username = $1 WHERE username = $2")
                    .bind(&new_username)
                    .bind(&username)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| {
                        if let sqlx::Error::Database(ref db_err) = e {
                            if db_err.is_unique_violation() {
                                return OpError::EntityConflict {
                                    reason: "Username already exists",
                                };
                            }
                        }
                        OpError::Generic(e.into())
                    })?;

                Ok(OpSuccess::UsernameChanged)
            }

            OpArgs::ChangePassword { curr_password, new_password } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let row = sqlx::query("SELECT password FROM users WHERE username = $1")
                    .bind(&username)
                    .fetch_one(&self.pool)
                    .await?;

                let hashed_password: String = row.get("password");

                let argon2 = Argon2::default();
                let parsed_hash = PasswordHash::new(&hashed_password)
                    .map_err(|e| OpError::Generic(format!("Failed to parse password hash: {e}").into()))?;

                argon2
                    .verify_password(curr_password.as_bytes(), &parsed_hash)
                    .map_err(|_| OpError::Unauthorized {
                        reason: "Incorrect password".into(),
                    })?;

                let new_hash = salt_and_hash_password(&new_password);

                sqlx::query("UPDATE users SET password = $1 WHERE username = $2")
                    .bind(&new_hash)
                    .bind(&username)
                    .execute(&self.pool)
                    .await?;

                Ok(OpSuccess::PasswordChanged)
            }

            OpArgs::GetTrashRetention => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let row = sqlx::query("SELECT trash_retention_days FROM users WHERE username = $1")
                    .bind(&username)
                    .fetch_one(&self.pool)
                    .await?;

                let days: i32 = row.get("trash_retention_days");
                Ok(OpSuccess::TrashRetention { days })
            }

            OpArgs::SetTrashRetention { days } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                const VALID_DAYS: [i32; 6] = [0, 7, 14, 30, 60, 90];
                if !VALID_DAYS.contains(&days) {
                    return Err(OpError::ValidationFailed {
                        reason: "Invalid retention period".into(),
                    });
                }

                sqlx::query("UPDATE users SET trash_retention_days = $1 WHERE username = $2")
                    .bind(days)
                    .bind(&username)
                    .execute(&self.pool)
                    .await?;

                Ok(OpSuccess::TrashRetentionSet)
            }
        }
    }
}
