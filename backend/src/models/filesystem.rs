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
pub struct ListParams {
    pub parent_id: Option<uuid::Uuid>,
}

#[derive(Deserialize)]
pub struct DeleteFolder {
    pub id: uuid::Uuid,
}

#[derive(Deserialize)]
pub struct DeleteFile {
    pub id: uuid::Uuid,
}

#[derive(Deserialize)]
pub struct RenameRequest {
    pub id: uuid::Uuid,
    pub name: String,
}

pub struct NewFile {
    pub data: Vec<u8>,
    pub name: String,
    pub mime_type: String,
    pub parent_id: Option<uuid::Uuid>,
}

impl NewFile {
    pub async fn from_multipart(
        mut multipart: axum::extract::Multipart,
    ) -> Result<Self, (axum::http::StatusCode, String)> {
        use axum::http::StatusCode;

        let mut file_data: Option<Vec<u8>> = None;
        let mut name: Option<String> = None;
        let mut mime_type: Option<String> = None;
        let mut parent_id: Option<uuid::Uuid> = None;

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Multipart error: {e}")))?
        {
            match field.name() {
                Some("file") => {
                    mime_type = field.content_type().map(|s| s.to_string());
                    file_data = Some(
                        field
                            .bytes()
                            .await
                            .map_err(|e| {
                                (StatusCode::BAD_REQUEST, format!("Failed to read file: {e}"))
                            })?
                            .to_vec(),
                    );
                }
                Some("name") => {
                    let text = field
                        .text()
                        .await
                        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid field: {e}")))?;
                    if !text.is_empty() {
                        name = Some(text);
                    }
                }
                Some("parent_id") => {
                    let text = field
                        .text()
                        .await
                        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid field: {e}")))?;
                    if !text.is_empty() {
                        parent_id = Some(text.parse().map_err(|_| {
                            (StatusCode::BAD_REQUEST, "Invalid parent_id".to_string())
                        })?);
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            data: file_data.ok_or((StatusCode::BAD_REQUEST, "No file provided".to_string()))?,
            name: name.ok_or((StatusCode::BAD_REQUEST, "No name provided".to_string()))?,
            mime_type: mime_type.unwrap_or_else(|| "application/octet-stream".to_string()),
            parent_id,
        })
    }
}

#[derive(Serialize)]
pub struct FileEntry {
    pub id: uuid::Uuid,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: i64,
}

#[derive(Deserialize)]
pub struct ReorderRequest {
    pub ids: Vec<uuid::Uuid>,
}

#[derive(Deserialize)]
pub struct MoveRequest {
    pub id: uuid::Uuid,
    pub parent_id: Option<uuid::Uuid>,
}

#[derive(Serialize)]
pub struct TrashEntry {
    pub id: uuid::Uuid,
    pub name: String,
    pub kind: String, // "folder" | "file_link"
    pub mime_type: Option<String>,
    pub deleted_at: chrono::DateTime<chrono::Utc>,
}
