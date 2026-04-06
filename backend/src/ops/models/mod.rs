use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Folder {
    pub id: uuid::Uuid,
    pub name: String,
}
