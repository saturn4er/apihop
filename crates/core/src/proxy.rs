use reqwest::Client;
use serde::{Serialize, de::DeserializeOwned};

#[derive(Debug)]
pub struct RemoteClient {
    client: Client,
    server_url: String,
    access_token: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Server error ({status}): {body}")]
    Server { status: u16, body: String },
}

impl RemoteClient {
    pub fn new(server_url: String, access_token: Option<String>) -> Self {
        Self {
            client: Client::new(),
            server_url,
            access_token,
        }
    }

    fn build_request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.server_url, path);
        let mut req = self.client.request(method, &url);
        if let Some(token) = &self.access_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        req
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ProxyError> {
        let resp = self.build_request(reqwest::Method::GET, path).send().await?;
        Self::handle_response(resp).await
    }

    pub async fn post<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T, ProxyError> {
        let resp = self.build_request(reqwest::Method::POST, path).json(body).send().await?;
        Self::handle_response(resp).await
    }

    pub async fn put<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T, ProxyError> {
        let resp = self.build_request(reqwest::Method::PUT, path).json(body).send().await?;
        Self::handle_response(resp).await
    }

    pub async fn delete(&self, path: &str) -> Result<(), ProxyError> {
        let resp = self.build_request(reqwest::Method::DELETE, path).send().await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ProxyError::Server { status, body });
        }
        Ok(())
    }

    async fn handle_response<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T, ProxyError> {
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ProxyError::Server { status, body });
        }
        Ok(resp.json().await?)
    }
}
