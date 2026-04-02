use crate::AppData;
use crate::models::auth::LoggedInUser;
use crate::models::filesystem::{Folder, NewFolder};
use axum::http::StatusCode;
use axum::{Json, extract::State, response::IntoResponse};
use sqlx::Row;

pub async fn create_folder(
    State(app): State<AppData>,
    user: LoggedInUser,
    Json(payload): Json<NewFolder>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let row = sqlx::query("INSERT INTO filesystem (name, type, owner_username, parent_id) VALUES ($1, 'folder', $2, $3) ON CONFLICT (parent_id, name) DO NOTHING")
        .bind(&payload.name)
        .bind(&user.username)
        .bind(payload.parent_id)
        .execute(&app.pool)
        .await.map_err(|e|
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Can't make folder error: {e}"))
        )?;

    // check if folder was created successfully
    if row.rows_affected() == 0 {
        return Err((
            StatusCode::CONFLICT,
            "A folder with that name already exists".to_string(),
        ));
    }

    Ok(StatusCode::CREATED)
}

pub async fn list_folders(
    State(app): State<AppData>,
    user: LoggedInUser,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let rows = sqlx::query(
        "SELECT id, name FROM filesystem WHERE owner_username = $1 AND type = 'folder'",
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

    let folders: Vec<Folder> = rows
        .into_iter()
        .map(|row| Folder {
            id: row.get("id"),
            name: row.get("name"),
        })
        .collect();

    Ok(Json(folders))
}
