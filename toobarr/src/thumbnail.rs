use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

const THUMBNAIL_DIR: &str = "static/thumbnails";

pub async fn download_channel_thumbnail(
    channel_id: &str,
    url: &str
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let extension = get_extension_from_url(url);
    let filename = format!("{channel_id}.{extension}");
    let local_path = format!("{THUMBNAIL_DIR}/channels/{filename}");
    let web_path = format!("/static/thumbnails/channels/{filename}");

    download_image(url, &local_path).await?;

    Ok(web_path)
}

pub async fn download_video_thumbnail(
    video_id: &str,
    url: &str
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let extension = get_extension_from_url(url);
    let filename = format!("{video_id}.{extension}");
    let local_path = format!("{THUMBNAIL_DIR}/videos/{filename}");
    let web_path = format!("/static/thumbnails/videos/{filename}");

    download_image(url, &local_path).await?;

    Ok(web_path)
}

pub async fn download_image(
    url: &str,
    local_path: &str
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if Path::new(local_path).exists() {
        return Ok(());
    }

    if let Some(parent) = Path::new(local_path).parent() {
        fs::create_dir_all(parent).await?;
    }

    let response = reqwest::get(url).await?;

    if !response.status().is_success() {
        return Err(format!("Failed to download image: HTTP {}", response.status()).into());
    }

    let bytes = response.bytes().await?;

    let mut file = fs::File::create(local_path).await?;
    file.write_all(&bytes).await?;

    tracing::debug!("Downloaded thumbnail to {}", local_path);

    Ok(())
}

pub fn get_extension_from_url(url: &str) -> &str {
    if url.contains(".png") {
        "png"
    } else if url.contains(".webp") {
        "webp"
    } else {
        "jpg"
    }
}
