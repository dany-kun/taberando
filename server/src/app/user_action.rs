use std::fmt::Formatter;

use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::app::core::{Coordinates, Meal};

const DRAW_LUNCH_ACTION: &str = "lunch_action";
const DRAW_DINNER_ACTION: &str = "dinner_action";
const POSTPONE_ACTION: &str = "postpone_action";
const DELETE_ACTION: &str = "delete_action";
const ARCHIVE_ACTION: &str = "archive_action";
const ADD_ACTION: &str = "add_action";
const REFRESH_ACTION: &str = "refresh_action";

pub enum UserAction {
    Draw(Meal, Option<Coordinates>),
    Postpone(Option<Coordinates>),
    DeleteCurrent(Option<Coordinates>),
    ArchiveCurrent(Option<Coordinates>),
    Add,
    Refresh,
}

impl Serialize for UserAction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let relative_url = match self {
            UserAction::Draw(meal, coordinates) => {
                let path = match meal {
                    Meal::Lunch => DRAW_LUNCH_ACTION,
                    Meal::Dinner => DRAW_DINNER_ACTION,
                };
                path_with_coordinates(path, coordinates)
            }
            UserAction::Postpone(coordinates) => {
                path_with_coordinates(POSTPONE_ACTION, coordinates)
            }
            UserAction::DeleteCurrent(coordinates) => {
                path_with_coordinates(DELETE_ACTION, coordinates)
            }
            UserAction::ArchiveCurrent(coordinates) => {
                path_with_coordinates(ARCHIVE_ACTION, coordinates)
            }
            UserAction::Add => ADD_ACTION.to_string(),
            UserAction::Refresh => REFRESH_ACTION.to_string(),
        };
        serializer.serialize_str(relative_url.as_str())
    }
}

impl Coordinates {
    fn to_path(&self, path: &str) -> String {
        format!("{}?lat={}&long={}", path, self.latitude, self.longitude)
    }
}

fn path_with_coordinates(path: &str, coordinates: &Option<Coordinates>) -> String {
    coordinates
        .as_ref()
        .map_or(path.to_string(), |c| c.to_path(path))
}

struct UserActionVisitor;

impl<'de> Visitor<'de> for UserActionVisitor {
    type Value = UserAction;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "A string representing a user action as a relative url"
        )
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let base_url = reqwest::Url::parse("taberando://postback").unwrap();
        let url = base_url.join(v).map_err(|_e| {
            E::custom(format!("A valid relative url path was expected, got {}", v).as_str())
        })?;
        let coordinates = coordinates(&url);
        match url.path().trim_start_matches('/') {
            DRAW_LUNCH_ACTION => Ok(UserAction::Draw(Meal::Lunch, coordinates)),
            DRAW_DINNER_ACTION => Ok(UserAction::Draw(Meal::Dinner, coordinates)),
            POSTPONE_ACTION => Ok(UserAction::Postpone(coordinates)),
            DELETE_ACTION => Ok(UserAction::DeleteCurrent(coordinates)),
            ARCHIVE_ACTION => Ok(UserAction::ArchiveCurrent(coordinates)),
            ADD_ACTION => Ok(UserAction::Add),
            REFRESH_ACTION => Ok(UserAction::Refresh),
            v => Err(E::custom(format!("Unknown action value {}", v))),
        }
    }
}

fn coordinates(url: &reqwest::Url) -> Option<Coordinates> {
    let mut latitude = None;
    let mut longitude = None;
    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "lat" => {
                latitude = v.parse::<f32>().ok();
            }
            "long" => {
                longitude = v.parse::<f32>().ok();
            }
            _ => {}
        }
    }
    if let (Some(lat), Some(lng)) = (latitude, longitude) {
        Some(Coordinates {
            latitude: lat,
            longitude: lng,
        })
    } else {
        None
    }
}

impl<'de> Deserialize<'de> for UserAction {
    fn deserialize<D>(deserializer: D) -> Result<UserAction, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(UserActionVisitor)
    }
}
