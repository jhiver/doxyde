// Doxyde - A modern, AI-native CMS built with Rust
// Copyright (C) 2025 Doxyde Project Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use image::GenericImageView;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Magic bytes for common image formats
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
    Webp,
    Svg,
}

impl ImageFormat {
    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::Gif => "gif",
            ImageFormat::Webp => "webp",
            ImageFormat::Svg => "svg",
        }
    }

    /// Get the MIME type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png => "image/png",
            ImageFormat::Gif => "image/gif",
            ImageFormat::Webp => "image/webp",
            ImageFormat::Svg => "image/svg+xml",
        }
    }

    /// Detect format from file content
    pub fn detect(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(anyhow::anyhow!("File too small to determine format"));
        }

        if data.starts_with(JPEG_MAGIC) {
            Ok(ImageFormat::Jpeg)
        } else if data.starts_with(PNG_MAGIC) {
            Ok(ImageFormat::Png)
        } else if data.starts_with(GIF_MAGIC) {
            Ok(ImageFormat::Gif)
        } else if data.starts_with(WEBP_MAGIC) && data.len() > 12 && &data[8..12] == b"WEBP" {
            Ok(ImageFormat::Webp)
        } else if data.starts_with(SVG_MAGIC) || data.starts_with(SVG_MAGIC_ALT) {
            Ok(ImageFormat::Svg)
        } else {
            Err(anyhow::anyhow!("Unsupported image format"))
        }
    }
}

/// Image metadata extracted from uploaded files
#[derive(Debug, Clone)]
pub struct ImageMetadata {
    pub format: ImageFormat,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub size: usize,
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

/// Generate a unique filename while preserving the original extension
pub fn generate_unique_filename(original_name: &str) -> String {
    let uuid = Uuid::new_v4();
    let extension = Path::new(original_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("bin");

    format!("{}.{}", uuid, extension)
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

/// Extract metadata from image data
pub fn extract_image_metadata(data: &[u8]) -> Result<ImageMetadata> {
    let format = ImageFormat::detect(data)?;
    let size = data.len();

    // Extract dimensions based on format
    let (width, height) = match format {
        ImageFormat::Svg => (None, None), // SVG dimensions are not fixed
        _ => {
            // Use the image crate to decode and get dimensions
            match image::load_from_memory(data) {
                Ok(img) => {
                    let dimensions = img.dimensions();
                    (Some(dimensions.0), Some(dimensions.1))
                }
                Err(_) => (None, None), // If we can't decode, just skip dimensions
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

/// Sanitize a slug to ensure it's URL-safe
pub fn sanitize_slug(slug: &str) -> String {
    let cleaned = slug
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c.to_lowercase().to_string()
            } else if c == '_' || c.is_whitespace() {
                "-".to_string()
            } else {
                "".to_string()
            }
        })
        .collect::<String>();

    // Remove multiple consecutive dashes and trim dashes from start/end
    let parts: Vec<&str> = cleaned.split('-').filter(|s| !s.is_empty()).collect();

    parts.join("-")
}

/// List of dangerous executable extensions that should be blocked
const DANGEROUS_EXTENSIONS: &[&str] = &[
    "exe", "bat", "cmd", "com", "pif", "scr", "vbs", "js", "jar", "msi",
    "app", "deb", "rpm", "dmg", "pkg", "run", "sh", "bash", "csh", "ksh",
    "ps1", "psm1", "psd1", "ps1xml", "psc1", "pssc", "cdxml", "clixml",
    "pl", "py", "rb", "php", "asp", "aspx", "jsp", "cgi", "htm", "html",
    "hta", "htaccess", "htpasswd", "ini", "inf", "reg", "scf", "url",
    "vb", "vba", "vbe", "vbs", "ws", "wsf", "wsh", "wsc", "sct",
];

/// Check if a filename has a dangerous extension or double extension
pub fn is_dangerous_filename(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    
    // Get all extensions (for detecting double extensions)
    let parts: Vec<&str> = lower.split('.').collect();
    if parts.len() < 2 {
        // No extension at all
        return false;
    }
    
    // Check each extension part
    for i in 1..parts.len() {
        let ext = parts[i];
        if DANGEROUS_EXTENSIONS.contains(&ext) {
            return true;
        }
    }
    
    // Check for specific dangerous double extension patterns
    // e.g., .php.jpg, .exe.txt
    if parts.len() > 2 {
        // Has double extension
        for i in 1..parts.len() - 1 {
            if DANGEROUS_EXTENSIONS.contains(&parts[i]) {
                // Dangerous extension hidden by another extension
                return true;
            }
        }
    }
    
    false
}

/// Validate that a filename is safe for upload
pub fn validate_upload_filename(filename: &str) -> Result<()> {
    if filename.is_empty() {
        return Err(anyhow!("Filename cannot be empty"));
    }
    
    if filename.len() > 255 {
        return Err(anyhow!("Filename too long"));
    }
    
    // Check for dangerous characters
    if filename.contains('\0') || filename.contains('/') || filename.contains('\\') {
        return Err(anyhow!("Filename contains invalid characters"));
    }
    
    // Check for dangerous extensions
    if is_dangerous_filename(filename) {
        return Err(anyhow!("File type not allowed for security reasons"));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_format_detection() {
        // Test JPEG - need at least 8 bytes
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        assert_eq!(ImageFormat::detect(&jpeg_data).unwrap(), ImageFormat::Jpeg);

        // Test PNG
        let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(ImageFormat::detect(&png_data).unwrap(), ImageFormat::Png);

        // Test GIF - need at least 8 bytes
        let gif_data = b"GIF89aXX".to_vec();
        assert_eq!(ImageFormat::detect(&gif_data).unwrap(), ImageFormat::Gif);

        // Test unsupported format
        let invalid_data = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert!(ImageFormat::detect(&invalid_data).is_err());

        // Test too small file
        let small_data = vec![0xFF, 0xD8];
        assert!(ImageFormat::detect(&small_data).is_err());
    }

    #[test]
    fn test_generate_unique_filename() {
        let filename1 = generate_unique_filename("photo.jpg");
        let filename2 = generate_unique_filename("photo.jpg");

        // Should have .jpg extension
        assert!(filename1.ends_with(".jpg"));
        assert!(filename2.ends_with(".jpg"));

        // Should be different
        assert_ne!(filename1, filename2);

        // Test with no extension
        let filename3 = generate_unique_filename("photo");
        assert!(filename3.ends_with(".bin"));
    }

    #[test]
    fn test_sanitize_slug() {
        assert_eq!(sanitize_slug("Hello World!"), "hello-world");
        assert_eq!(sanitize_slug("test@#$%image"), "testimage");
        assert_eq!(sanitize_slug("___test___"), "test");
        assert_eq!(sanitize_slug("CamelCase"), "camelcase");
        assert_eq!(sanitize_slug("multiple---dashes"), "multiple-dashes");
        assert_eq!(sanitize_slug("123-numbers"), "123-numbers");
        assert_eq!(sanitize_slug("test - image"), "test-image");
        assert_eq!(sanitize_slug("test_image"), "test-image");
    }

    #[test]
    fn test_image_format_extensions_and_mime_types() {
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");

        assert_eq!(ImageFormat::Png.extension(), "png");
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");

        assert_eq!(ImageFormat::Gif.extension(), "gif");
        assert_eq!(ImageFormat::Gif.mime_type(), "image/gif");

        assert_eq!(ImageFormat::Webp.extension(), "webp");
        assert_eq!(ImageFormat::Webp.mime_type(), "image/webp");

        assert_eq!(ImageFormat::Svg.extension(), "svg");
        assert_eq!(ImageFormat::Svg.mime_type(), "image/svg+xml");
    }

    #[test]
    fn test_webp_detection() {
        // Valid WebP file header - needs to be > 12 bytes
        let mut webp_data = b"RIFF".to_vec();
        webp_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // file size (placeholder)
        webp_data.extend_from_slice(b"WEBP");
        webp_data.extend_from_slice(&[0x00]); // Extra byte to make it > 12 bytes
        assert_eq!(ImageFormat::detect(&webp_data).unwrap(), ImageFormat::Webp);

        // Invalid RIFF file (not WebP)
        let mut riff_data = b"RIFF".to_vec();
        riff_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        riff_data.extend_from_slice(b"WAVE"); // WAV file, not WebP
        riff_data.extend_from_slice(&[0x00]); // Extra byte to make it > 12 bytes
        assert!(ImageFormat::detect(&riff_data).is_err());
    }

    #[test]
    fn test_svg_detection() {
        // SVG with direct tag
        let svg_data1 = b"<svg xmlns=\"http://www.w3.org/2000/svg\">".to_vec();
        assert_eq!(ImageFormat::detect(&svg_data1).unwrap(), ImageFormat::Svg);

        // SVG with XML declaration
        let svg_data2 = b"<?xml version=\"1.0\"?>".to_vec();
        assert_eq!(ImageFormat::detect(&svg_data2).unwrap(), ImageFormat::Svg);
    }

    #[test]
    fn test_create_upload_directory() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();
        let date = DateTime::parse_from_rfc3339("2024-03-15T10:30:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let upload_dir = create_upload_directory(base_path, date).unwrap();

        assert!(upload_dir.exists());
        assert_eq!(upload_dir.file_name().unwrap(), "15");
        assert_eq!(upload_dir.parent().unwrap().file_name().unwrap(), "03");
        assert_eq!(
            upload_dir
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .file_name()
                .unwrap(),
            "2024"
        );
    }

    #[test]
    fn test_save_upload() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let data = b"test image data";
        let filename = "test-image.jpg";

        let saved_path = save_upload(data, temp_dir.path(), filename).unwrap();

        assert!(saved_path.exists());
        assert_eq!(saved_path.file_name().unwrap(), filename);

        // Verify content
        let saved_data = fs::read(&saved_path).unwrap();
        assert_eq!(saved_data, data);
    }

    #[test]
    fn test_extract_image_metadata() {
        // Create a minimal valid PNG
        let mut png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        // Add IHDR chunk (required for PNG)
        png_data.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x0D, // chunk length
            b'I', b'H', b'D', b'R', // chunk type
            0x00, 0x00, 0x00, 0x10, // width = 16
            0x00, 0x00, 0x00, 0x10, // height = 16
            0x08, 0x02, 0x00, 0x00, 0x00, // bit depth, color type, etc.
        ]);

        let metadata = extract_image_metadata(&png_data).unwrap();
        assert_eq!(metadata.format, ImageFormat::Png);
        assert_eq!(metadata.size, png_data.len());
        // Dimensions might be None if image crate can't decode our minimal PNG

        // Test SVG (no dimensions)
        let svg_data = b"<svg><rect/></svg>".to_vec();
        let svg_metadata = extract_image_metadata(&svg_data).unwrap();
        assert_eq!(svg_metadata.format, ImageFormat::Svg);
        assert_eq!(svg_metadata.width, None);
        assert_eq!(svg_metadata.height, None);
    }
    
    #[test]
    fn test_is_dangerous_filename() {
        // Test dangerous single extensions
        assert!(is_dangerous_filename("malware.exe"));
        assert!(is_dangerous_filename("script.bat"));
        assert!(is_dangerous_filename("payload.php"));
        assert!(is_dangerous_filename("shell.sh"));
        assert!(is_dangerous_filename("UPPERCASE.EXE"));
        
        // Test dangerous double extensions
        assert!(is_dangerous_filename("backdoor.php.jpg"));
        assert!(is_dangerous_filename("virus.exe.txt"));
        assert!(is_dangerous_filename("script.asp.png"));
        assert!(is_dangerous_filename("payload.jsp.gif"));
        
        // Test safe files
        assert!(!is_dangerous_filename("image.jpg"));
        assert!(!is_dangerous_filename("document.pdf"));
        assert!(!is_dangerous_filename("photo.png"));
        assert!(!is_dangerous_filename("archive.zip"));
        assert!(!is_dangerous_filename("no_extension"));
        
        // Test edge cases
        assert!(!is_dangerous_filename("file.execute")); // Not in list
        assert!(is_dangerous_filename("file.jpg.exe")); // Dangerous at end
        assert!(is_dangerous_filename("file.exe.jpg.php")); // Multiple dangerous
    }
    
    #[test]
    fn test_validate_upload_filename() {
        // Valid filenames
        assert!(validate_upload_filename("image.jpg").is_ok());
        assert!(validate_upload_filename("document.pdf").is_ok());
        assert!(validate_upload_filename("my-file_123.png").is_ok());
        
        // Invalid filenames
        assert!(validate_upload_filename("").is_err());
        assert!(validate_upload_filename("a".repeat(256).as_str()).is_err());
        assert!(validate_upload_filename("file\0name.jpg").is_err());
        assert!(validate_upload_filename("../../../etc/passwd").is_err());
        assert!(validate_upload_filename("file\\path.jpg").is_err());
        
        // Dangerous extensions
        assert!(validate_upload_filename("virus.exe").is_err());
        assert!(validate_upload_filename("backdoor.php.jpg").is_err());
        
        // Error messages
        let result = validate_upload_filename("script.sh");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("security reasons"));
    }
}
