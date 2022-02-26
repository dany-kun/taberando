use async_trait::async_trait;
use reqwest::Client;
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

#[async_trait]
pub trait HttpClient {
    type Request;
    type Client;

    async fn make_json_request<T: DeserializeOwned, O: FnOnce(&Self::Client) -> Self::Request>(
        &self,
        to_request: O,
    ) -> HttpResult<T>
    where
        O: Send;
}

#[async_trait]
impl HttpClient for Client {
    type Request = reqwest::RequestBuilder;
    type Client = reqwest::Client;

    async fn make_json_request<T: DeserializeOwned, O: FnOnce(&Client) -> Self::Request>(
        &self,
        to_request: O,
    ) -> HttpResult<T>
    where
        O: Send,
    {
        let response = to_request(self)
            .send()
            .await
            .map_err(|e| ApiError::Network { error: e })?;

        match response.error_for_status_ref() {
            Ok(_) => response
                .json()
                .await
                .map_err(|e| ApiError::JsonParsing { error: e }),
            Err(e) => {
                let message = response.text().await.map_err(|e| ApiError::Unknown {
                    message: format!("Could not decode response, got {:?}", e),
                })?;
                let status = e.status().ok_or(ApiError::Unknown {
                    message: format!("Could not decode status, got {:?}", e),
                })?;
                Err(ApiError::Http {
                    code: status.as_u16(),
                    message,
                })
            }
        }
    }
}
