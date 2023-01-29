use std::collections::HashMap;

use async_trait::async_trait;
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use reqwest::Body;

use crate::http::{ApiError, Empty, HttpClient, HttpResult};
use crate::line::http::LineClient;

use super::json::*;

const BASE_LINE_URL: &str = "https://api.line.me/v2/bot";

#[async_trait]
pub trait LineApi {
    async fn set_rich_menu(&self, rich_menu_id: &str, user: Option<&str>) -> HttpResult<Empty>;

    async fn set_rich_menu_alias(&self, rich_menu_id: &str, alias: &str) -> HttpResult<Empty>;

    async fn get_rich_menus(&self) -> HttpResult<Vec<RichMenu>>;

    async fn get_rich_menu_id_from_alias(&self, alias: &str) -> HttpResult<String>;

    async fn create_rich_menu(&self, menu: &RichMenu, image_bytes: Vec<u8>) -> HttpResult<String>;

    async fn delete_rich_menu(&self, menu_id: &str) -> HttpResult<Empty>;

    async fn get_default_menu(&self, user_id: Option<&str>) -> HttpResult<String>;

    async fn update_line_webhook_url(&self, url: &str) -> HttpResult<Empty>;

    async fn send_messages(&self, message: &Message) -> HttpResult<Empty>;

    fn api_url(path: &str) -> String {
        format!("{BASE_LINE_URL}/{path}")
    }

    async fn set_rich_menu_from_alias(
        &self,
        menu_alias: &str,
        user: Option<&str>,
    ) -> HttpResult<Empty> {
        let menu_id = self.get_rich_menu_id_from_alias(menu_alias).await?;
        self.set_rich_menu(menu_id.as_str(), user).await
    }
}

#[async_trait]
impl LineApi for LineClient {
    async fn set_rich_menu(&self, rich_menu_id: &str, user_id: Option<&str>) -> HttpResult<Empty> {
        let path = match user_id {
            Some(id) => format!("user/{id}/richmenu/{rich_menu_id}"),
            None => format!("user/all/richmenu/{rich_menu_id}"),
        };
        self.make_json_request(|client| client.post(Self::api_url(path.as_str())))
            .await
    }

    async fn set_rich_menu_alias(&self, rich_menu_id: &str, alias: &str) -> HttpResult<Empty> {
        self.make_json_request(|client| {
            client
                .post(Self::api_url("richmenu/alias"))
                .json(&HashMap::from([
                    ("richMenuId", rich_menu_id),
                    ("richMenuAliasId", alias),
                ]))
        })
        .await
    }

    async fn get_rich_menus(&self) -> HttpResult<Vec<RichMenu>> {
        let menus: RichMenus = self
            .make_json_request(|client| client.get(Self::api_url("richmenu/list")))
            .await?;
        Ok(menus.rich_menus)
    }

    async fn get_rich_menu_id_from_alias(&self, alias: &str) -> HttpResult<String> {
        let response: HashMap<String, String> = self
            .make_json_request(|client| {
                client.get(Self::api_url(&format!("richmenu/alias/{alias}")))
            })
            .await?;
        let menu_id = response.get("richMenuId").ok_or(ApiError::Unknown {
            message: "No menu id from alias".to_string(),
        })?;
        HttpResult::Ok(menu_id.to_string())
    }

    async fn create_rich_menu(&self, menu: &RichMenu, image: Vec<u8>) -> HttpResult<String> {
        let menu: RichMenuId = self
            .make_json_request(|client| client.post(Self::api_url("richmenu")).json(menu))
            .await?;

        let menu_id = menu.rich_menu_id;
        let menu_url = format!("https://api-data.line.me/v2/bot/richmenu/{menu_id}/content");
        let _: Empty = self
            .make_json_request(|client| {
                client
                    .post(menu_url)
                    .header(CONTENT_TYPE, HeaderValue::from_static("image/jpeg"))
                    .body(Body::from(image))
            })
            .await?;
        Ok(menu_id)
    }

    async fn delete_rich_menu(&self, menu_id: &str) -> HttpResult<Empty> {
        self.make_json_request(|client| {
            client.delete(Self::api_url(format!("richmenu/{menu_id}").as_str()))
        })
        .await
    }

    async fn get_default_menu(&self, user_id: Option<&str>) -> HttpResult<String> {
        self.make_json_request(|client| match user_id {
            Some(id) => client.get(Self::api_url(format!("user/{id}/richmenu").as_str())),
            None => client.get(Self::api_url("user/all/richmenu")),
        })
        .await
        .map(|m: RichMenuId| m.rich_menu_id)
    }

    async fn update_line_webhook_url(&self, url: &str) -> HttpResult<Empty> {
        let server_webhook_url = format!("{url}/line/webhook");
        let payload = WebHookPayload {
            endpoint: server_webhook_url,
        };
        self.make_json_request(|client| {
            client
                .put(Self::api_url("channel/webhook/endpoint"))
                .json(&payload)
        })
        .await
    }

    async fn send_messages(&self, message: &Message) -> HttpResult<Empty> {
        self.make_json_request(|client| client.post(Self::api_url("message/push")).json(message))
            .await
    }
}
