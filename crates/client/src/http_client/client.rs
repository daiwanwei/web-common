use std::time::Duration;

use http::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client as ReqwestClient, Method};
use serde::{de::DeserializeOwned, Serialize};

use crate::http_client::{HttpClientError, Result};

#[derive(Debug, Clone)]
pub struct Client {
    inner: ReqwestClient,
    base_url: String,
    default_headers: HeaderMap,
}

#[derive(Debug, Default)]
pub struct ClientBuilder {
    timeout: Option<Duration>,
    headers: HeaderMap,
}

impl ClientBuilder {
    pub fn new() -> Self { Self { timeout: None, headers: HeaderMap::new() } }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn header(mut self, key: HeaderName, value: HeaderValue) -> Self {
        drop(self.headers.insert(key, value));
        self
    }

    pub fn build(self, base_url: impl Into<String>) -> Result<Client> {
        let mut builder = ReqwestClient::builder();

        if let Some(timeout) = self.timeout {
            builder = builder.timeout(timeout);
        } else {
            builder = builder.timeout(Duration::from_secs(30));
        }

        let client = builder.build()?;

        Ok(Client { inner: client, base_url: base_url.into(), default_headers: self.headers })
    }
}

impl Client {
    pub fn builder() -> ClientBuilder { ClientBuilder::new() }

    pub fn new(base_url: impl Into<String>) -> Result<Self> { ClientBuilder::new().build(base_url) }

    pub fn with_timeout(mut self, timeout: Duration) -> Result<Self> {
        self.inner = ReqwestClient::builder().timeout(timeout).build()?;
        Ok(self)
    }

    pub fn with_header(mut self, key: HeaderName, value: HeaderValue) -> Self {
        drop(self.default_headers.insert(key, value));
        self
    }

    pub async fn get<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.request::<T, ()>(Method::GET, path, None).await
    }

    pub async fn post<T, B>(&self, path: &str, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        self.request(Method::POST, path, Some(body)).await
    }

    pub async fn put<T, B>(&self, path: &str, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        self.request(Method::PUT, path, Some(body)).await
    }

    pub async fn delete<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.request::<T, ()>(Method::DELETE, path, None).await
    }

    async fn request<T, B>(&self, method: Method, path: &str, body: Option<&B>) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.inner.request(method, &url);

        // Add default headers
        for (key, value) in self.default_headers.iter() {
            if let Ok(value_str) = value.to_str() {
                request = request.header(key.as_str(), value_str);
            }
        }

        // Add body if present
        if let Some(body) = body {
            request = request.json(body);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(HttpClientError::InvalidStatus { status: response.status() });
        }

        let data = response.json().await?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};

    use super::*;

    #[derive(Deserialize)]
    struct TestResponse {
        pub message: String,
    }

    #[tokio::test]
    async fn test_get_request() -> Result<()> {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"message": "success"})))
            .mount(&mock_server)
            .await;

        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .header(HeaderName::from_static("user-agent"), HeaderValue::from_static("test-client"))
            .build(mock_server.uri())?;

        let response: TestResponse = client.get("/test").await?;

        assert_eq!(response.message, "success");
        Ok(())
    }
}
