use serde::Deserialize;

use crate::app::core::{Client, Coordinates};
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

pub struct PostbackUrl(reqwest::Url);

impl From<reqwest::Url> for PostbackUrl {
    fn from(value: reqwest::Url) -> Self {
        PostbackUrl(value)
    }
}

impl Postback {
    pub fn data_url(&self) -> Result<PostbackUrl, Box<dyn std::error::Error>> {
        let base_url = reqwest::Url::parse("taberando://postback")?;
        base_url
            .join(self.data.as_str())
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
            .map(|p| p.into())
    }
}

impl PostbackUrl {
    pub(crate) fn path(&self) -> &str {
        self.0.path().trim_start_matches('/')
    }

    pub fn coordinates(&self) -> Option<Coordinates> {
        let mut latitude = None;
        let mut longitude = None;
        for (k, v) in self.0.query_pairs() {
            match k.as_ref() {
                "latitude" => {
                    latitude = v.parse::<f32>().ok();
                }
                "longitude" => {
                    longitude = v.parse::<f32>().ok();
                }
                _ => {}
            }
        }
        latitude.and_then(|lat| {
            longitude.map(|lng| Coordinates {
                latitude: lat,
                longitude: lng,
            })
        })
    }
}

impl EventSource {
    pub fn to_client(&self) -> Option<Client> {
        let user_id = self.user_id.as_ref();
        match self.source_type.as_str() {
            "user" => user_id.map(|id| Client::Line(LineChannel::User(id.to_string()))),
            "group" => self.group_id.as_ref().map(|id| {
                Client::Line(LineChannel::Group {
                    id: id.to_string(),
                    user_id: user_id.map(|user_id| user_id.to_string()),
                })
            }),
            "room" => self.room_id.as_ref().map(|id| {
                Client::Line(LineChannel::Room {
                    id: id.to_string(),
                    user_id: user_id.map(|user_id| user_id.to_string()),
                })
            }),
            _ => Option::None,
        }
    }
}

pub async fn setup(_client: &LineClient, _source: &EventSource) -> HttpResult<()> {
    // let menu_id = client.create_rich_menu(&RichMenu::default()).await?;
    // println!("{}", menu_id);
    // client.set_rich_menu(menu_id.as_str()).await?;
    Ok(())
}
