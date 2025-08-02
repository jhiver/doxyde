use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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
}
