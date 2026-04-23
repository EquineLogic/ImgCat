pub mod models;

use crate::AppData;
use crate::notify::SharingEvent;
use argon2::{
    Argon2, PasswordHasher, PasswordVerifier,
    password_hash::{PasswordHash, SaltString, rand_core::OsRng},
};
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use kittycat::perms::{check_patch_changes_str, has_perm_str};
use models::*;
use sqlx::Row;
use std::fmt::Display;
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
    ListGroups {},
    AcceptGroupInvite { group_id: Uuid },
    DenyGroupInvite { group_id: Uuid },
    InviteGroupMember { target_id: Uuid, perms: Vec<String> },
}

impl OpArgs {
    /// What perms we need to check for every op
    const fn needed_perm(&self) -> Option<&'static str> {
        match self {
            // --- Filesystem ---
            Self::CreateFolder { .. }     => Some("fs.create_folder"),
            Self::ListFolder { .. }       => Some("fs.list_folder"),
            Self::DeleteFolder { .. }     => Some("fs.delete_folder"),
            Self::RenameFolder { .. }     => Some("fs.rename_folder"),
            Self::UploadFile { .. }       => Some("fs.upload_file"),
            Self::ListFiles { .. }        => Some("fs.list_files"),
            Self::RenameFile { .. }       => Some("fs.rename_file"),
            Self::DeleteFile { .. }       => Some("fs.delete_file"),
            Self::Reorder { .. }          => Some("fs.reorder_entries"),
            Self::MoveEntry { .. }        => Some("fs.move_entry"),
            Self::ListTrash               => Some("fs.list_trash"),
            Self::RestoreEntry { .. }     => Some("fs.restore_entry"),
            Self::DeleteTrashEntry { .. } => Some("fs.delete_trash_entry"),

            // --- Auth ---
            // Public entry points
            Self::CreateUser { .. }           => None,
            Self::CreateLoginSession { .. }   => None,
            
            Self::DeleteSession { .. }        => Some("auth.delete_session"),
            Self::ChangeUsername { .. }       => Some("auth.change_username"),
            Self::ChangePassword { .. }       => None, // user-only op
            Self::GetTrashRetention           => Some("auth.get_trash_retention"),
            Self::SetTrashRetention { .. }    => Some("auth.set_trash_retention"),

            // --- Groups ---
            Self::CreateGroup { .. }          => None, // user-only op
            Self::ListGroups { .. }           => None, // user-only op
            Self::AcceptGroupInvite { .. }    => None, // user-only op
            Self::DenyGroupInvite { .. }      => None, // user-only op
            Self::InviteGroupMember { .. }    => Some("group_members.invite"),
        }
    }
}

// ── Outputs ─────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
#[serde(tag = "op")]
pub enum OpSuccess {
    // Filesystem
    Folders {
        folders: Vec<Folder>,
    },
    Files {
        files: Vec<FileEntry>,
    },
    TrashItems {
        items: Vec<TrashEntry>,
    },

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
    TrashRetention {
        days: i32,
    },

    // Groups
    GroupMembers {
        group_members: Vec<GroupMember>
    },
    CreatedGroup {
        group_id: Uuid
    },

    // Ok
    Ok
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
    Unauthorized { reason: &'static str },
    OpNeedsPerms { perm: &'static str },
    UserOnlyOp,
    GroupOnlyOp
}

impl<T: Display + Send + Sync + 'static> From<T> for OpError {
    fn from(value: T) -> Self {
        Self::Generic(value.to_string().into())
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum OpCtx {
    User { id: Uuid, username: String, group: Option<(Uuid, String)> },
}

impl OpCtx {
    pub fn account_id(&self) -> Uuid {
        match self {
            Self::User { id: user_id, group, .. } => {
                if let Some((group_id, _)) = group {
                    *group_id
                } else {
                    *user_id
                }
            },
        }
    }

    /// Returns the real account id (like account_id but returns user id instead of group id if the user is executing in a group subcontext)
    pub fn real_account_id(&self) -> Uuid {
        match self {
            Self::User { id: user_id, .. } => {
                *user_id
            },
        }
    }

    pub fn is_user(&self) -> bool {
        match self {
            Self::User { group, .. } => group.is_none()
        }
    }

    pub fn is_group(&self) -> bool {
        match self {
            Self::User { group, .. } => group.is_some()
        }
    }

    pub fn username(&self) -> String {
        match self {
            Self::User { username: user_username, group, .. } => {
                if let Some((_, group_username)) = group {
                    group_username.to_string()
                } else {
                    user_username.to_string()
                }
            },
        }
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

#[allow(dead_code)]
struct GetPerms {
    perms: Vec<String>
}

impl AppData {
    /// Returns underlying user id and access level of current context, taking into account groups etc.
    async fn get_perms(&self, ctx: &OpCtx) -> Result<GetPerms, OpError> {
        match ctx {
            OpCtx::User { id: user_id, group, .. } => {
                let Some((group_id, _)) = group else {
                    return Ok(GetPerms { perms: vec!["global.*".to_string()] })
                };

                let Some(res) = sqlx::query("SELECT perms FROM group_members WHERE user_id = $1 AND group_id = $2 AND state = 'accepted'")
                .bind(user_id)
                .bind(group_id)
                .fetch_optional(&self.pool)
                .await? else {
                    return Err(OpError::EntityNotFound { reason: "Connected user is not an accepted member of the group" });
                };

                let perms: Vec<String> = res.try_get(0)?;
                Ok(GetPerms { perms })
            }
        }
    }

    fn is_valid_password(password: &str) -> bool {
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_symbol = password.chars().any(|c| !c.is_alphanumeric());
        let is_long_enough = password.len() >= 8;
        has_uppercase && has_lowercase && has_digit && has_symbol && is_long_enough
    }

    pub async fn exec_op(&self, op: OpArgs, user_id: Option<OpCtx>) -> Result<OpSuccess, OpError> {
        let mut user_perms = None;

        if let Some(np) = op.needed_perm() {
            let Some(ref uid) = user_id else {
                return Err(OpError::UserNotLoggedIn);
            };
            let perms = self.get_perms(uid).await?;

            if !has_perm_str(&perms.perms, np) {
                return Err(OpError::OpNeedsPerms { perm: np })
            }

            user_perms = Some(perms)
        }

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

                Ok(OpSuccess::Ok)
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

                Ok(OpSuccess::Ok)
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

                Ok(OpSuccess::Ok)
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
                Ok(OpSuccess::Ok)
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

                Ok(OpSuccess::Ok)
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

                Ok(OpSuccess::Ok)
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
                Ok(OpSuccess::Ok)
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

                Ok(OpSuccess::Ok)
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

                Ok(OpSuccess::Ok)
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

                Ok(OpSuccess::Ok)
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

                if username.len() < 5 {
                    return Err("Username must be at least 5 characters".into());
                }
                if name.len() < 5 {
                    return Err("Display name must be at least 5 characters".into());
                }
                if password.len() < 8 {
                    return Err("Password must be at least 8 characters".into());
                }
                if !Self::is_valid_password(&password) {
                    return Err("Password must contain at least one uppercase letter, one lowercase letter, one digit, and one special character".into());
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

                let session = models::auth::Session::new(&mut *tx, new_user_id, username)
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
                    return Err(OpError::UserOnlyOp)
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

                let group_id: Uuid = row.try_get(0)?;

                // Add user to group members
                sqlx::query(
                    "
                    INSERT INTO group_members (
                        group_id, 
                        user_id, 
                        sender_id, 
                        perms, 
                        state
                    )
                    VALUES (
                        $1,        -- The ID of the group just created
                        $2,        -- The ID of the user creating the group
                        $2,        -- The creator is also the sender
                        $3,        -- Owner gets full perms (global.*)
                        'accepted' -- No invite needed for the creator
                    )
                    "
                )
                .bind(&group_id)
                .bind(&uid.account_id())
                .bind(&["global.*"])
                .execute(&mut *tx)
                .await?;

                tx.commit().await?;

                Ok(OpSuccess::CreatedGroup { group_id })
            }

            OpArgs::ListGroups {  } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };
                if !uid.is_user() {
                    return Err(OpError::UserOnlyOp)
                }

                let group_members: Vec<GroupMember> = sqlx::query_as(
                    "SELECT 
                            gm.id, 
                            gm.group_id, 
                            gm.group_type,
                            u_group.name AS group_name,           
                            u_group.username AS group_username,  
                            gm.user_id,
                            gm.user_type,
                            gm.sender_id,
                            gm.sender_type,
                            u.username AS sender_username, -- The new field
                            gm.perms, 
                            gm.state, 
                            gm.created_at
                        FROM group_members gm
                        JOIN users u_group ON gm.group_id = u_group.id AND gm.group_type = u_group.user_type
                        JOIN users u ON gm.sender_id = u.id AND gm.sender_type = u.user_type
                        WHERE gm.user_id = $1
                        ORDER BY gm.created_at DESC
                    "
                )
                .bind(uid.account_id())
                .fetch_all(&self.pool)
                .await?;

                Ok(OpSuccess::GroupMembers { group_members })
            }

            OpArgs::AcceptGroupInvite { group_id } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };
                if !uid.is_user() {
                    return Err(OpError::UserOnlyOp)
                }

                let mut tx = self.pool.begin().await?;

                let Some(id) = sqlx::query("UPDATE group_members SET state = 'accepted' WHERE user_id = $1 AND state = 'pending_invite' AND group_id = $2 RETURNING sender_id")
                .bind(uid.account_id())
                .bind(group_id)
                .fetch_optional(&mut *tx)
                .await? else {
                    return Err(OpError::EntityNotFound { reason: "User does not have a pending invite for the requested group" });
                };

                let sender_id = id.try_get(0)?;

                tx.commit().await?;

                self.notify.send_to(sender_id, SharingEvent::AcceptedGroupInvite { group_id, user_id: uid.account_id() });

                Ok(OpSuccess::Ok)
            }

            OpArgs::DenyGroupInvite { group_id } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };
                if !uid.is_user() {
                    return Err(OpError::UserOnlyOp)
                }

                let Some(res) = sqlx::query("DELETE FROM group_members WHERE user_id = $1 AND state = 'pending_invite' AND group_id = $2 RETURNING sender_id")
                .bind(uid.account_id())
                .bind(group_id)
                .fetch_optional(&self.pool)
                .await? else {
                    return Err(OpError::EntityNotFound { reason: "User does not have a pending invite for the requested group" });
                };

                let sender_id = res.get(0);

                self.notify.send_to(sender_id, SharingEvent::DeniedGroupInvite { group_id, user_id: uid.account_id() });

                Ok(OpSuccess::Ok)
            }

            OpArgs::InviteGroupMember { target_id, perms } => {
                let Some(gid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };
                if !gid.is_group() {
                    return Err(OpError::GroupOnlyOp)
                }

                let sender_user_id = gid.real_account_id();
                let group_id = gid.account_id();
                assert!(sender_user_id != group_id);

                let Some(gp) = user_perms else {
                    return Err("internal error: no user_perms found".to_string().into())
                };

                check_patch_changes_str(&gp.perms, &[], &perms)?;

                sqlx::query(
                    "
                    INSERT INTO group_members (
                        group_id, 
                        user_id, 
                        sender_id, 
                        perms, 
                        state
                    )
                    VALUES (
                        $1,        -- The ID of the group 
                        $2,        -- The ID of the user to add
                        $3,        -- The creator 
                        $4, 
                        'pending_invite' 
                    )
                    "
                )
                .bind(group_id)
                .bind(target_id)
                .bind(sender_user_id)
                .bind(perms)
                .execute(&self.pool)
                .await?;

                self.notify.send_to(target_id, SharingEvent::NewGroupInvite { group_id });

                Ok(OpSuccess::Ok)
            }

            OpArgs::CreateLoginSession { username, password } => {
                let mut tx = self.pool.begin().await?;

                let row = sqlx::query("SELECT id, password FROM users WHERE username = $1")
                    .bind(&username)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|_| OpError::Unauthorized {
                        reason: "Invalid username or password",
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
                        reason: "Invalid username or password"
                    })?;

                let session = models::auth::Session::new(&mut *tx, found_user_id, username)
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
                        reason: "No active session"
                    });
                }

                Ok(OpSuccess::DeletedSession {
                    id,
                    token_type: SessionType::Login,
                }) 
            }

            OpArgs::ChangeUsername { new_username } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };

                if new_username.len() < 5 {
                    return Err("Username must be at least 5 characters".into());
                }

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

                Ok(OpSuccess::Ok)
            }

            OpArgs::ChangePassword {
                curr_password,
                new_password,
            } => {
                let Some(uid) = user_id else {
                    return Err(OpError::UserNotLoggedIn);
                };
                if !uid.is_user() {
                    return Err(OpError::UserOnlyOp)
                }

                if !Self::is_valid_password(&new_password) {
                    return Err("Password must contain at least one uppercase letter, one lowercase letter, one digit, and one special character".into());
                }

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
                        reason: "Incorrect password"
                    })?;

                let new_hash = salt_and_hash_password(&new_password);

                sqlx::query("UPDATE users SET password = $1 WHERE id = $2")
                    .bind(&new_hash)
                    .bind(uid.account_id())
                    .execute(&self.pool)
                    .await?;

                Ok(OpSuccess::Ok)
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

                Ok(OpSuccess::Ok)
            }
        }
    }
}
