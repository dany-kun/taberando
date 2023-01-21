use std::fmt::Debug;

use crate::app;
use serde::{Deserialize, Serialize};

use crate::line::bot::{EventSource, Postback};
use crate::line::json;

pub const DRAW_LUNCH_ACTION: &str = "lunch_action";
pub const DRAW_DINNER_ACTION: &str = "dinner_action";
pub const POSTPONE_ACTION: &str = "postpone_action";
pub const DELETE_ACTION: &str = "delete_action";
pub const ARCHIVE_ACTION: &str = "archive_action";
pub const ADD_ACTION: &str = "add_action";
pub const REFRESH_ACTION: &str = "refresh_action";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub(crate) to: String,
    pub(crate) messages: Vec<MessageContent>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Payload {
    pub(crate) destination: String,
    pub(crate) events: Vec<Event>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Event {
    #[serde(rename(deserialize = "type"))]
    pub(crate) event_type: String,
    pub(crate) mode: String,
    pub(crate) source: EventSource,
    pub(crate) postback: Option<Postback>,
    pub(crate) message: Option<MessageContent>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageContent {
    #[serde(rename(deserialize = "type", serialize = "type"))]
    pub(crate) message_type: String,
    pub(crate) text: Option<String>,
    pub(crate) latitude: Option<f32>,
    pub(crate) longitude: Option<f32>,
    #[serde(rename(serialize = "quickReply"))]
    #[serde(skip_deserializing)]
    quick_replies: Option<QuickReplyItems>,
}

#[derive(Debug, Serialize, Clone)]
struct QuickReplyItems {
    items: Vec<QuickReply>,
}

#[derive(Debug, Serialize, Clone)]
pub struct QuickReply {
    #[serde(rename(serialize = "type"))]
    pub(crate) quick_reply_type: String,
    #[serde(rename(serialize = "imageUrl"))]
    pub(crate) image_url: Option<String>,
    pub(crate) action: QuickReplyAction,
}

#[derive(Debug, Serialize, Clone)]
pub struct QuickReplyAction {
    #[serde(rename(serialize = "type"))]
    pub(crate) quick_reply_action_type: String,
    pub(crate) label: String,
    pub(crate) data: Option<String>,
    pub(crate) uri: Option<String>,
}

pub enum QuickReplyState {
    Idle,
    ActiveDraw,
    NoShops,
}

impl MessageContent {
    pub(crate) fn postback_quick_reply(
        label: &str,
        data: &str,
        icon: Option<String>,
    ) -> QuickReply {
        QuickReply {
            quick_reply_type: "action".to_string(),
            image_url: icon,
            action: QuickReplyAction {
                quick_reply_action_type: "postback".to_string(),
                label: label.to_string(),
                data: Some(data.to_string()),
                uri: None,
            },
        }
    }

    pub(crate) fn uri_quick_reply(label: &str, uri: &str, icon: Option<String>) -> QuickReply {
        QuickReply {
            quick_reply_type: "action".to_string(),
            image_url: icon,
            action: QuickReplyAction {
                quick_reply_action_type: "uri".to_string(),
                label: label.to_string(),
                data: None,
                uri: Some(uri.to_string()),
            },
        }
    }

    #[allow(dead_code)]
    pub(crate) fn location_quick_reply(label: &str, icon: Option<String>) -> QuickReply {
        QuickReply {
            quick_reply_type: "action".to_string(),
            image_url: icon,
            action: QuickReplyAction {
                quick_reply_action_type: "location".to_string(),
                label: label.to_string(),
                data: None,
                uri: None,
            },
        }
    }

    pub(crate) fn text(message: &str) -> MessageContent {
        MessageContent {
            message_type: "text".to_string(),
            text: Some(message.to_string()),
            quick_replies: None,
            latitude: None,
            longitude: None,
        }
    }

    pub fn with_quick_replies(
        &mut self,
        client: &app::core::Client,
        host: &str,
        quick_reply_state: QuickReplyState,
    ) -> MessageContent {
        let replies = match quick_reply_state {
            QuickReplyState::Idle => vec![
                client.add_place_quick_reply(host),
                // MessageContent::location_quick_reply("location", None),
                MessageContent::postback_quick_reply("🎲 昼", json::DRAW_LUNCH_ACTION, None),
                MessageContent::postback_quick_reply("🎲 夜", json::DRAW_DINNER_ACTION, None),
            ],
            QuickReplyState::ActiveDraw => vec![
                client.add_place_quick_reply(host),
                // MessageContent::location_quick_reply("location", None),
                MessageContent::postback_quick_reply("✓ 完", ARCHIVE_ACTION, None),
                MessageContent::postback_quick_reply("📅 延", POSTPONE_ACTION, None),
                MessageContent::postback_quick_reply("❌ 削", DELETE_ACTION, None),
            ],
            QuickReplyState::NoShops => vec![client.add_place_quick_reply(host)],
        };
        self.quick_replies = Some(QuickReplyItems { items: replies });
        self.clone()
    }

    pub(crate) fn error_message<E: Debug>(error: &E) -> MessageContent {
        MessageContent {
            message_type: "text".to_string(),
            text: Some(format!("Error {:?}", error)),
            quick_replies: None,
            latitude: None,
            longitude: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RichMenu {
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    #[serde(rename(deserialize = "richMenuId"))]
    id: Option<String>,
    size: Size,
    selected: bool,
    name: String,
    #[serde(rename(serialize = "chatBarText", deserialize = "chatBarText"))]
    chat_bar_text: String,
    areas: Vec<Area>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Size {
    width: i32,
    height: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Bound {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
enum Action {
    #[serde(rename(serialize = "postback", deserialize = "postback"))]
    Postback { data: String },
    #[serde(rename(serialize = "uri", deserialize = "uri"))]
    Uri { uri: String },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Area {
    bounds: Bound,
    action: Action,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RichMenus {
    #[serde(rename(deserialize = "richmenus"))]
    pub rich_menus: Vec<RichMenu>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RichMenuId {
    #[serde(rename(deserialize = "richMenuId"))]
    pub rich_menu_id: String,
}

#[derive(Serialize)]
pub struct WebHookPayload {
    pub endpoint: String,
}

#[cfg(test)]
pub mod fixtures {
    use super::*;

    pub fn rich_menu_fixture() -> RichMenu {
        RichMenu {
            id: None,
            size: Size {
                width: 2500,
                height: 843,
            },
            selected: false,
            name: String::from("test_menu"),
            chat_bar_text: String::from("chat bar text"),
            areas: vec![],
        }
    }
}
