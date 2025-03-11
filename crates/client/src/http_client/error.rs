use reqwest::StatusCode;
use thiserror::Error;

#[allow(variant_size_differences)]
#[derive(Debug, Error)]
pub enum HttpClientError {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Invalid status code: {status}")]
    InvalidStatus { status: StatusCode },
}

pub type Result<T> = anyhow::Result<T, HttpClientError>;
