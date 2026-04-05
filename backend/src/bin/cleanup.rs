use aws_sdk_s3::Client;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use backend::{AppData, Error};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let app = AppData::build()
        .await
        .expect("Failed to initialize app data");

    match cleanup(&app.pool, &app.s3, &app.bucket).await {
        Ok(_) => log::info!("Cleanup completed successfully"),
        Err(e) => {
            log::error!("Error during cleanup: {:?}", e);
            std::process::exit(1);
        }
    }
}

async fn cleanup(pool: &PgPool, s3: &Client, bucket: &str) -> Result<usize, Error> {
    let mut tx = pool.begin().await?;

    // get all expired files in trash and s3 keys (per-user retention from users table)
    let files = sqlx::query(
        "SELECT fs.file_id, f.s3_fileid
              FROM filesystem fs
              JOIN files f ON fs.file_id = f.id
              JOIN users u ON fs.owner_username = u.username
              WHERE fs.deleted_at IS NOT NULL
                AND u.trash_retention_days > 0
                AND fs.deleted_at < NOW() - make_interval(days => u.trash_retention_days)
                AND fs.type = 'file_link'",
    )
    .fetch_all(&mut *tx)
    .await?;

    let s3_keys: Vec<String> = files.iter().map(|row| row.get("s3_fileid")).collect();
    let file_ids: Vec<Uuid> = files.iter().map(|row| row.get("file_id")).collect();

    // delete from fs (files and folders) per-user retention
    let fs_res = sqlx::query(
        "DELETE FROM filesystem fs
              USING users u
              WHERE fs.owner_username = u.username
                AND fs.deleted_at IS NOT NULL
                AND u.trash_retention_days > 0
                AND fs.deleted_at < NOW() - make_interval(days => u.trash_retention_days)",
    )
    .execute(&mut *tx)
    .await?;

    // delete from files
    if !file_ids.is_empty() {
        sqlx::query(
            "DELETE FROM files
              WHERE id = ANY($1)",
        )
        .bind(&file_ids)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    // then delete from s3
    // 4. Best-effort S3 cleanup (failures logged, not bubbled)
    if !s3_keys.is_empty() {
        let objects: Vec<ObjectIdentifier> = s3_keys
            .into_iter()
            .map(|k| ObjectIdentifier::builder().key(k).build())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("S3 builder error: {e:?}"))?;

        let delete = Delete::builder()
            .set_objects(Some(objects))
            .build()
            .map_err(|e| format!("S3 builder error: {e:?}"))?;

        match s3
            .delete_objects()
            .bucket(bucket)
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

    Ok(fs_res.rows_affected() as usize)
}
