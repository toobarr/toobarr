use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Channel {
    pub id: String,
    pub youtube_id: String,
    pub name: String,
    pub url: String,
    pub thumbnail_url: Option<String>,
    pub description: Option<String>,
    pub video_count: Option<i64>,
    pub last_synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String
}

#[derive(Debug, Deserialize)]
pub struct CreateChannel {
    pub url: String
}

impl Channel {
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r"SELECT id, youtube_id, name, url, thumbnail_url, description,
                      video_count, last_synced_at, created_at, updated_at
               FROM channels ORDER BY created_at DESC"
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r"SELECT id, youtube_id, name, url, thumbnail_url, description,
                      video_count, last_synced_at, created_at, updated_at
               FROM channels WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_youtube_id(
        pool: &SqlitePool,
        youtube_id: &str
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r"SELECT id, youtube_id, name, url, thumbnail_url, description,
                      video_count, last_synced_at, created_at, updated_at
               FROM channels WHERE youtube_id = ?"
        )
        .bind(youtube_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn insert(
        pool: &SqlitePool,
        id: &str,
        youtube_id: &str,
        name: &str,
        url: &str,
        thumbnail_url: Option<&str>,
        description: Option<&str>
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"INSERT INTO channels (id, youtube_id, name, url, thumbnail_url, description)
               VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(id)
        .bind(youtube_id)
        .bind(name)
        .bind(url)
        .bind(thumbnail_url)
        .bind(description)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_sync_info(
        pool: &SqlitePool,
        id: &str,
        video_count: i64,
        last_synced_at: &str
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"UPDATE channels SET video_count = ?, last_synced_at = ?, updated_at = datetime('now')
               WHERE id = ?"
        )
        .bind(video_count)
        .bind(last_synced_at)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM channels WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn update_thumbnail(
        pool: &SqlitePool,
        id: &str,
        thumbnail_url: &str
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"UPDATE channels SET thumbnail_url = ?, updated_at = datetime('now')
               WHERE id = ?"
        )
        .bind(thumbnail_url)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
