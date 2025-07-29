use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use image::ImageFormat;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct ImageMetadata {
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
    pub size: usize,
}

impl ImageMetadata {
    pub fn format(&self) -> &ImageFormat {
        &self.format
    }
}

pub trait ImageFormatExt {
    fn extension(&self) -> &str;
    fn mime_type(&self) -> &str;
}

impl ImageFormatExt for ImageFormat {
    fn extension(&self) -> &str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Gif => "gif",
            ImageFormat::WebP => "webp",
            ImageFormat::Bmp => "bmp",
            ImageFormat::Ico => "ico",
            ImageFormat::Tiff => "tiff",
            _ => "bin",
        }
    }

    fn mime_type(&self) -> &str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Gif => "image/gif",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Bmp => "image/bmp",
            ImageFormat::Ico => "image/x-icon",
            ImageFormat::Tiff => "image/tiff",
            _ => "application/octet-stream",
        }
    }
}

/// Extract metadata from image data
pub fn extract_image_metadata(data: &[u8]) -> Result<ImageMetadata> {
    use std::io::Cursor;
    
    let cursor = Cursor::new(data);
    let reader = image::ImageReader::new(cursor)
        .with_guessed_format()
        .context("Failed to guess image format")?;

    let format = reader
        .format()
        .ok_or_else(|| anyhow::anyhow!("Unknown image format"))?;

    let img = reader.decode().context("Failed to decode image")?;

    Ok(ImageMetadata {
        format,
        width: img.width(),
        height: img.height(),
        size: data.len(),
    })
}

/// Create the upload directory structure for a given date
pub fn create_upload_directory(base_path: &Path, date: DateTime<Utc>) -> Result<PathBuf> {
    let year = date.format("%Y").to_string();
    let month = date.format("%m").to_string();
    let day = date.format("%d").to_string();

    let dir_path = base_path.join(year).join(month).join(day);

    fs::create_dir_all(&dir_path)
        .with_context(|| format!("Failed to create upload directory: {:?}", dir_path))?;

    Ok(dir_path)
}

/// Generate a unique filename
pub fn generate_unique_filename(original_name: &str) -> String {
    use chrono::Utc;
    
    let timestamp = Utc::now().timestamp_millis();
    let random_suffix: u32 = rand::random();
    
    let extension = Path::new(original_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("bin");

    let base_name = Path::new(original_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("file");

    // Sanitize the base name
    let safe_base: String = base_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();

    format!("{}_{:x}_{:x}.{}", safe_base, timestamp, random_suffix, extension)
}

/// Save uploaded data to disk
pub fn save_upload(data: &[u8], upload_dir: &Path, filename: &str) -> Result<PathBuf> {
    let file_path = upload_dir.join(filename);

    let mut file = fs::File::create(&file_path)
        .with_context(|| format!("Failed to create file: {:?}", file_path))?;

    file.write_all(data)
        .with_context(|| format!("Failed to write file: {:?}", file_path))?;

    Ok(file_path)
}