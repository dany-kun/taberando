use crate::app::jar::JarError;
use async_trait::async_trait;
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use serde::Deserialize;

pub(crate) type HttpResult<T> = std::result::Result<T, ApiError>;

#[derive(Debug, Deserialize, Clone)]
pub struct Empty {}

#[derive(Debug)]
pub enum ApiError {
    JsonParsing { error: reqwest::Error },
    Network { error: reqwest::Error },
    Http { code: u16, message: String },
    Unknown { message: String },
}

impl From<JarError> for ApiError {
    fn from(_value: JarError) -> Self {
        ApiError::Unknown {
            message: "JarError".to_string(),
        }
    }
}

#[async_trait]
pub trait HttpClient {
    type Request;
    type Client;

    async fn make_request<O: FnOnce(&Self::Client) -> Self::Request>(
        &self,
        to_request: O,
    ) -> HttpResult<Response>
    where
        O: Send;

    async fn make_json_request<T: DeserializeOwned, O: FnOnce(&Self::Client) -> Self::Request>(
        &self,
        to_request: O,
    ) -> HttpResult<T>
    where
        O: Send,
    {
        self.make_request(to_request)
            .await?
            .json()
            .await
            .map_err(|e| ApiError::JsonParsing { error: e })
    }
}

#[async_trait]
impl HttpClient for Client {
    type Request = reqwest::RequestBuilder;
    type Client = reqwest::Client;

    async fn make_request<O: FnOnce(&Client) -> Self::Request>(
        &self,
        to_request: O,
    ) -> HttpResult<Response>
    where
        O: Send,
    {
        let response = to_request(self)
            .send()
            .await
            .map_err(|e| ApiError::Network { error: e })?;

        match response.error_for_status_ref() {
            Ok(_res) => Ok(response),
            Err(e) => {
                let message = response.text().await.map_err(|e| ApiError::Unknown {
                    message: format!("Could not decode response, got {e:?}"),
                })?;
                let status = e.status().ok_or(ApiError::Unknown {
                    message: format!("Could not decode status, got {e:?}"),
                })?;
                Err(ApiError::Http {
                    code: status.as_u16(),
                    message,
                })
            }
        }
    }
}
