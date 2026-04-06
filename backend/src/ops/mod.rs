mod models;

use crate::AppData;
use models::*;
use std::error::Error;
use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "op")]
pub enum OpArgs {
    CreateFolder {
        name: String,
        parent_id: Option<Uuid>,
    },
    ListFolder {
        parent_id: Option<uuid::Uuid>,
    },
    DeleteFolder {
        id: uuid::Uuid,
    },
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "op")]
pub enum OpSuccess {
    /// The desired folder has been created
    FolderCreated,
    /// A list of folders in the specified parent folder
    Folders { folders: Vec<Folder> },
    /// The desired folder has been deleted
    FolderDeleted,
}

pub enum OpError {
    /// A generic error occurred during the operation
    Generic(crate::Error),
    /// The operation performed created a conflict with another entity
    EntityConflict { reason: &'static str },
    /// The operation performed referenced an entity that was not found
    EntityNotFound { reason: &'static str },

    /// User must be logged in to perform this action
    UserNotLoggedIn,
}

impl<T: Error + Send + Sync + 'static> From<T> for OpError {
    fn from(value: T) -> Self {
        Self::Generic(value.into())
    }
}

impl AppData {
    /// Execute an operation. If username is None, then only viewing will be allowed (TODO)
    pub async fn exec_op(
        &self,
        op: OpArgs,
        username: Option<String>,
    ) -> Result<OpSuccess, OpError> {
        match op {
            OpArgs::CreateFolder { name, parent_id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn);
                };

                // COALESCE handles empty folders where MAX(sort_order) returns NULL -> defaults to 0, then +1
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

                // check if folder was created successfully
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

                // Cascade soft-delete: mark the folder and every descendant (via ltree subtree)
                // as deleted. The path prefix match picks up the folder row itself plus every
                // file/folder underneath it, in a single UPDATE.
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
        }
    }
}
