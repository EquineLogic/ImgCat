use uuid::Uuid;
use crate::AppData;
use std::error::Error;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "op")]
pub enum OpArgs {
    CreateFolder {
        name: String,
        parent_id: Option<Uuid>,
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "op")]
pub enum OpSuccess {
    /// The desired folder has been created
    FolderCreated,
}

pub enum OpError {
    Generic(crate::Error),
    /// The operation performed created a conflict with another entity
    EntityConflict { reason: &'static str },
    /// User must be logged in to perform this action
    UserNotLoggedIn
}

impl<T: Error + Send + Sync + 'static> From<T> for OpError {
    fn from(value: T) -> Self {
        Self::Generic(value.into())
    }
}

impl AppData {
    /// Execute an operation. If username is None, then only viewing will be allowed (TODO)
    pub async fn exec_op(&self, op: OpArgs, username: Option<String>) -> Result<OpSuccess, OpError> {
        match op {
            OpArgs::CreateFolder { name, parent_id } => {
                let Some(username) = username else {
                    return Err(OpError::UserNotLoggedIn)
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
                    return Err(OpError::EntityConflict { reason: "A folder with that name already exists" });
                }

                Ok(OpSuccess::FolderCreated)
            }
        }
    }
}