use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Response;

use crate::http::{Empty, HttpClient, HttpResult};
use crate::line::api::LineApi;
use crate::line::json::{Message, MessageContent};

pub(crate) type LineUserId = String;

#[derive(Debug)]
pub enum LineChannel {
    User(LineUserId),
    Room { id: String, user_id: Option<String> },
    Group { id: String, user_id: Option<String> },
}

#[derive(Clone)]
pub struct LineClient(reqwest::Client);

impl LineClient {
    pub fn new(line_token: &str) -> Self {
        let mut header_map = HeaderMap::new();

        let authorization_header = &*format!("Bearer {line_token}");
        let mut auth_value = HeaderValue::from_str(authorization_header).unwrap();
        auth_value.set_sensitive(true);
        header_map.append(AUTHORIZATION, auth_value);

        header_map.append(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        LineClient(
            reqwest::Client::builder()
                .default_headers(header_map)
                .connection_verbose(true)
                .build()
                .unwrap(),
        )
    }

    pub async fn send_to(&self, id: &str, message: MessageContent) -> HttpResult<Empty> {
        self.send_messages(&Message {
            to: id.to_string(),
            messages: [message].to_vec(),
        })
        .await
    }
}

#[async_trait]
impl HttpClient for LineClient {
    type Request = reqwest::RequestBuilder;
    type Client = reqwest::Client;

    async fn make_request<O: FnOnce(&Self::Client) -> Self::Request>(
        &self,
        to_request: O,
    ) -> HttpResult<Response>
    where
        O: Send,
    {
        self.0.make_request(to_request).await
    }
}
