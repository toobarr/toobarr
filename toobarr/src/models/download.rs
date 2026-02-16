use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, SqlitePool};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Completed,
    Failed
}

impl DownloadStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Downloading => "downloading",
            Self::Completed => "completed",
            Self::Failed => "failed"
        }
    }
}

impl std::fmt::Display for DownloadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Download {
    pub id: String,
    pub video_id: String,
    pub status: String,
    pub file_path: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub progress_percent: Option<f64>,
    pub error_message: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadWithVideo {
    pub download: Download,
    pub video_title: String,
    pub video_thumbnail: Option<String>,
    pub channel_name: String
}

impl Download {
    pub fn status_enum(&self) -> DownloadStatus {
        match self.status.as_str() {
            "downloading" => DownloadStatus::Downloading,
            "completed" => DownloadStatus::Completed,
            "failed" => DownloadStatus::Failed,
            _ => DownloadStatus::Pending
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn progress_int(&self) -> i64 {
        self.progress_percent.unwrap_or(0.0) as i64
    }

    pub async fn find_all_with_video(
        pool: &SqlitePool
    ) -> Result<Vec<DownloadWithVideo>, sqlx::Error> {
        let rows = sqlx::query(
            r"SELECT d.id, d.video_id, d.status, d.file_path, d.file_size_bytes,
                      d.progress_percent, d.error_message, d.started_at, d.completed_at,
                      d.created_at, d.updated_at,
                      v.title as video_title, v.thumbnail_url as video_thumbnail,
                      c.name as channel_name
               FROM downloads d
               JOIN videos v ON d.video_id = v.id
               JOIN channels c ON v.channel_id = c.id
               ORDER BY d.created_at DESC"
        )
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| DownloadWithVideo {
                download: Download {
                    id: r.get("id"),
                    video_id: r.get("video_id"),
                    status: r.get("status"),
                    file_path: r.get("file_path"),
                    file_size_bytes: r.get("file_size_bytes"),
                    progress_percent: r.get("progress_percent"),
                    error_message: r.get("error_message"),
                    started_at: r.get("started_at"),
                    completed_at: r.get("completed_at"),
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at")
                },
                video_title: r.get("video_title"),
                video_thumbnail: r.get("video_thumbnail"),
                channel_name: r.get("channel_name")
            })
            .collect())
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r"SELECT id, video_id, status, file_path, file_size_bytes, progress_percent,
                      error_message, started_at, completed_at, created_at, updated_at
               FROM downloads WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    #[allow(dead_code)]
    pub async fn find_pending(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r"SELECT id, video_id, status, file_path, file_size_bytes, progress_percent,
                      error_message, started_at, completed_at, created_at, updated_at
               FROM downloads WHERE status = 'pending' ORDER BY created_at ASC"
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_video_id(
        pool: &SqlitePool,
        video_id: &str
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r"SELECT id, video_id, status, file_path, file_size_bytes, progress_percent,
                      error_message, started_at, completed_at, created_at, updated_at
               FROM downloads WHERE video_id = ? ORDER BY created_at DESC LIMIT 1"
        )
        .bind(video_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn insert(pool: &SqlitePool, id: &str, video_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO downloads (id, video_id) VALUES (?, ?)")
            .bind(id)
            .bind(video_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn update_status(
        pool: &SqlitePool,
        id: &str,
        status: DownloadStatus
    ) -> Result<(), sqlx::Error> {
        let status_str = status.as_str();
        let now = chrono::Utc::now().to_rfc3339();

        match status {
            DownloadStatus::Downloading => {
                sqlx::query(
                    r"UPDATE downloads SET status = ?, started_at = ?, updated_at = datetime('now')
                       WHERE id = ?"
                )
                .bind(status_str)
                .bind(&now)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DownloadStatus::Completed => {
                sqlx::query(
                    r"UPDATE downloads SET status = ?, completed_at = ?, progress_percent = 100.0,
                       updated_at = datetime('now') WHERE id = ?"
                )
                .bind(status_str)
                .bind(&now)
                .bind(id)
                .execute(pool)
                .await?;
            }
            _ => {
                sqlx::query(
                    r"UPDATE downloads SET status = ?, updated_at = datetime('now') WHERE id = ?"
                )
                .bind(status_str)
                .bind(id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn update_progress(
        pool: &SqlitePool,
        id: &str,
        progress_percent: f64
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"UPDATE downloads SET progress_percent = ?, updated_at = datetime('now') WHERE id = ?"
        )
        .bind(progress_percent)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_completed(
        pool: &SqlitePool,
        id: &str,
        file_path: &str,
        file_size_bytes: Option<i64>
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r"UPDATE downloads SET status = 'completed', file_path = ?, file_size_bytes = ?,
               progress_percent = 100.0, completed_at = ?, updated_at = datetime('now')
               WHERE id = ?"
        )
        .bind(file_path)
        .bind(file_size_bytes)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_failed(
        pool: &SqlitePool,
        id: &str,
        error_message: &str
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"UPDATE downloads SET status = 'failed', error_message = ?,
               updated_at = datetime('now') WHERE id = ?"
        )
        .bind(error_message)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM downloads WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
