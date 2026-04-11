pub mod models;

use crate::AppData;
use crate::notify::SharingEvent;
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "op")]
pub enum OpArgs {
    // Filesystem
    CreateFolder {
        name: String,
        parent_id: Option<Uuid>,
    },
    ListFolder {
        parent_id: Option<Uuid>,
    },
    DeleteFolder {
        id: Uuid,
    },
    RenameFolder {
        id: Uuid,
        name: String,
    },
    UploadFile {
        data: Vec<u8>,
        name: String,
        mime_type: String,
        parent_id: Option<Uuid>,
    },
    ListFiles {
        parent_id: Option<Uuid>,
    },
    RenameFile {
        id: Uuid,
        name: String,
    },
    DeleteFile {
        id: Uuid,
    },
    Reorder {
        ids: Vec<Uuid>,
    },
    MoveEntry {
        id: Uuid,
        parent_id: Option<Uuid>,
    },
    ListTrash,
    RestoreEntry {
        id: Uuid,
    },
    DeleteTrashEntry {
        id: Uuid,
    },

    // Sharing
    SendShareRequest {
        filesystem_id: Uuid,
        recipient_username: String,
        access_level: String,
    },
    CancelShareRequest {
        id: Uuid,
    },
    AcceptShareRequest {
        id: Uuid,
    },
    DeclineShareRequest {
        id: Uuid,
    },
    ListPendingRequests,
    ListSentRequests,
    RevokePermission {
        id: Uuid,
    },
    ListMyGrants,
    ListSharedWithMe,
    ListSharedFolder {
        permission_filesystem_id: Uuid,
        parent_id: Option<Uuid>,
    },
    ListSharedFiles {
        permission_filesystem_id: Uuid,
        parent_id: Option<Uuid>,
    },
    CopySharedFile {
        filesystem_id: Uuid,
        parent_id: Option<Uuid>,
    },

    // Auth
    CreateUser {
        username: String,
        password: String,
        name: String,
    },
    CreateLoginSession {
        username: String,
        password: String,
    },
    DeleteSession {
        id: Uuid,
    },
    ChangeUsername {
        new_username: String,
    },
    ChangePassword {
        curr_password: String,
        new_password: String,
    },
    GetTrashRetention,
    SetTrashRetention {
        days: i32,
    },

    // Groups
    CreateGroup {
        username: String,
        name: String,
    },
}

// ── Outputs ─────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
#[serde(tag = "op")]
pub enum OpSuccess {
    // Filesystem
    FolderCreated,
    Folders {
        folders: Vec<Folder>,
    },
    FolderDeleted,
    FolderRenamed,
    FileUploaded,
    Files {
        files: Vec<FileEntry>,
    },
    FileRenamed,
    FileDeleted,
    Reordered,
    EntryMoved,
    TrashItems {
        items: Vec<TrashEntry>,
    },
    EntryRestored,
    TrashEntryDeleted,

    // Sharing
    ShareRequestSent,
    ShareRequestCancelled,
    ShareRequestAccepted,
    ShareRequestDeclined,
    PendingRequests {
        requests: Vec<ShareRequestEntry>,
    },
    SentRequests {
        requests: Vec<ShareRequestEntry>,
    },
    PermissionRevoked,
    MyGrants {
        grants: Vec<PermissionEntry>,
    },
    SharedWithMe {
        items: Vec<PermissionEntry>,
    },
    SharedFolders {
        folders: Vec<Folder>,
    },
    SharedFiles {
        files: Vec<FileEntry>,
    },
    // GetSharedFile reuses FileData
    FileCopied,

    // Auth
    CreatedSession {
        username: String,
        token: String,
        token_type: SessionType,
    },
    DeletedSession {
        id: Uuid,
        token_type: SessionType,
    },
    UsernameChanged,
    PasswordChanged,
    TrashRetention {
        days: i32,
    },
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
    UserOnlyOp,
}

impl<T: Error + Send + Sync + 'static> From<T> for OpError {
    fn from(value: T) -> Self {
        Self::Generic(value.into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpCtx {
    User(Uuid),
    Group { id: Uuid, underlying_group_member: Uuid }
}

impl OpCtx {
    pub fn account_id(self) -> Uuid {
        match self {
            Self::User(u) | Self::Group { id: u, .. } => u,
        }
    }

    pub fn is_user(self) -> bool {
        matches!(self, Self::User(_))
    }

    pub fn is_group(self) -> bool {
        matches!(self, Self::Group { .. })
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
    pub async fn exec_op(&self, op: OpArgs, user_id: Option<OpCtx>) -> Result<OpSuccess, OpError> {
        match op {
            // ─── Filesystem ─────────────────────────────────────────
            OpArgs::CreateFolder { name, parent_id } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let row = sqlx::query(
                    "INSERT INTO filesystem (name, type, owner_id, parent_id, sort_order)
                    VALUES ($1, 'folder', $2, $3,
                        COALESCE((SELECT MAX(sort_order) FROM filesystem WHERE parent_id IS NOT DISTINCT FROM $3 AND owner_id = $2 AND deleted_at IS NULL), 0) + 1
                    ) ON CONFLICT (parent_id, name) WHERE deleted_at IS NULL DO NOTHING"
                )
                .bind(name)
                .bind(uid.account_id())
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
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let rows: Vec<Folder> = sqlx::query_as(
                    "SELECT id, name FROM filesystem WHERE owner_id = $1 AND type = 'folder' AND parent_id IS NOT DISTINCT FROM $2 AND deleted_at IS NULL ORDER BY sort_order"
                )
                .bind(uid.account_id())
                .bind(parent_id)
                .fetch_all(&self.pool)
                .await?;

                Ok(OpSuccess::Folders { folders: rows })
            }

            OpArgs::DeleteFolder { id } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let res = sqlx::query(
                    "UPDATE filesystem SET deleted_at = NOW(), updated_at = NOW()
                            WHERE owner_id = $2
                            AND deleted_at IS NULL
                            AND path <@ (
                                SELECT path FROM filesystem
                                WHERE id = $1 AND owner_id = $2 AND type = 'folder' AND deleted_at IS NULL
                            )",
                )
                .bind(id)
                .bind(uid.account_id())
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
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let res = sqlx::query(
                    "UPDATE filesystem SET name = $1, updated_at = NOW() WHERE id = $2 AND owner_id = $3 AND type = 'folder' AND deleted_at IS NULL",
                )
                .bind(&name)
                .bind(id)
                .bind(uid.account_id())
                .execute(&self.pool)
                .await?;

                if res.rows_affected() == 0 {
                    return Err(OpError::EntityNotFound {
                        reason: "Folder not found",
                    });
                }

                Ok(OpSuccess::FolderRenamed)
            }

            OpArgs::UploadFile {
                data,
                name,
                mime_type,
                parent_id,
            } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let s3_key = Uuid::new_v4().to_string();

                self.local_s3
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
                    "INSERT INTO files (owner_id, s3_fileid, size_bytes, mime_type) VALUES ($1, $2, $3, $4) RETURNING id"
                )
                .bind(uid.account_id())
                .bind(&s3_key)
                .bind(data.len() as i64)
                .bind(&mime_type)
                .fetch_one(&mut *tx)
                .await?;

                let f_id: Uuid = f_row.get("id");

                let fs_row = sqlx::query(
                    "INSERT INTO filesystem (name, type, owner_id, parent_id, file_id, sort_order)
                     VALUES ($1, 'file_link', $2, $3, $4,
                        COALESCE((SELECT MAX(sort_order) FROM filesystem WHERE parent_id IS NOT DISTINCT FROM $3 AND owner_id = $2 AND deleted_at IS NULL), 0) + 1
                     )"
                )
                .bind(&name)
                .bind(uid.account_id())
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
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let rows: Vec<FileRow> = sqlx::query_as(
                    "SELECT fs.id, fs.name, f.size_bytes, f.mime_type, f.s3_fileid
                          FROM filesystem fs
                          JOIN files f ON fs.file_id = f.id
                          WHERE fs.owner_id = $1 AND fs.type = 'file_link'
                          AND fs.parent_id IS NOT DISTINCT FROM $2
                          AND fs.deleted_at IS NULL
                          ORDER BY fs.sort_order",
                )
                .bind(uid.account_id())
                .bind(parent_id)
                .fetch_all(&self.pool)
                .await?;

                let mut files: Vec<FileEntry> = Vec::with_capacity(rows.len());
                for row in rows {
                    let furl = FileUrl::new(self, row.s3_fileid)
                        .await
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
            OpArgs::RenameFile { id, name } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let res = sqlx::query(
                    "UPDATE filesystem SET name = $1, updated_at = NOW()
                     WHERE id = $2 AND owner_id = $3 AND type = 'file_link' AND deleted_at IS NULL",
                )
                .bind(&name)
                .bind(id)
                .bind(uid.account_id())
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
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let res = sqlx::query(
                    "UPDATE filesystem SET deleted_at = NOW(), updated_at = NOW()
                     WHERE id = $1 AND owner_id = $2 AND type = 'file_link' AND deleted_at IS NULL",
                )
                .bind(id)
                .bind(uid.account_id())
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
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let mut tx = self.pool.begin().await?;

                for (i, id) in ids.iter().enumerate() {
                    sqlx::query(
                        "UPDATE filesystem SET sort_order = $1 WHERE id = $2 AND owner_id = $3 AND deleted_at IS NULL"
                    )
                    .bind((i + 1) as i32)
                    .bind(id)
                    .bind(uid.account_id())
                    .execute(&mut *tx)
                    .await?;
                }

                tx.commit().await?;
                Ok(OpSuccess::Reordered)
            }

            OpArgs::MoveEntry { id, parent_id } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let res = sqlx::query(
                    "UPDATE filesystem
                     SET parent_id = $1,
                         sort_order = COALESCE((SELECT MAX(sort_order) FROM filesystem WHERE parent_id IS NOT DISTINCT FROM $1 AND owner_id = $3 AND deleted_at IS NULL), 0) + 1,
                         updated_at = NOW()
                     WHERE id = $2 AND owner_id = $3 AND deleted_at IS NULL
                       AND id <> $1
                       AND ($1 IS NULL OR EXISTS (
                           SELECT 1 FROM filesystem p
                           WHERE p.id = $1
                             AND p.owner_id = $3
                             AND p.type = 'folder'
                             AND p.deleted_at IS NULL
                             AND NOT (p.path <@ (SELECT path FROM filesystem WHERE id = $2 AND deleted_at IS NULL))
                       ))",
                )
                .bind(parent_id)
                .bind(id)
                .bind(uid.account_id())
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
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let rows: Vec<TrashRow> = sqlx::query_as(
                    "SELECT fs.id, fs.name, fs.type::text AS kind, fs.deleted_at, f.mime_type, f.s3_fileid
                       FROM filesystem fs
                       LEFT JOIN files f ON fs.file_id = f.id
                      WHERE fs.owner_id = $1
                        AND fs.deleted_at IS NOT NULL
                        AND NOT EXISTS (
                            SELECT 1 FROM filesystem anc
                            WHERE anc.owner_id = $1
                              AND anc.deleted_at IS NOT NULL
                              AND anc.id <> fs.id
                              AND fs.path <@ anc.path
                        )
                      ORDER BY fs.deleted_at DESC, fs.name",
                )
                .bind(uid.account_id())
                .fetch_all(&self.pool)
                .await?;

                let mut items: Vec<TrashEntry> = Vec::with_capacity(rows.len());
                for r in rows {
                    let furl = match r.s3_fileid {
                        Some(s3_fileid) => Some(
                            FileUrl::new(self, s3_fileid)
                                .await
                                .map_err(|e| OpError::Generic(e))?,
                        ),
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
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let res = sqlx::query(
                    "UPDATE filesystem SET deleted_at = NULL, updated_at = NOW()
                     WHERE owner_id = $2
                       AND deleted_at IS NOT NULL
                       AND path <@ (SELECT path FROM filesystem WHERE id = $1 AND owner_id = $2 AND deleted_at IS NOT NULL)
                       AND deleted_at = (SELECT deleted_at FROM filesystem WHERE id = $1 AND owner_id = $2)",
                )
                .bind(id)
                .bind(uid.account_id())
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
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let mut tx = self.pool.begin().await?;

                let file_rows = sqlx::query(
                    "SELECT fs.file_id, f.s3_fileid FROM filesystem fs
                       JOIN files f ON fs.file_id = f.id
                       WHERE fs.owner_id = $1
                         AND fs.deleted_at IS NOT NULL
                         AND fs.path <@ (SELECT path FROM filesystem WHERE id = $2 AND owner_id = $1 AND deleted_at IS NOT NULL)
                         AND fs.type = 'file_link'",
                )
                .bind(uid.account_id())
                .bind(id)
                .fetch_all(&mut *tx)
                .await?;

                let s3_keys: Vec<String> = file_rows.iter().map(|r| r.get("s3_fileid")).collect();
                let file_ids: Vec<Uuid> = file_rows.iter().map(|r| r.get("file_id")).collect();

                if s3_keys.len() > 1000 {
                    return Err(OpError::TooManyItems);
                }

                let fs_res = sqlx::query(
                    "DELETE FROM filesystem WHERE owner_id = $1
                       AND deleted_at IS NOT NULL
                       AND path <@ (SELECT path FROM filesystem WHERE id = $2 AND owner_id = $1 AND deleted_at IS NOT NULL)",
                )
                .bind(uid.account_id())
                .bind(id)
                .execute(&mut *tx)
                .await?;

                if fs_res.rows_affected() == 0 {
                    return Err(OpError::EntityNotFound {
                        reason: "Trash item not found",
                    });
                }

                if !file_ids.is_empty() {
                    sqlx::query("DELETE FROM files WHERE owner_id = $1 AND id = ANY($2)")
                        .bind(uid.account_id())
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
                        match self.local_s3
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
            OpArgs::SendShareRequest {
                filesystem_id,
                recipient_username,
                access_level,
            } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                // Verify caller owns the entry and capture its name
                let entry_row = sqlx::query(
                    "SELECT name FROM filesystem WHERE id = $1 AND owner_id = $2 AND deleted_at IS NULL"
                )
                .bind(filesystem_id)
                .bind(uid.account_id())
                .fetch_optional(&self.pool)
                .await?;

                let Some(entry_row) = entry_row else {
                    return Err(OpError::EntityNotFound {
                        reason: "Entry not found",
                    });
                };
                let entry_name: String = entry_row.get("name");

                // Look up recipient by username
                let recipient_row = sqlx::query("SELECT id FROM users WHERE username = $1")
                    .bind(&recipient_username)
                    .fetch_optional(&self.pool)
                    .await?;

                let Some(recipient_row) = recipient_row else {
                    return Err(OpError::EntityNotFound {
                        reason: "User not found",
                    });
                };

                let recipient_id: Uuid = recipient_row.get("id");

                if uid.account_id() == recipient_id {
                    return Err(OpError::ValidationFailed {
                        reason: "Cannot share with yourself".into(),
                    });
                }

                // Validate access_level
                if !["viewer", "editor"].contains(&access_level.as_str()) {
                    return Err(OpError::ValidationFailed {
                        reason: "Invalid access level".into(),
                    });
                }

                // Get sender username for the notification
                let sender_username: String =
                    sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
                        .bind(uid.account_id())
                        .fetch_one(&self.pool)
                        .await?;

                let request_id: Option<Uuid> = sqlx::query_scalar(
                    "INSERT INTO share_requests (filesystem_id, sender_id, recipient_id, access_level)
                     VALUES ($1, $2, $3, $4::access_level)
                     ON CONFLICT (filesystem_id, recipient_id) DO NOTHING
                     RETURNING id"
                )
                .bind(filesystem_id)
                .bind(uid.account_id())
                .bind(recipient_id)
                .bind(&access_level)
                .fetch_optional(&self.pool)
                .await?;

                let Some(request_id) = request_id else {
                    return Err(OpError::EntityConflict {
                        reason: "A share request already exists for this item and user",
                    });
                };

                self.notify.send_to(
                    recipient_id,
                    SharingEvent::NewShareRequest {
                        request_id,
                        filesystem_id,
                        entry_name,
                        sender_username,
                        access_level,
                    },
                );

                Ok(OpSuccess::ShareRequestSent)
            }

            OpArgs::CancelShareRequest { id } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let row = sqlx::query(
                    "DELETE FROM share_requests WHERE id = $1 AND sender_id = $2 RETURNING recipient_id",
                )
                .bind(id)
                .bind(uid.account_id())
                .fetch_optional(&self.pool)
                .await?;

                let Some(row) = row else {
                    return Err(OpError::EntityNotFound {
                        reason: "Share request not found",
                    });
                };

                let recipient_id: Uuid = row.get("recipient_id");
                self.notify.send_to(
                    recipient_id,
                    SharingEvent::ShareRequestCancelled { request_id: id },
                );

                Ok(OpSuccess::ShareRequestCancelled)
            }

            OpArgs::AcceptShareRequest { id } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let mut tx = self.pool.begin().await?;

                // Fetch and delete the request (must be addressed to caller)
                let row = sqlx::query(
                    "DELETE FROM share_requests WHERE id = $1 AND recipient_id = $2
                     RETURNING filesystem_id, sender_id, access_level::text",
                )
                .bind(id)
                .bind(uid.account_id())
                .fetch_optional(&mut *tx)
                .await?;

                let Some(row) = row else {
                    return Err(OpError::EntityNotFound {
                        reason: "Share request not found",
                    });
                };

                let filesystem_id: Uuid = row.get("filesystem_id");
                let sender_id: Uuid = row.get("sender_id");
                let access_level: String = row.get("access_level");

                // Insert permission
                sqlx::query(
                    "INSERT INTO permissions (filesystem_id, grantee_id, access_level, granted_by_id)
                     VALUES ($1, $2, $3::access_level, $4)
                     ON CONFLICT (filesystem_id, grantee_id) DO UPDATE SET access_level = $3::access_level"
                )
                .bind(filesystem_id)
                .bind(uid.account_id())
                .bind(&access_level)
                .bind(sender_id)
                .execute(&mut *tx)
                .await?;

                tx.commit().await?;

                let recipient_username: String =
                    sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
                        .bind(uid.account_id())
                        .fetch_one(&self.pool)
                        .await?;

                self.notify.send_to(
                    sender_id,
                    SharingEvent::ShareRequestAccepted {
                        request_id: id,
                        filesystem_id,
                        recipient_username,
                    },
                );

                Ok(OpSuccess::ShareRequestAccepted)
            }

            OpArgs::DeclineShareRequest { id } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let row = sqlx::query(
                    "DELETE FROM share_requests WHERE id = $1 AND recipient_id = $2 RETURNING sender_id",
                )
                .bind(id)
                .bind(uid.account_id())
                .fetch_optional(&self.pool)
                .await?;

                let Some(row) = row else {
                    return Err(OpError::EntityNotFound {
                        reason: "Share request not found",
                    });
                };

                let sender_id: Uuid = row.get("sender_id");
                let recipient_username: String =
                    sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
                        .bind(uid.account_id())
                        .fetch_one(&self.pool)
                        .await?;

                self.notify.send_to(
                    sender_id,
                    SharingEvent::ShareRequestDeclined {
                        request_id: id,
                        recipient_username,
                    },
                );

                Ok(OpSuccess::ShareRequestDeclined)
            }

            OpArgs::ListPendingRequests => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let rows: Vec<ShareRequestRow> = sqlx::query_as(
                    "SELECT sr.id, sr.filesystem_id, fs.name AS entry_name, fs.type::text AS entry_type,
                            su.username AS sender_username, ru.username AS recipient_username,
                            sr.access_level::text AS access_level,
                            sr.created_at, f.s3_fileid
                     FROM share_requests sr
                     JOIN filesystem fs ON sr.filesystem_id = fs.id
                     JOIN users su ON sr.sender_id = su.id
                     JOIN users ru ON sr.recipient_id = ru.id
                     LEFT JOIN files f ON fs.file_id = f.id
                     WHERE sr.recipient_id = $1 AND fs.deleted_at IS NULL
                     ORDER BY sr.created_at DESC"
                )
                .bind(uid.account_id())
                .fetch_all(&self.pool)
                .await?;

                let mut requests = Vec::with_capacity(rows.len());
                for r in rows {
                    let url = match r.s3_fileid {
                        Some(s3id) => {
                            Some(FileUrl::new(self, s3id).await.map_err(OpError::Generic)?)
                        }
                        None => None,
                    };
                    requests.push(ShareRequestEntry {
                        id: r.id,
                        filesystem_id: r.filesystem_id,
                        entry_name: r.entry_name,
                        entry_type: r.entry_type,
                        sender_username: r.sender_username,
                        recipient_username: r.recipient_username,
                        access_level: r.access_level,
                        created_at: r.created_at,
                        url,
                    });
                }

                Ok(OpSuccess::PendingRequests { requests })
            }

            OpArgs::ListSentRequests => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let rows: Vec<ShareRequestRow> = sqlx::query_as(
                    "SELECT sr.id, sr.filesystem_id, fs.name AS entry_name, fs.type::text AS entry_type,
                            su.username AS sender_username, ru.username AS recipient_username,
                            sr.access_level::text AS access_level,
                            sr.created_at, f.s3_fileid
                     FROM share_requests sr
                     JOIN filesystem fs ON sr.filesystem_id = fs.id
                     JOIN users su ON sr.sender_id = su.id
                     JOIN users ru ON sr.recipient_id = ru.id
                     LEFT JOIN files f ON fs.file_id = f.id
                     WHERE sr.sender_id = $1 AND fs.deleted_at IS NULL
                     ORDER BY sr.created_at DESC"
                )
                .bind(uid.account_id())
                .fetch_all(&self.pool)
                .await?;

                let mut requests = Vec::with_capacity(rows.len());
                for r in rows {
                    let url = match r.s3_fileid {
                        Some(s3id) => {
                            Some(FileUrl::new(self, s3id).await.map_err(OpError::Generic)?)
                        }
                        None => None,
                    };
                    requests.push(ShareRequestEntry {
                        id: r.id,
                        filesystem_id: r.filesystem_id,
                        entry_name: r.entry_name,
                        entry_type: r.entry_type,
                        sender_username: r.sender_username,
                        recipient_username: r.recipient_username,
                        access_level: r.access_level,
                        created_at: r.created_at,
                        url,
                    });
                }

                Ok(OpSuccess::SentRequests { requests })
            }

            OpArgs::RevokePermission { id } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let row = sqlx::query(
                    "DELETE FROM permissions WHERE id = $1 AND granted_by_id = $2 RETURNING grantee_id, filesystem_id",
                )
                .bind(id)
                .bind(uid.account_id())
                .fetch_optional(&self.pool)
                .await?;

                let Some(row) = row else {
                    return Err(OpError::EntityNotFound {
                        reason: "Permission not found",
                    });
                };

                let grantee_id: Uuid = row.get("grantee_id");
                let filesystem_id: Uuid = row.get("filesystem_id");
                self.notify.send_to(
                    grantee_id,
                    SharingEvent::PermissionRevoked {
                        permission_id: id,
                        filesystem_id,
                    },
                );

                Ok(OpSuccess::PermissionRevoked)
            }

            OpArgs::ListMyGrants => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let rows: Vec<PermissionRow> = sqlx::query_as(
                    "SELECT p.id, p.filesystem_id, fs.name AS entry_name, fs.type::text AS entry_type,
                            gu.username AS grantee_username, gbu.username AS granted_by,
                            p.access_level::text AS access_level,
                            p.created_at, f.s3_fileid
                     FROM permissions p
                     JOIN filesystem fs ON p.filesystem_id = fs.id
                     JOIN users gu ON p.grantee_id = gu.id
                     LEFT JOIN users gbu ON p.granted_by_id = gbu.id
                     LEFT JOIN files f ON fs.file_id = f.id
                     WHERE p.granted_by_id = $1 AND fs.deleted_at IS NULL
                     ORDER BY p.created_at DESC"
                )
                .bind(uid.account_id())
                .fetch_all(&self.pool)
                .await?;

                let mut grants = Vec::with_capacity(rows.len());
                for r in rows {
                    let url = match r.s3_fileid {
                        Some(s3id) => {
                            Some(FileUrl::new(self, s3id).await.map_err(OpError::Generic)?)
                        }
                        None => None,
                    };
                    grants.push(PermissionEntry {
                        id: r.id,
                        filesystem_id: r.filesystem_id,
                        entry_name: r.entry_name,
                        entry_type: r.entry_type,
                        grantee_username: r.grantee_username,
                        granted_by: r.granted_by,
                        access_level: r.access_level,
                        created_at: r.created_at,
                        url,
                    });
                }

                Ok(OpSuccess::MyGrants { grants })
            }

            OpArgs::ListSharedWithMe => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let rows: Vec<PermissionRow> = sqlx::query_as(
                    "SELECT p.id, p.filesystem_id, fs.name AS entry_name, fs.type::text AS entry_type,
                            gu.username AS grantee_username, gbu.username AS granted_by,
                            p.access_level::text AS access_level,
                            p.created_at, f.s3_fileid
                     FROM permissions p
                     JOIN filesystem fs ON p.filesystem_id = fs.id
                     JOIN users gu ON p.grantee_id = gu.id
                     LEFT JOIN users gbu ON p.granted_by_id = gbu.id
                     LEFT JOIN files f ON fs.file_id = f.id
                     WHERE p.grantee_id = $1 AND fs.deleted_at IS NULL
                     ORDER BY p.created_at DESC"
                )
                .bind(uid.account_id())
                .fetch_all(&self.pool)
                .await?;

                let mut items = Vec::with_capacity(rows.len());
                for r in rows {
                    let url = match r.s3_fileid {
                        Some(s3id) => {
                            Some(FileUrl::new(self, s3id).await.map_err(OpError::Generic)?)
                        }
                        None => None,
                    };
                    items.push(PermissionEntry {
                        id: r.id,
                        filesystem_id: r.filesystem_id,
                        entry_name: r.entry_name,
                        entry_type: r.entry_type,
                        grantee_username: r.grantee_username,
                        granted_by: r.granted_by,
                        access_level: r.access_level,
                        created_at: r.created_at,
                        url,
                    });
                }

                Ok(OpSuccess::SharedWithMe { items })
            }

            OpArgs::ListSharedFolder {
                permission_filesystem_id,
                parent_id,
            } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                // Verify permission exists and get the shared entry's path
                let perm_row = sqlx::query(
                    "SELECT fs.path::text FROM permissions p
                     JOIN filesystem fs ON p.filesystem_id = fs.id
                     WHERE p.filesystem_id = $1 AND p.grantee_id = $2 AND fs.deleted_at IS NULL",
                )
                .bind(permission_filesystem_id)
                .bind(uid.account_id())
                .fetch_optional(&self.pool)
                .await?;

                let Some(perm_row) = perm_row else {
                    return Err(OpError::Unauthorized {
                        reason: "No permission for this item".into(),
                    });
                };

                let shared_path: String = perm_row.get("path");

                // The parent to list: if None, use the shared folder itself
                let listing_parent = parent_id.unwrap_or(permission_filesystem_id);

                let rows: Vec<Folder> = sqlx::query_as(
                    "SELECT fs.id, fs.name FROM filesystem fs
                     WHERE fs.parent_id = $1 AND fs.type = 'folder' AND fs.deleted_at IS NULL
                       AND fs.path <@ $2::ltree
                     ORDER BY fs.sort_order",
                )
                .bind(listing_parent)
                .bind(&shared_path)
                .fetch_all(&self.pool)
                .await?;

                Ok(OpSuccess::SharedFolders { folders: rows })
            }

            OpArgs::ListSharedFiles {
                permission_filesystem_id,
                parent_id,
            } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                // Verify permission exists and get the shared entry's path
                let perm_row = sqlx::query(
                    "SELECT fs.path::text FROM permissions p
                     JOIN filesystem fs ON p.filesystem_id = fs.id
                     WHERE p.filesystem_id = $1 AND p.grantee_id = $2 AND fs.deleted_at IS NULL",
                )
                .bind(permission_filesystem_id)
                .bind(uid.account_id())
                .fetch_optional(&self.pool)
                .await?;

                let Some(perm_row) = perm_row else {
                    return Err(OpError::Unauthorized {
                        reason: "No permission for this item".into(),
                    });
                };

                let shared_path: String = perm_row.get("path");

                let listing_parent = parent_id.unwrap_or(permission_filesystem_id);

                let rows: Vec<FileRow> = sqlx::query_as(
                    "SELECT fs.id, fs.name, f.size_bytes, f.mime_type, f.s3_fileid
                     FROM filesystem fs
                     JOIN files f ON fs.file_id = f.id
                     WHERE fs.parent_id = $1 AND fs.type = 'file_link' AND fs.deleted_at IS NULL
                       AND fs.path <@ $2::ltree
                     ORDER BY fs.sort_order",
                )
                .bind(listing_parent)
                .bind(&shared_path)
                .fetch_all(&self.pool)
                .await?;

                let mut files = Vec::with_capacity(rows.len());
                for row in rows {
                    let furl = FileUrl::new(self, row.s3_fileid)
                        .await
                        .map_err(OpError::Generic)?;
                    files.push(FileEntry {
                        id: row.id,
                        name: row.name,
                        mime_type: row.mime_type,
                        size_bytes: row.size_bytes,
                        url: furl,
                    });
                }

                Ok(OpSuccess::SharedFiles { files })
            }
            OpArgs::CopySharedFile {
                filesystem_id,
                parent_id,
            } => {
                let Some(uid) = user_id else {
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
                           WHERE p.grantee_id = $2
                             AND fs_shared.deleted_at IS NULL
                             AND fs.path <@ fs_shared.path
                       )",
                )
                .bind(filesystem_id)
                .bind(uid.account_id())
                .fetch_optional(&self.pool)
                .await?;

                let Some(row) = row else {
                    return Err(OpError::EntityNotFound {
                        reason: "File not found or no permission",
                    });
                };

                let name: String = row.get("name");
                let s3_fileid: String = row.get("s3_fileid");
                let size_bytes: i64 = row.get("size_bytes");
                let mime_type: String = row.get("mime_type");

                // Create a new files row pointing to the same S3 object
                let mut tx = self.pool.begin().await?;

                let f_row = sqlx::query(
                    "INSERT INTO files (owner_id, s3_fileid, size_bytes, mime_type) VALUES ($1, $2, $3, $4) RETURNING id"
                )
                .bind(uid.account_id())
                .bind(&s3_fileid)
                .bind(size_bytes)
                .bind(&mime_type)
                .fetch_one(&mut *tx)
                .await?;

                let f_id: Uuid = f_row.get("id");

                sqlx::query(
                    "INSERT INTO filesystem (name, type, owner_id, parent_id, file_id, sort_order)
                     VALUES ($1, 'file_link', $2, $3, $4,
                        COALESCE((SELECT MAX(sort_order) FROM filesystem WHERE parent_id IS NOT DISTINCT FROM $3 AND owner_id = $2 AND deleted_at IS NULL), 0) + 1
                     )"
                )
                .bind(&name)
                .bind(uid.account_id())
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
            OpArgs::CreateUser {
                username,
                password,
                name,
            } => {
                if !crate::config::CONFIG.allow_register {
                    return Err(OpError::EntityConflict {
                        reason: "Registration is disabled",
                    });
                }

                let hashed_password = salt_and_hash_password(&password);

                let mut tx = self.pool.begin().await?;

                let res = sqlx::query(
                    "INSERT INTO users (username, name, password) VALUES ($1, $2, $3) ON CONFLICT (username) DO NOTHING RETURNING id"
                )
                .bind(&username)
                .bind(&name)
                .bind(&hashed_password)
                .fetch_optional(&mut *tx)
                .await?;

                let Some(row) = res else {
                    return Err(OpError::EntityConflict {
                        reason: "Username already exists",
                    });
                };

                let new_user_id: Uuid = row.get("id");

                let session = models::auth::Session::new(&mut *tx, new_user_id, username, None)
                    .await
                    .map_err(|e| OpError::Generic(e))?;

                tx.commit().await?;

                Ok(OpSuccess::CreatedSession {
                    username: session.username,
                    token: session.token,
                    token_type: SessionType::Login,
                })
            }

            OpArgs::CreateGroup {
                username,
                name,
            } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };
                if !uid.is_user() {

                }

                let mut tx = self.pool.begin().await?;

                let res = sqlx::query(
                    "INSERT INTO users (username, name, user_type) VALUES ($1, $2, 'group') ON CONFLICT (username) DO NOTHING RETURNING id"
                )
                .bind(&username)
                .bind(&name)
                .fetch_optional(&mut *tx)
                .await?;

                let Some(row) = res else {
                    return Err(OpError::EntityConflict {
                        reason: "Username already exists",
                    });
                };

                let new_user_id: Uuid = row.get("id");

                // Add user to group members
                sqlx::query(
                    "
                    INSERT INTO group_members (
                        group_id, 
                        user_id, 
                        sender_id, 
                        access_level, 
                        state
                    )
                    VALUES (
                        $1,        -- The ID of the group just created
                        $2,        -- The ID of the user creating the group
                        $2,        -- The creator is also the sender
                        'owner',   -- Initial owner gets full permissions
                        'accepted' -- No invite needed for the creator
                    )
                    "
                )
                .bind(&new_user_id)
                .bind(&uid.account_id())
                .fetch_optional(&mut *tx)
                .await?;


                let session = models::auth::Session::new(&mut *tx, new_user_id, username, Some(uid.account_id()))
                    .await
                    .map_err(|e| OpError::Generic(e))?;

                tx.commit().await?;

                Ok(OpSuccess::CreatedSession {
                    username: session.username,
                    token: session.token,
                    token_type: SessionType::Login,
                })
            }

            OpArgs::CreateLoginSession { username, password } => {
                let mut tx = self.pool.begin().await?;

                let row = sqlx::query("SELECT id, password FROM users WHERE username = $1")
                    .bind(&username)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|_| OpError::Unauthorized {
                        reason: "Invalid username or password".into(),
                    })?;

                let found_user_id: Uuid = row.get("id");
                let hashed_password: String = row.get("password");

                let argon2 = Argon2::default();
                let parsed_hash = PasswordHash::new(&hashed_password).map_err(|e| {
                    OpError::Generic(format!("Failed to parse password hash: {e}").into())
                })?;

                argon2
                    .verify_password(password.as_bytes(), &parsed_hash)
                    .map_err(|_| OpError::Unauthorized {
                        reason: "Invalid username or password".into(),
                    })?;

                let session = models::auth::Session::new(&mut *tx, found_user_id, username, None)
                    .await
                    .map_err(|e| OpError::Generic(e))?;

                tx.commit().await?;

                Ok(OpSuccess::CreatedSession {
                    username: session.username,
                    token: session.token,
                    token_type: SessionType::Login,
                })
            }

            OpArgs::DeleteSession { id } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let row = sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND id = $2")
                    .bind(uid.account_id())
                    .bind(id)
                    .execute(&self.pool)
                    .await?;

                if row.rows_affected() == 0 {
                    return Err(OpError::Unauthorized {
                        reason: "No active session".into(),
                    });
                }

                Ok(OpSuccess::DeletedSession {
                    id,
                    token_type: SessionType::Login,
                }) // TODO: Change this if/when we add support for diff session types
            }

            OpArgs::ChangeUsername { new_username } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                sqlx::query("UPDATE users SET username = $1 WHERE id = $2")
                    .bind(&new_username)
                    .bind(uid.account_id())
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

            OpArgs::ChangePassword {
                curr_password,
                new_password,
            } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let row = sqlx::query("SELECT password FROM users WHERE id = $1")
                    .bind(uid.account_id())
                    .fetch_one(&self.pool)
                    .await?;

                let hashed_password: String = row.get("password");

                let argon2 = Argon2::default();
                let parsed_hash = PasswordHash::new(&hashed_password).map_err(|e| {
                    OpError::Generic(format!("Failed to parse password hash: {e}").into())
                })?;

                argon2
                    .verify_password(curr_password.as_bytes(), &parsed_hash)
                    .map_err(|_| OpError::Unauthorized {
                        reason: "Incorrect password".into(),
                    })?;

                let new_hash = salt_and_hash_password(&new_password);

                sqlx::query("UPDATE users SET password = $1 WHERE id = $2")
                    .bind(&new_hash)
                    .bind(uid.account_id())
                    .execute(&self.pool)
                    .await?;

                Ok(OpSuccess::PasswordChanged)
            }

            OpArgs::GetTrashRetention => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                let row = sqlx::query("SELECT trash_retention_days FROM users WHERE id = $1")
                    .bind(uid.account_id())
                    .fetch_one(&self.pool)
                    .await?;

                let days: i32 = row.get("trash_retention_days");
                Ok(OpSuccess::TrashRetention { days })
            }

            OpArgs::SetTrashRetention { days } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                const VALID_DAYS: [i32; 6] = [0, 7, 14, 30, 60, 90];
                if !VALID_DAYS.contains(&days) {
                    return Err(OpError::ValidationFailed {
                        reason: "Invalid retention period".into(),
                    });
                }

                sqlx::query("UPDATE users SET trash_retention_days = $1 WHERE id = $2")
                    .bind(days)
                    .bind(uid.account_id())
                    .execute(&self.pool)
                    .await?;

                Ok(OpSuccess::TrashRetentionSet)
            }
        }
    }
}
