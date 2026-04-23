use aws_sdk_s3::presigning::PresigningConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use crate::AppData;

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Folder {
    pub id: Uuid,
    pub name: String,
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct FileUrl {
    url: String,
}

impl FileUrl {
    const URL_EXPIRY: Duration = Duration::from_secs(10 * 60);

    pub async fn new(data: &AppData, fileid: String) -> Result<Self, crate::Error> {
        let presigned_request = data.cdn_s3
            .get_object()
            .bucket(&data.bucket)
            .key(&fileid)
            .presigned(PresigningConfig::expires_in(Self::URL_EXPIRY)?)
            .await?;

        Ok(Self {
            url: presigned_request.uri().to_string(),
        })
    }
}

#[derive(Serialize)]
pub struct FileEntry {
    pub id: Uuid,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub url: FileUrl,
}

#[derive(Serialize)]
pub struct TrashEntry {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub mime_type: Option<String>,
    pub deleted_at: chrono::DateTime<chrono::Utc>,
    pub url: Option<FileUrl>,
}

#[derive(sqlx::FromRow)]
pub struct FileRow {
    pub id: Uuid,
    pub name: String,
    pub size_bytes: i64,
    pub mime_type: String,
    pub s3_fileid: String,
}

#[derive(sqlx::FromRow)]
pub struct TrashRow {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub mime_type: Option<String>,
    pub deleted_at: chrono::DateTime<chrono::Utc>,
    pub s3_fileid: Option<String>,
}

pub struct NewFile {
    pub data: Vec<u8>,
    pub name: String,
    pub mime_type: String,
    pub parent_id: Option<Uuid>,
}

impl NewFile {
    pub async fn from_multipart(
        mut multipart: axum::extract::Multipart,
    ) -> Result<Self, (axum::http::StatusCode, String)> {
        use axum::http::StatusCode;

        let mut file_data: Option<Vec<u8>> = None;
        let mut name: Option<String> = None;
        let mut mime_type: Option<String> = None;
        let mut parent_id: Option<Uuid> = None;

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

#[derive(Deserialize)]
pub struct ReorderRequest {
    pub ids: Vec<Uuid>,
}

#[derive(Deserialize)]
pub struct MoveRequest {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
}
