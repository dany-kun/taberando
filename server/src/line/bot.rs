use serde::Deserialize;

use crate::app::core::Client;
use crate::http::HttpResult;
use crate::line::http::{LineChannel, LineClient};

#[derive(Debug, Deserialize, Clone)]
pub struct EventSource {
    #[serde(rename(deserialize = "type"))]
    source_type: String,
    #[serde(rename(deserialize = "userId"))]
    user_id: Option<String>,
    #[serde(rename(deserialize = "roomId"))]
    room_id: Option<String>,
    #[serde(rename(deserialize = "groupId"))]
    group_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Postback {
    pub(crate) data: String,
}

impl EventSource {
    pub fn to_client(&self) -> Option<Client> {
        match self.source_type.as_str() {
            "user" => self
                .user_id
                .clone()
                .map(|id| Client::Line(LineChannel::User(id))),
            "group" => self.group_id.clone().map(|id| {
                Client::Line(LineChannel::Group {
                    id,
                    user_id: self.user_id.clone(),
                })
            }),
            "room" => self.room_id.clone().map(|id| {
                Client::Line(LineChannel::Room {
                    id,
                    user_id: self.user_id.clone(),
                })
            }),
            _ => Option::None,
        }
    }
}

pub async fn setup(_client: &LineClient, _source: EventSource) -> HttpResult<()> {
    // let menu_id = client.create_rich_menu(&RichMenu::default()).await?;
    // println!("{}", menu_id);
    // client.set_rich_menu(menu_id.as_str()).await?;
    Ok(())
}
