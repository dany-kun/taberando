use serde::{Deserialize, Serialize};

pub const DRAW_LUNCH_ACTION: &str = "lunch_action";
pub const DRAW_DINNER_ACTION: &str = "dinner_action";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RichMenu {
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
