// TCP client for the external i18n translation service.
// Ported (near-verbatim) from yatoo.travel backend/src/services/i18n.rs.
// Protocol: 4-byte big-endian length prefix + JSON frame.

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::debug;

const MAX_FRAME_SIZE: u32 = 16 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum I18nError {
    #[error("i18n service connection failed: {0}")]
    Connect(std::io::Error),

    #[error("i18n service io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("i18n service error [{code}]: {message}")]
    Server { code: String, message: String },

    #[error("i18n protocol error: {0}")]
    Protocol(String),

    #[error("i18n frame too large: {0} bytes")]
    FrameTooLarge(u32),
}

impl I18nError {
    /// Whether this error is transient (the service was unreachable or the
    /// connection dropped) rather than a definitive rejection of the content.
    /// Transient failures must NOT be cached as `is_failed`, so they are retried
    /// freely instead of being gated behind the failure cooldown — important
    /// when the pre-warmer runs before the i18n service is ready.
    pub fn is_transient(&self) -> bool {
        matches!(self, I18nError::Connect(_) | I18nError::Io(_))
    }
}

#[derive(Clone, Debug)]
pub struct I18nClient {
    addr: String,
}

#[derive(Serialize)]
struct TranslateRequest {
    id: String,
    method: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_lang: Option<String>,
    target_lang: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
}

#[derive(Serialize)]
struct TranslateBatchRequest {
    id: String,
    method: String,
    separator: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_lang: Option<String>,
    target_lang: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
}

#[derive(Deserialize)]
struct TranslateResponse {
    #[allow(dead_code)]
    id: String,
    status: String,
    translated: Option<String>,
    cached: Option<bool>,
    error_code: Option<String>,
    error_message: Option<String>,
}

#[derive(Deserialize)]
struct TranslateBatchResponse {
    #[allow(dead_code)]
    id: String,
    status: String,
    #[allow(dead_code)]
    separator: Option<String>,
    translated: Option<String>,
    cached: Option<bool>,
    error_code: Option<String>,
    error_message: Option<String>,
}

#[derive(Debug)]
pub struct TranslateResult {
    pub translated: String,
    pub cached: bool,
}

impl I18nClient {
    pub fn new(addr: &str) -> Self {
        Self {
            addr: addr.to_string(),
        }
    }

    pub async fn translate(
        &self,
        content: &str,
        source_lang: Option<&str>,
        target_lang: &str,
        context: Option<&str>,
    ) -> Result<TranslateResult, I18nError> {
        let mut stream = TcpStream::connect(&self.addr)
            .await
            .map_err(I18nError::Connect)?;

        let req = TranslateRequest {
            id: uuid::Uuid::new_v4().to_string(),
            method: "translate".to_string(),
            content: content.to_string(),
            source_lang: source_lang.map(|s| s.to_string()),
            target_lang: target_lang.to_string(),
            context: context.map(|s| s.to_string()),
        };

        let json = serde_json::to_vec(&req)?;
        stream.write_u32(json.len() as u32).await?;
        stream.write_all(&json).await?;
        stream.flush().await?;

        let len = stream.read_u32().await?;
        if len > MAX_FRAME_SIZE {
            return Err(I18nError::FrameTooLarge(len));
        }
        let mut buf = vec![0u8; len as usize];
        stream.read_exact(&mut buf).await?;

        let resp: TranslateResponse = serde_json::from_slice(&buf)?;

        if resp.status == "ok" {
            let translated = resp
                .translated
                .ok_or_else(|| I18nError::Protocol("missing translated field".into()))?;
            debug!(
                target_lang,
                cached = resp.cached.unwrap_or(false),
                "i18n translation complete"
            );
            Ok(TranslateResult {
                translated,
                cached: resp.cached.unwrap_or(false),
            })
        } else {
            Err(I18nError::Server {
                code: resp.error_code.unwrap_or_else(|| "unknown".into()),
                message: resp.error_message.unwrap_or_else(|| "unknown error".into()),
            })
        }
    }

    pub async fn translate_batch(
        &self,
        items: &[&str],
        source_lang: Option<&str>,
        target_lang: &str,
        context: Option<&str>,
    ) -> Result<Vec<TranslateResult>, I18nError> {
        let mut stream = TcpStream::connect(&self.addr)
            .await
            .map_err(I18nError::Connect)?;

        let separator = uuid::Uuid::new_v4().to_string();
        let content = items.join(&format!("\n{separator}\n"));

        let req = TranslateBatchRequest {
            id: uuid::Uuid::new_v4().to_string(),
            method: "translate_batch".to_string(),
            separator: separator.clone(),
            content,
            source_lang: source_lang.map(|s| s.to_string()),
            target_lang: target_lang.to_string(),
            context: context.map(|s| s.to_string()),
        };

        let json = serde_json::to_vec(&req)?;
        stream.write_u32(json.len() as u32).await?;
        stream.write_all(&json).await?;
        stream.flush().await?;

        let len = stream.read_u32().await?;
        if len > MAX_FRAME_SIZE {
            return Err(I18nError::FrameTooLarge(len));
        }
        let mut buf = vec![0u8; len as usize];
        stream.read_exact(&mut buf).await?;

        let resp: TranslateBatchResponse = serde_json::from_slice(&buf)?;

        if resp.status == "ok" {
            let translated = resp
                .translated
                .ok_or_else(|| I18nError::Protocol("missing translated field".into()))?;

            let sep_pattern = format!("\n{separator}\n");
            let parts: Vec<&str> = translated.split(&sep_pattern).collect();

            if parts.len() != items.len() {
                return Err(I18nError::Protocol(format!(
                    "expected {} parts, got {}",
                    items.len(),
                    parts.len()
                )));
            }

            let cached = resp.cached.unwrap_or(false);
            debug!(
                target_lang,
                cached,
                count = parts.len(),
                "i18n batch translation complete"
            );

            Ok(parts
                .into_iter()
                .map(|p| TranslateResult {
                    translated: p.trim().to_string(),
                    cached,
                })
                .collect())
        } else {
            Err(I18nError::Server {
                code: resp.error_code.unwrap_or_else(|| "unknown".into()),
                message: resp.error_message.unwrap_or_else(|| "unknown error".into()),
            })
        }
    }
}
