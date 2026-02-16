use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};
use std::path::Path;

pub type DbPool = Pool<Sqlite>;

pub async fn init_pool(database_path: &str) -> Result<DbPool, Box<dyn std::error::Error + Send + Sync>> {
    let db_path = Path::new(database_path);
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let database_url = format!("sqlite:{database_path}?mode=rwc");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    Ok(pool)
}
