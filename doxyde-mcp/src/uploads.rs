use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use image::GenericImageView;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Maximum thumbnail width in pixels
const THUMBNAIL_MAX_WIDTH: u32 = 1600;

// Magic bytes for image format detection
const JPEG_MAGIC: &[u8] = &[0xFF, 0xD8, 0xFF];
const PNG_MAGIC: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
const GIF_MAGIC: &[u8] = b"GIF";
const WEBP_MAGIC: &[u8] = b"RIFF";
const SVG_MAGIC: &[u8] = b"<svg";
const SVG_MAGIC_ALT: &[u8] = b"<?xml";

/// Supported image formats
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    WebP,
    Svg,
    Bmp,
    Ico,
    Tiff,
}

impl ImageFormat {
    /// Detect format from file content
    pub fn detect(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(anyhow::anyhow!("File too small to determine format"));
        }

        // Check for SVG first (text-based format)
        if data.starts_with(SVG_MAGIC) || data.starts_with(SVG_MAGIC_ALT) {
            return Ok(ImageFormat::Svg);
        }

        // Check for other formats
        if data.starts_with(JPEG_MAGIC) {
            Ok(ImageFormat::Jpeg)
        } else if data.starts_with(PNG_MAGIC) {
            Ok(ImageFormat::Png)
        } else if data.starts_with(GIF_MAGIC) {
            Ok(ImageFormat::Gif)
        } else if data.starts_with(WEBP_MAGIC) && data.len() > 12 && &data[8..12] == b"WEBP" {
            Ok(ImageFormat::WebP)
        } else {
            // Try to detect using image crate for other formats
            use std::io::Cursor;
            let cursor = Cursor::new(data);
            let reader = image::ImageReader::new(cursor)
                .with_guessed_format()
                .context("Failed to guess image format")?;

            match reader.format() {
                Some(image::ImageFormat::Bmp) => Ok(ImageFormat::Bmp),
                Some(image::ImageFormat::Ico) => Ok(ImageFormat::Ico),
                Some(image::ImageFormat::Tiff) => Ok(ImageFormat::Tiff),
                _ => Err(anyhow::anyhow!("Unknown image format")),
            }
        }
    }

    /// Convert to image crate format (if applicable)
    pub fn to_image_format(&self) -> Option<image::ImageFormat> {
        match self {
            ImageFormat::Jpeg => Some(image::ImageFormat::Jpeg),
            ImageFormat::Png => Some(image::ImageFormat::Png),
            ImageFormat::Gif => Some(image::ImageFormat::Gif),
            ImageFormat::WebP => Some(image::ImageFormat::WebP),
            ImageFormat::Bmp => Some(image::ImageFormat::Bmp),
            ImageFormat::Ico => Some(image::ImageFormat::Ico),
            ImageFormat::Tiff => Some(image::ImageFormat::Tiff),
            ImageFormat::Svg => None, // SVG is not supported by image crate
        }
    }
}

pub struct ImageMetadata {
    pub format: ImageFormat,
    pub width: Option<u32>,
    pub height: Option<u32>,
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
            ImageFormat::Svg => "svg",
            ImageFormat::Bmp => "bmp",
            ImageFormat::Ico => "ico",
            ImageFormat::Tiff => "tiff",
        }
    }

    fn mime_type(&self) -> &str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Gif => "image/gif",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Svg => "image/svg+xml",
            ImageFormat::Bmp => "image/bmp",
            ImageFormat::Ico => "image/x-icon",
            ImageFormat::Tiff => "image/tiff",
        }
    }
}

/// Extract metadata from image data
pub fn extract_image_metadata(data: &[u8]) -> Result<ImageMetadata> {
    let format = ImageFormat::detect(data)?;
    let size = data.len();

    // Extract dimensions based on format
    let (width, height) = match format {
        ImageFormat::Svg => {
            // SVG dimensions are not fixed, return None
            (None, None)
        }
        _ => {
            // Use the image crate to decode and get dimensions
            match image::load_from_memory(data) {
                Ok(img) => (Some(img.width()), Some(img.height())),
                Err(_) => {
                    // If we can't decode, just skip dimensions
                    (None, None)
                }
            }
        }
    };

    Ok(ImageMetadata {
        format,
        width,
        height,
        size,
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

    format!("{}_{}.{}", timestamp, random_suffix, extension)
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

/// Result of saving an image with optional thumbnail
#[derive(Debug, Clone)]
pub struct SavedImage {
    pub file_path: PathBuf,
    pub thumb_file_path: Option<PathBuf>,
    pub content_hash: String,
}

/// Compute SHA256 hex digest of content
pub fn compute_content_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Build a hash-based file path: base/ab/cd/abcd...64chars.ext
pub fn build_hash_based_path(base: &Path, hash: &str, ext: &str) -> Result<PathBuf> {
    if hash.len() < 4 {
        return Err(anyhow!("Hash too short: {}", hash));
    }
    let dir = base.join(&hash[0..2]).join(&hash[2..4]);
    Ok(dir.join(format!("{}.{}", hash, ext)))
}

/// Build thumbnail path by adding _thumb suffix to the stem
pub fn build_thumb_path(original: &Path) -> Result<PathBuf> {
    let stem = original
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid file path for thumbnail"))?;
    let ext = original
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| anyhow!("File has no extension"))?;
    let parent = original
        .parent()
        .ok_or_else(|| anyhow!("File has no parent directory"))?;
    Ok(parent.join(format!("{}_thumb.{}", stem, ext)))
}

/// Generate a thumbnail from image data
///
/// Returns None for SVG, GIF, or images already <= max_width.
pub fn generate_thumbnail(
    data: &[u8],
    format: &ImageFormat,
    max_width: u32,
) -> Result<Option<Vec<u8>>> {
    // Skip SVG (vector) and GIF (animation)
    match format {
        ImageFormat::Svg | ImageFormat::Gif => return Ok(None),
        _ => {}
    }

    let img_format = format
        .to_image_format()
        .ok_or_else(|| anyhow!("No image crate format for {:?}", format))?;

    let img = image::load_from_memory(data).context("Failed to decode image for thumbnail")?;

    let (width, _height) = img.dimensions();
    if width <= max_width {
        return Ok(None);
    }

    let thumb = img.resize(max_width, u32::MAX, image::imageops::FilterType::Lanczos3);

    let mut buf = std::io::Cursor::new(Vec::new());
    thumb
        .write_to(&mut buf, img_format)
        .context("Failed to encode thumbnail")?;

    Ok(Some(buf.into_inner()))
}

/// Save data to hash-based path with dedup
///
/// Returns (file_path, content_hash). Skips write if file already exists.
pub fn save_with_dedup(data: &[u8], base: &Path, ext: &str) -> Result<(PathBuf, String)> {
    let hash = compute_content_hash(data);
    let file_path = build_hash_based_path(base, &hash, ext)?;

    if !file_path.exists() {
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create dir: {:?}", parent))?;
        }
        let mut file = fs::File::create(&file_path)
            .with_context(|| format!("Failed to create: {:?}", file_path))?;
        file.write_all(data)
            .with_context(|| format!("Failed to write: {:?}", file_path))?;
    }

    Ok((file_path, hash))
}

/// Save image with hash-based naming and generate thumbnail
pub fn save_image_with_thumbnail(
    data: &[u8],
    base: &Path,
    metadata: &ImageMetadata,
) -> Result<SavedImage> {
    let ext = metadata.format.extension();
    let (file_path, content_hash) = save_with_dedup(data, base, ext)?;

    let thumb_file_path = match generate_thumbnail(data, &metadata.format, THUMBNAIL_MAX_WIDTH)? {
        Some(thumb_data) => {
            let thumb_path = build_thumb_path(&file_path)?;
            if !thumb_path.exists() {
                let mut file = fs::File::create(&thumb_path)
                    .with_context(|| format!("Failed to create thumb: {:?}", thumb_path))?;
                file.write_all(&thumb_data)
                    .with_context(|| format!("Failed to write thumb: {:?}", thumb_path))?;
            }
            Some(thumb_path)
        }
        None => None,
    };

    Ok(SavedImage {
        file_path,
        thumb_file_path,
        content_hash,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_svg() {
        let svg_data = b"<svg width=\"100\" height=\"100\"></svg>";
        assert_eq!(ImageFormat::detect(svg_data).unwrap(), ImageFormat::Svg);

        let svg_xml_data = b"<?xml version=\"1.0\"?><svg></svg>";
        assert_eq!(ImageFormat::detect(svg_xml_data).unwrap(), ImageFormat::Svg);
    }

    #[test]
    fn test_detect_formats() {
        // Test JPEG
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(ImageFormat::detect(&jpeg_data).unwrap(), ImageFormat::Jpeg);

        // Test PNG
        let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(ImageFormat::detect(&png_data).unwrap(), ImageFormat::Png);

        // Test GIF
        let gif_data = b"GIF89a\x00\x00";
        assert_eq!(ImageFormat::detect(gif_data).unwrap(), ImageFormat::Gif);
    }

    #[test]
    fn test_format_extensions() {
        assert_eq!(ImageFormat::Svg.extension(), "svg");
        assert_eq!(ImageFormat::Png.extension(), "png");
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
    }

    #[test]
    fn test_format_mime_types() {
        assert_eq!(ImageFormat::Svg.mime_type(), "image/svg+xml");
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
    }

    #[test]
    fn test_compute_content_hash() {
        let data = b"hello world";
        let hash = compute_content_hash(data);
        assert_eq!(hash.len(), 64);
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_compute_content_hash_deterministic() {
        let data = b"some image bytes";
        assert_eq!(compute_content_hash(data), compute_content_hash(data));
    }

    #[test]
    fn test_build_hash_based_path() {
        let base = std::path::Path::new("/uploads");
        let hash = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
        let path = build_hash_based_path(base, hash, "jpg").unwrap();
        assert_eq!(
            path,
            std::path::PathBuf::from(format!("/uploads/a1/b2/{}.jpg", hash))
        );
    }

    #[test]
    fn test_build_hash_based_path_short_hash() {
        let base = std::path::Path::new("/uploads");
        assert!(build_hash_based_path(base, "ab", "jpg").is_err());
    }

    #[test]
    fn test_build_thumb_path() {
        let original = std::path::PathBuf::from("/uploads/a1/b2/abc123.jpg");
        let thumb = build_thumb_path(&original).unwrap();
        assert_eq!(
            thumb,
            std::path::PathBuf::from("/uploads/a1/b2/abc123_thumb.jpg")
        );
    }

    #[test]
    fn test_generate_thumbnail_svg_returns_none() {
        let svg_data = b"<svg><rect/></svg>";
        let result = generate_thumbnail(svg_data, &ImageFormat::Svg, 1600).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_generate_thumbnail_gif_returns_none() {
        let gif_data = b"GIF89aXXXXXXXX";
        let result = generate_thumbnail(gif_data, &ImageFormat::Gif, 1600).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_save_with_dedup() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let data = b"dedup test data";

        let (path1, hash1) = save_with_dedup(data, temp_dir.path(), "bin").unwrap();
        let (path2, hash2) = save_with_dedup(data, temp_dir.path(), "bin").unwrap();

        assert_eq!(hash1, hash2);
        assert_eq!(path1, path2);
        assert!(path1.exists());
        assert_eq!(std::fs::read(&path1).unwrap(), data);
    }

    #[test]
    fn test_save_with_dedup_different_data() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let (path1, hash1) = save_with_dedup(b"data1", temp_dir.path(), "bin").unwrap();
        let (path2, hash2) = save_with_dedup(b"data2", temp_dir.path(), "bin").unwrap();

        assert_ne!(hash1, hash2);
        assert_ne!(path1, path2);
    }

    #[test]
    fn test_to_image_format() {
        assert!(ImageFormat::Jpeg.to_image_format().is_some());
        assert!(ImageFormat::Png.to_image_format().is_some());
        assert!(ImageFormat::Gif.to_image_format().is_some());
        assert!(ImageFormat::WebP.to_image_format().is_some());
        assert!(ImageFormat::Svg.to_image_format().is_none());
    }
}
