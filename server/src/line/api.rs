use async_trait::async_trait;
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use reqwest::{Body, Client};

use serde::{Deserialize, Serialize};

use crate::http::{Empty, HttpClient, HttpResult};


use super::menu::*;

const BASE_LINE_URL: &str = "https://api.line.me/v2/bot";

#[derive(Debug, Deserialize, Clone)]
struct RichMenus {
    #[serde(rename(deserialize = "richmenus"))]
    rich_menus: Vec<RichMenu>,
}

#[derive(Debug, Deserialize, Clone)]
struct RichMenuId {
    #[serde(rename(deserialize = "richMenuId"))]
    rich_menu_id: String,
}

#[derive(Serialize)]
struct WebHookPayload {
    endpoint: String,
}

#[async_trait]
pub trait LineApi {
    async fn set_rich_menu(&self, rich_menu_id: &str, user: Option<&str>) -> HttpResult<Empty>;

    async fn get_rich_menus(&self) -> HttpResult<Vec<RichMenu>>;

    async fn create_rich_menu(&self, menu: &RichMenu, image_bytes: Vec<u8>) -> HttpResult<String>;

    async fn delete_rich_menu(&self, menu_id: &str) -> HttpResult<Empty>;

    async fn get_default_menu(&self, user_id: Option<&str>) -> HttpResult<String>;

    async fn update_line_webhook_url(&self, url: &str) -> HttpResult<Empty>;

    fn api_url(&self, path: &str) -> String {
        format!("{}/{}", BASE_LINE_URL, path)
    }
}

#[async_trait]
impl LineApi for Client {
    async fn set_rich_menu(&self, rich_menu_id: &str, user_id: Option<&str>) -> HttpResult<Empty> {
        let path = match user_id {
            Some(id) => format!("user/{}/richmenu/{}", id, rich_menu_id),
            None => format!("user/all/richmenu/{}", rich_menu_id),
        };
        self.make_json_request(|client| client.post(self.api_url(path.as_str())))
            .await
    }

    async fn get_rich_menus(&self) -> HttpResult<Vec<RichMenu>> {
        let menus: RichMenus = self
            .make_json_request(|client| client.get(self.api_url("richmenu/list")))
            .await?;
        Ok(menus.rich_menus)
    }

    async fn create_rich_menu(&self, _menu: &RichMenu, image: Vec<u8>) -> HttpResult<String> {
        let menu: RichMenuId = self
            .make_json_request(|client| client.post(self.api_url("richmenu")))
            .await?;

        let menu_id = menu.rich_menu_id;
        let menu_url = format!(
            "https://api-data.line.me/v2/bot/richmenu/{}/content",
            menu_id
        );
        let _: () = self
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
            client.delete(self.api_url(format!("richmenu/{}", menu_id).as_str()))
        })
        .await
    }

    async fn get_default_menu(&self, user_id: Option<&str>) -> HttpResult<String> {
        self.make_json_request(|client| match user_id {
            Some(id) => client.get(self.api_url(format!("user/{}/richmenu", id).as_str())),
            None => client.get(self.api_url("user/all/richmenu")),
        })
        .await
        .map(|m: RichMenuId| m.rich_menu_id)
    }

    async fn update_line_webhook_url(&self, url: &str) -> HttpResult<Empty> {
        let server_webhook_url = format!("{}/line/webhook", url);
        let payload = WebHookPayload {
            endpoint: server_webhook_url,
        };
        self.make_json_request(|client| {
            client
                .put(self.api_url("channel/webhook/endpoint"))
                .json(&payload)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_gets_menu_list() {
        let client = client();
        let menu = client.get_rich_menus().await;
        match menu {
            Ok(m) => println!("Got {} menus", m.len()),
            Err(e) => {
                println!("Got error {:?}", e)
            }
        }
        assert_eq!(2 + 2, 4);
    }

    fn client() -> Client {
        let string = std::fs::read_to_string("../tools/line_tunnel/line.token").unwrap();
        let client = http::get_line_client(Some(string));
        client
    }
}
