// IMPORTANT: every SELECT/UPDATE query against `filesystem` MUST include
// `AND deleted_at IS NULL` unless it is intentionally operating on soft-deleted
// rows (e.g. trash listing or restore). Missing this filter will leak deleted
// items into the UI.

use crate::AppData;
use crate::models::auth::LoggedInUser;
use crate::models::filesystem::{
    DeleteFile, DeleteFolder, FileEntry, FileUrl, ListParams, MoveRequest, NewFile, NewFolder,
    RenameRequest, ReorderRequest, TrashEntry,
};
use crate::ops::{OpArgs, OpError, OpSuccess};
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::IntoResponse,
};
use sqlx::Row;
use uuid::Uuid;

pub async fn create_folder(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<NewFolder>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::CreateFolder {
            name: payload.name,
            parent_id: payload.parent_id,
        },
        Some(user.username),
    )
    .await
}

pub async fn list_folders(
    State(app): State<AppData>,
    user: LoggedInUser,
    Query(params): Query<ListParams>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(
        OpArgs::ListFolder {
            parent_id: params.parent_id,
        },
        Some(user.username),
    )
    .await
}

pub async fn delete_folder(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<DeleteFolder>,
) -> Result<OpSuccess, OpError> {
    app.exec_op(OpArgs::DeleteFolder { id: payload.id }, Some(user.username))
        .await
}

pub async fn rename_folder(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<RenameRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let res = sqlx::query(
        "UPDATE filesystem SET name = $1, updated_at = NOW() WHERE id = $2 AND owner_username = $3 AND type = 'folder' AND deleted_at IS NULL",
    )
    .bind(&payload.name)
    .bind(payload.id)
    .bind(&user.username)
    .execute(&app.pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    if res.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Folder not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn upload_file(
    State(app): State<AppData>,
    user: LoggedInUser,
    multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let payload = NewFile::from_multipart(multipart).await?;

    let s3_key = Uuid::new_v4().to_string();

    app.s3
        .put_object()
        .bucket(&app.bucket)
        .key(&s3_key)
        .body(payload.data.clone().into())
        .content_type(&payload.mime_type)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("S3 upload error: {e:?}"),
            )
        })?;

    let mut tx = app.pool.begin().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    let f_row = sqlx::query("INSERT INTO files (owner_username, s3_fileid, size_bytes, mime_type) VALUES ($1, $2, $3, $4) RETURNING id")
        .bind(&user.username)
        .bind(&s3_key)
        .bind(payload.data.len() as i64)
        .bind(&payload.mime_type)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {e}"),
            )
        })?;

    // don't have to check if rows b/c of fetch_one

    let f_id: uuid::Uuid = f_row.get("id");

    // COALESCE handles empty folders where MAX(sort_order) returns NULL -> defaults to 0, then +1
    let fs_row = sqlx::query(
        "INSERT INTO filesystem (name, type, owner_username, parent_id, file_id, sort_order)
         VALUES ($1, 'file_link', $2, $3, $4,
            COALESCE((SELECT MAX(sort_order) FROM filesystem WHERE parent_id IS NOT DISTINCT FROM $3 AND owner_username = $2 AND deleted_at IS NULL), 0) + 1
         )"
    )
        .bind(&payload.name)
        .bind(&user.username)
        .bind(payload.parent_id)
        .bind(f_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {e}"),
            )
        })?;

    if fs_row.rows_affected() == 0 {
        return Err((
            StatusCode::CONFLICT,
            "A file with that name already exists".to_string(),
        ));
    }

    tx.commit().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    Ok(StatusCode::CREATED)
}

pub async fn list_files(
    State(app): State<AppData>,
    user: LoggedInUser,
    Query(params): Query<ListParams>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: uuid::Uuid,
        name: String,
        size_bytes: i64,
        mime_type: String,
        s3_fileid: String,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT fs.id, fs.name, f.size_bytes, f.mime_type, f.s3_fileid
              FROM filesystem fs
              JOIN files f ON fs.file_id = f.id
              WHERE fs.owner_username = $1 AND fs.type = 'file_link'
              AND fs.parent_id IS NOT DISTINCT FROM $2
              AND fs.deleted_at IS NULL
              ORDER BY fs.sort_order",
    )
    .bind(&user.username)
    .bind(params.parent_id)
    .fetch_all(&app.pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    let mut files: Vec<FileEntry> = Vec::with_capacity(rows.len());
    for row in rows {
        let furl = FileUrl::new(&app, row.s3_fileid).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {e}"),
            )
        })?;

        files.push(FileEntry {
            id: row.id,
            name: row.name,
            mime_type: row.mime_type,
            size_bytes: row.size_bytes,
            url: furl,
        })
    }

    Ok(Json(files))
}

pub async fn get_file(
    State(app): State<AppData>,
    user: LoggedInUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Intentionally NOT filtering `deleted_at IS NULL`: the trash UI needs to
    // preview soft-deleted images so the user knows what they're restoring.
    // Ownership check still applies, so users can't read other users' files.
    let row = sqlx::query(
        "SELECT f.s3_fileid, f.mime_type
              FROM filesystem fs
              JOIN files f on fs.file_id = f.id
              WHERE fs.id = $1 AND fs.owner_username = $2 AND fs.type = 'file_link'",
    )
    .bind(&id)
    .bind(&user.username)
    .fetch_one(&app.pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    let s3_fileid: String = row.get("s3_fileid");
    let mime_type: String = row.get("mime_type");

    let obj = app
        .s3
        .get_object()
        .bucket(&app.bucket)
        .key(s3_fileid)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("S3 error: {e:?}"),
            )
        })?;

    let data = obj.body.collect().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("S3 error: {e:?}"),
        )
    })?;

    Ok(([(CONTENT_TYPE, mime_type)], data.into_bytes()))
}

pub async fn reorder(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<ReorderRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut tx = app.pool.begin().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    // Assign sort_order 1, 2, 3... based on the order of IDs received
    for (i, id) in payload.ids.iter().enumerate() {
        sqlx::query("UPDATE filesystem SET sort_order = $1 WHERE id = $2 AND owner_username = $3 AND deleted_at IS NULL")
            .bind((i + 1) as i32)
            .bind(id)
            .bind(&user.username)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database error: {e}"),
                )
            })?;
    }

    tx.commit().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn rename_file(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<RenameRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let res = sqlx::query(
        "UPDATE filesystem SET name = $1, updated_at = NOW()
         WHERE id = $2 AND owner_username = $3 AND type = 'file_link' AND deleted_at IS NULL",
    )
    .bind(&payload.name)
    .bind(payload.id)
    .bind(&user.username)
    .execute(&app.pool)
    .await
    .map_err(|e| {
        if let Some(db_err) = e.as_database_error() {
            if db_err.is_unique_violation() {
                return (
                    StatusCode::CONFLICT,
                    "A file with that name already exists".to_string(),
                );
            }
        }
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    if res.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_file(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<DeleteFolder>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Soft-delete: mark the row as deleted. The underlying files-row + S3 blob
    // are left in place for the cleanup script (or a future trash/undo feature)
    // to purge once the grace period has passed.
    let res = sqlx::query(
        "UPDATE filesystem SET deleted_at = NOW(), updated_at = NOW()
         WHERE id = $1 AND owner_username = $2 AND type = 'file_link' AND deleted_at IS NULL",
    )
    .bind(payload.id)
    .bind(&user.username)
    .execute(&app.pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    if res.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn move_entry(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<MoveRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Prevent moving into a non-existent/cross-user/non-folder target
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
    .bind(payload.parent_id)
    .bind(payload.id)
    .bind(&user.username)
    .execute(&app.pool)
    .await
    .map_err(|e| {
        // Unique (parent_id, name) collision when target folder has a same-named entry
        if let Some(db_err) = e.as_database_error() {
            if db_err.is_unique_violation() {
                return (
                    StatusCode::CONFLICT,
                    "An item with that name already exists in the target folder".to_string(),
                );
            }
        }
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    if res.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            "Entry or target not found".to_string(),
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_trash(
    State(app): State<AppData>,
    user: LoggedInUser,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: uuid::Uuid,
        name: String,
        kind: String, // "folder" | "file_link"
        mime_type: Option<String>,
        deleted_at: chrono::DateTime<chrono::Utc>,
        s3_fileid: Option<String>,
    }

    // Only return the "root" of each deletion: items whose nearest soft-deleted
    // ancestor is themselves. For cascaded folder deletes, this returns just the
    // top folder, not every descendant.
    let rows: Vec<Row> = sqlx::query_as(
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
    .bind(&user.username)
    .fetch_all(&app.pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    let mut items: Vec<TrashEntry> = Vec::with_capacity(rows.len());
    for r in rows {
        let furl = match r.s3_fileid {
            Some(s3_fileid) => {
                let furl = FileUrl::new(&app, s3_fileid).await.map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Database error: {e}"),
                    )
                })?;

                Some(furl)
            }
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

    Ok(Json(items))
}

pub async fn restore_entry(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<DeleteFolder>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Restore the entry and any descendants that were deleted together.
    // Deletion-timestamp matching groups items soft-deleted in the same
    // operation (NOW() in a single UPDATE yields identical timestamps).
    let res = sqlx::query(
        "UPDATE filesystem SET deleted_at = NULL, updated_at = NOW()
         WHERE owner_username = $2
           AND deleted_at IS NOT NULL
           AND path <@ (SELECT path FROM filesystem WHERE id = $1 AND owner_username = $2 AND deleted_at IS NOT NULL)
           AND deleted_at = (SELECT deleted_at FROM filesystem WHERE id = $1 AND owner_username = $2)",
    )
    .bind(payload.id)
    .bind(&user.username)
    .execute(&app.pool)
    .await
    .map_err(|e| {
        if let Some(db_err) = e.as_database_error() {
            if db_err.is_unique_violation() {
                return (
                    StatusCode::CONFLICT,
                    "A non-deleted item with that name already exists in the target folder. Rename it first.".to_string(),
                );
            }
        }
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    if res.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Trash item not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_trash_entry(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<DeleteFile>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut tx = app.pool.begin().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    // Gather S3 keys from all file_link descendants
    let file_rows = sqlx::query(
          "SELECT fs.file_id, f.s3_fileid FROM filesystem fs
           JOIN files f ON fs.file_id = f.id
           WHERE fs.owner_username = $1
             AND fs.deleted_at IS NOT NULL
             AND fs.path <@ (SELECT path FROM filesystem WHERE id = $2 AND owner_username = $1 AND deleted_at IS NOT NULL)
             AND fs.type = 'file_link'",
      )
      .bind(&user.username)
      .bind(payload.id)
      .fetch_all(&mut *tx)
      .await
      .map_err(|e| {
          (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {e}"))
      })?;

    let s3_keys: Vec<String> = file_rows.iter().map(|r| r.get("s3_fileid")).collect();
    let file_ids: Vec<Uuid> = file_rows.iter().map(|r| r.get("file_id")).collect();

    if s3_keys.len() > 1000 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Too many files to purge at once".to_string(),
        ));
    }

    // 1. Delete filesystem subtree first (FK RESTRICT forbids deleting files otherwise)
    let fs_res = sqlx::query(
          "DELETE FROM filesystem WHERE owner_username = $1
             AND deleted_at IS NOT NULL
             AND path <@ (SELECT path FROM filesystem WHERE id = $2 AND owner_username = $1 AND deleted_at IS NOT NULL)",
      )
      .bind(&user.username)
      .bind(payload.id)
      .execute(&mut *tx)
      .await
      .map_err(|e| {
          (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {e}"))
      })?;

    if fs_res.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Trash item not found".to_string()));
    }

    // 2. Delete files rows (safe now that filesystem refs are gone)
    if !file_ids.is_empty() {
        sqlx::query("DELETE FROM files WHERE owner_username = $1 AND id = ANY($2)")
            .bind(&user.username)
            .bind(&file_ids)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database error: {e}"),
                )
            })?;
    }

    // 3. Commit DB changes BEFORE S3 (orphan blob is recoverable; orphan row is not)
    tx.commit().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
    })?;

    // 4. Best-effort S3 cleanup (failures logged, not bubbled)
    if !s3_keys.is_empty() {
        let objects: Vec<ObjectIdentifier> = s3_keys
            .into_iter()
            .map(|k| ObjectIdentifier::builder().key(k).build())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("S3 builder error: {e:?}"),
                )
            })?;

        let delete = Delete::builder()
            .set_objects(Some(objects))
            .build()
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("S3 builder error: {e:?}"),
                )
            })?;

        match app
            .s3
            .delete_objects()
            .bucket(&app.bucket)
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

    Ok(StatusCode::NO_CONTENT)
}
