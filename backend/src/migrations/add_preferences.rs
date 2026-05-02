use futures::future::{BoxFuture, FutureExt};
use sqlx::{Pool, Postgres};
use crate::migrations::Migration;

pub const MIGRATION: Migration = Migration {
    id: "add_preferences",
    description: "Add preferences column to users table",
    up,
};

fn up(pool: Pool<Postgres>) -> BoxFuture<'static, Result<(), crate::Error>> {
    async move {
        sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS preferences JSONB NOT NULL DEFAULT '{}'::jsonb")
            .execute(&pool)
            .await?;
        Ok(())
    }
    .boxed()
}
