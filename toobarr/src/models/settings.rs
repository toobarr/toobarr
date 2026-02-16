use sqlx::{Row, SqlitePool};

pub struct Settings;

impl Settings {
    pub async fn get(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?;
        Ok(row.map(|r| r.get("value")))
    }

    pub async fn set(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value"
        )
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_download_path(pool: &SqlitePool) -> Result<String, sqlx::Error> {
        Ok(Self::get(pool, "download_path")
            .await?
            .unwrap_or_else(|| "./downloads".to_string()))
    }

    pub async fn get_max_concurrent_downloads(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
        let value = Self::get(pool, "max_concurrent_downloads")
            .await?
            .unwrap_or_else(|| "2".to_string());
        Ok(value.parse().unwrap_or(2))
    }

    pub async fn get_extractor_args(pool: &SqlitePool) -> Result<String, sqlx::Error> {
        Ok(Self::get(pool, "extractor_args")
            .await?
            .unwrap_or_default())
    }

    pub async fn get_cookies_file(pool: &SqlitePool) -> Result<Option<String>, sqlx::Error> {
        Self::get(pool, "cookies_file").await
    }

    #[allow(dead_code)]
    pub async fn get_all(pool: &SqlitePool) -> Result<Vec<(String, String)>, sqlx::Error> {
        let rows = sqlx::query("SELECT key, value FROM settings ORDER BY key")
            .fetch_all(pool)
            .await?;
        Ok(rows.into_iter().map(|r| (r.get("key"), r.get("value"))).collect())
    }
}
