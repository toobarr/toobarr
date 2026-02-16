use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Video {
    pub id: String,
    pub channel_id: String,
    pub youtube_id: String,
    pub title: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<i64>,
    pub upload_date: Option<String>,
    pub view_count: Option<i64>,
    pub webpage_url: String,
    pub created_at: String,
    pub updated_at: String
}

impl Video {
    pub async fn find_by_channel(
        pool: &SqlitePool,
        channel_id: &str
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r"SELECT id, channel_id, youtube_id, title, description, thumbnail_url,
                      duration_seconds, upload_date, view_count, webpage_url,
                      created_at, updated_at
               FROM videos WHERE channel_id = ? ORDER BY upload_date DESC"
        )
        .bind(channel_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r"SELECT id, channel_id, youtube_id, title, description, thumbnail_url,
                      duration_seconds, upload_date, view_count, webpage_url,
                      created_at, updated_at
               FROM videos WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    #[allow(dead_code)]
    pub async fn find_by_youtube_id(
        pool: &SqlitePool,
        youtube_id: &str
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r"SELECT id, channel_id, youtube_id, title, description, thumbnail_url,
                      duration_seconds, upload_date, view_count, webpage_url,
                      created_at, updated_at
               FROM videos WHERE youtube_id = ?"
        )
        .bind(youtube_id)
        .fetch_optional(pool)
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn upsert(
        pool: &SqlitePool,
        id: &str,
        channel_id: &str,
        youtube_id: &str,
        title: &str,
        description: Option<&str>,
        thumbnail_url: Option<&str>,
        duration_seconds: Option<i64>,
        upload_date: Option<&str>,
        view_count: Option<i64>,
        webpage_url: &str
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"INSERT INTO videos (id, channel_id, youtube_id, title, description,
                                   thumbnail_url, duration_seconds, upload_date,
                                   view_count, webpage_url)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(youtube_id) DO UPDATE SET
                   title = excluded.title,
                   description = excluded.description,
                   thumbnail_url = excluded.thumbnail_url,
                   view_count = excluded.view_count,
                   updated_at = datetime('now')"
        )
        .bind(id)
        .bind(channel_id)
        .bind(youtube_id)
        .bind(title)
        .bind(description)
        .bind(thumbnail_url)
        .bind(duration_seconds)
        .bind(upload_date)
        .bind(view_count)
        .bind(webpage_url)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub fn format_duration(&self) -> String {
        match self.duration_seconds {
            Some(secs) => {
                let hours = secs / 3600;
                let mins = (secs % 3600) / 60;
                let secs = secs % 60;
                if hours > 0 {
                    format!("{hours}:{mins:02}:{secs:02}")
                } else {
                    format!("{mins}:{secs:02}")
                }
            }
            None => String::from("--:--")
        }
    }

    #[allow(dead_code)]
    pub async fn update_thumbnail(
        pool: &SqlitePool,
        id: &str,
        thumbnail_url: &str
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"UPDATE videos SET thumbnail_url = ?, updated_at = datetime('now')
               WHERE id = ?"
        )
        .bind(thumbnail_url)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
