use serde::{Deserialize, Serialize};
use uuid;

#[derive(Deserialize)]
pub struct NewFolder {
    pub name: String,
    pub parent_id: Option<uuid::Uuid>,
}

#[derive(Serialize)]
pub struct Folder {
    pub id: uuid::Uuid,
    pub name: String,
}

#[derive(Deserialize)]
pub struct ListFoldersParams {
    pub parent_id: Option<uuid::Uuid>,
}

#[derive(Deserialize)]
pub struct DeleteFolder {
    pub id: uuid::Uuid,
}

#[derive(Deserialize)]
pub struct RenameFolder {
    pub id: uuid::Uuid,
    pub name: String,
}
