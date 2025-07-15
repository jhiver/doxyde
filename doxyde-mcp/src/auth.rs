use anyhow::{Context, Result};
use reqwest::{Client, Url};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct AuthClient {
    client: Client,
    base_url: Url,
    credentials: AuthCredentials,
    session: Arc<RwLock<Option<String>>>,
}

#[derive(Debug, Clone)]
pub struct AuthCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
struct LoginRequest {
    username: String,
    password: String,
}

impl AuthClient {
    pub fn new(base_url: Url, credentials: AuthCredentials) -> Result<Self> {
        let client = Client::builder()
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::none()) // Don't follow redirects automatically
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url,
            credentials,
            session: Arc::new(RwLock::new(None)),
        })
    }

    pub async fn login(&self) -> Result<String> {
        let login_url = self
            .base_url
            .join(".login")
            .context("Failed to construct login URL")?;

        let response = self
            .client
            .post(login_url)
            .form(&LoginRequest {
                username: self.credentials.username.clone(),
                password: self.credentials.password.clone(),
            })
            .send()
            .await
            .context("Failed to send login request")?;

        // Accept both 2xx success and 303 redirect as successful login
        if !response.status().is_success() && response.status() != reqwest::StatusCode::SEE_OTHER {
            anyhow::bail!("Login failed with status: {}", response.status());
        }

        // Extract session from cookies
        let session_id = response
            .cookies()
            .find(|cookie| cookie.name() == "session_id")
            .map(|cookie| cookie.value().to_string())
            .context("No session cookie found in login response")?;

        // Store the session
        let mut session_guard = self.session.write().await;
        *session_guard = Some(session_id.clone());

        tracing::info!("Successfully authenticated with Doxyde");
        Ok(session_id)
    }

    pub async fn get_session(&self) -> Result<String> {
        let session_guard = self.session.read().await;
        if let Some(session) = session_guard.as_ref() {
            return Ok(session.clone());
        }
        drop(session_guard);

        // Need to login
        self.login().await
    }

    pub async fn invalidate_session(&self) {
        let mut session_guard = self.session.write().await;
        *session_guard = None;
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub fn get_base_url(&self) -> &Url {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_credentials_creation() {
        let creds = AuthCredentials {
            username: "test_user".to_string(),
            password: "test_pass".to_string(),
        };
        assert_eq!(creds.username, "test_user");
        assert_eq!(creds.password, "test_pass");
    }

    #[tokio::test]
    async fn test_auth_client_creation() {
        let base_url = Url::parse("http://localhost:3000").unwrap();
        let creds = AuthCredentials {
            username: "test_user".to_string(),
            password: "test_pass".to_string(),
        };

        let client = AuthClient::new(base_url.clone(), creds).unwrap();
        assert_eq!(client.get_base_url().as_str(), base_url.as_str());
    }
}
