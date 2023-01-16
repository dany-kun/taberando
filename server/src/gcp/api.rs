use std::fmt::Formatter;

use async_trait::async_trait;
use rand::seq::IteratorRandom;
use serde::de::{Error, Visitor};

use crate::app::core::{Meal, Place};
use crate::gcp::constants::BASE_URL;
use crate::http::{HttpClient, HttpResult};

const FIREBASE_API_V2_CURRENT_DRAW_KEY: &str = "current_draw";
const FIREBASE_API_V2_PLACES_KEY: &str = "places";
const FIREBASE_API_V2_SLOTS_KEY: &str = "timeslots";
const FIREBASE_API_V2_PLACE_NAME_TABLE: &str = "place_id_name";
const LABEL_PATH: &str = "label";

pub(crate) type Jar = String;

impl From<Meal> for &str {
    fn from(meal: Meal) -> Self {
        match meal {
            Meal::Lunch => "昼だけ",
            Meal::Dinner => "夜だけ",
        }
    }
}

pub struct FirebaseApiV2 {
    client: reqwest::Client,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ApiV2Place {
    name: String,
    timeslot: Vec<Meal>,
}

impl Meal {
    fn serialized(&self) -> &'static str {
        match self {
            Meal::Lunch => "昼",
            Meal::Dinner => "夜",
        }
    }
}

impl serde::Serialize for Meal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.serialized())
    }
}

struct MealVisitor;

impl<'de> Visitor<'de> for MealVisitor {
    type Value = Meal;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "A string representing a meal time slot")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match v {
            "昼" => Result::Ok(Meal::Lunch),
            "夜" => Result::Ok(Meal::Dinner),
            _ => Result::Err(E::custom(format!("Unknown meal value {}", v))),
        }
    }
}

impl<'de> serde::Deserialize<'de> for Meal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(MealVisitor)
    }
}

#[async_trait]
pub trait FirebaseApi {
    async fn add_label(&self, jar: &Jar, label: &str) -> HttpResult<String>;

    async fn get_current_draw(&self, jar: &Jar) -> HttpResult<Option<Place>>;

    async fn draw(&self, jar: &Jar, meal: Meal) -> HttpResult<Option<Place>>;

    async fn add_place(&self, jar: &Jar, place_name: &str, meal: Vec<Meal>) -> HttpResult<Place>;

    async fn remove_drawn_place(&self, jar: &Jar, place: Option<Place>) -> HttpResult<()>;

    async fn delete_place(&self, jar: &Jar, place: Place) -> HttpResult<Place>;

    fn firebase_url(&self, jar: &Jar, path: &str) -> String {
        format!("{}/{}/{}.json", BASE_URL, jar, path)
    }
}

#[derive(Debug, serde::Deserialize, Clone)]
struct AppendedKey {
    #[serde(rename(deserialize = "name"))]
    key: String,
}

#[async_trait]
impl FirebaseApi for FirebaseApiV2 {
    async fn add_label(&self, jar: &Jar, label: &str) -> HttpResult<String> {
        let _ = self
            .client
            .make_json_request(|client| client.put(self.firebase_url(jar, LABEL_PATH)).json(label))
            .await?;
        Ok(label.to_string())
    }

    async fn get_current_draw(&self, jar: &Jar) -> HttpResult<Option<Place>> {
        // Get the current draw key
        let key: Option<String> = self
            .client
            .make_json_request(|client| {
                client.get(self.firebase_url(jar, FIREBASE_API_V2_CURRENT_DRAW_KEY))
            })
            .await?;
        let current_place = if let Some(k) = key {
            let name = self.get_current_draw_name(jar, k.clone()).await?;
            Some(Place {
                name: name.unwrap_or_else(|| "Could not find place name".to_string()),
                key: k,
            })
        } else {
            None
        };
        Ok(current_place)
    }

    async fn draw(&self, jar: &Jar, meal: Meal) -> HttpResult<Option<Place>> {
        // Very un-efficient since we are retrieving the whole set of places for a meal...
        let place_response: serde_json::Value = self
            .client
            .make_json_request(|client| {
                client
                    .get(self.firebase_url(
                        jar,
                        format!("{}/{}", FIREBASE_API_V2_SLOTS_KEY, meal.serialized()).as_str(),
                    ))
                    .query(&[("shallow", "true")])
            })
            .await?;
        let place_keys = match place_response {
            serde_json::Value::Object(map) => map,
            _ => unreachable!(),
        };
        let maybe_drawn_place_key = place_keys
            .keys()
            .choose(&mut rand::thread_rng())
            .map(|v| v.to_string());
        if let Some(drawn_place_key) = &maybe_drawn_place_key {
            let _response: serde_json::Value = self
                .client
                .make_json_request(|client| {
                    client
                        .put(self.firebase_url(jar, FIREBASE_API_V2_CURRENT_DRAW_KEY))
                        .json(drawn_place_key)
                })
                .await?;
            let maybe_name = self
                .get_current_draw_name(jar, drawn_place_key.clone())
                .await?;
            return Ok(Some(Place {
                name: maybe_name.unwrap_or_default(),
                key: drawn_place_key.clone(),
            }));
        }
        Ok(None)
    }

    async fn add_place(&self, jar: &Jar, place_name: &str, meal: Vec<Meal>) -> HttpResult<Place> {
        let response: AppendedKey = self
            .client
            .make_json_request(|client| {
                client
                    .post(self.firebase_url(jar, FIREBASE_API_V2_PLACES_KEY))
                    .json::<ApiV2Place>(&ApiV2Place {
                        name: place_name.to_string(),
                        timeslot: meal.clone(),
                    })
            })
            .await?;

        // Store the timeslot to tables
        let _: Vec<serde_json::Value> = futures::future::try_join_all(meal.iter().map(|meal| {
            self.client.make_json_request(|client| {
                client
                    .put(
                        self.firebase_url(
                            jar,
                            format!(
                                "{}/{}/{}",
                                FIREBASE_API_V2_SLOTS_KEY,
                                meal.serialized(),
                                response.key
                            )
                            .as_str(),
                        ),
                    )
                    .json(&serde_json::Value::Bool(true))
            })
        }))
        .await?;

        // Store the generated key to some indexing table
        // Should be done in a transaction + need monitoring..if this fails we corrupt our DB data...
        let added_place_key = response.key;
        let _place_name: String = self
            .client
            .make_json_request(|client| {
                client
                    .put(
                        self.firebase_url(
                            jar,
                            format!("{}/{}", FIREBASE_API_V2_PLACE_NAME_TABLE, &added_place_key)
                                .as_str(),
                        ),
                    )
                    .json(place_name)
            })
            .await?;

        HttpResult::Ok(Place {
            name: place_name.to_string(),
            key: added_place_key,
        })
    }

    async fn remove_drawn_place(&self, jar: &Jar, _place: Option<Place>) -> HttpResult<()> {
        // TODO use the passed parameter
        if let Some(_drawn_place) = self.get_current_draw(jar).await? {
            self.client
                .make_json_request(|client| {
                    client.delete(self.firebase_url(jar, FIREBASE_API_V2_CURRENT_DRAW_KEY))
                })
                .await?;
        }
        HttpResult::Ok(())
    }

    // https://firebase.google.com/docs/database/rest/save-data#section-conditional-requests
    async fn delete_place(&self, jar: &Jar, place: Place) -> HttpResult<Place> {
        // Delete from places list
        self.client
            .make_request(|client| {
                client.delete(self.firebase_url(
                    jar,
                    format!("{}/{}", FIREBASE_API_V2_PLACES_KEY, place.key).as_str(),
                ))
            })
            .await?;
        // Delete from timeslots
        for meal in vec![Meal::Lunch, Meal::Dinner] {
            self.client
                .make_request(|client| {
                    client.delete(
                        self.firebase_url(
                            jar,
                            format!(
                                "{}/{}/{}",
                                FIREBASE_API_V2_SLOTS_KEY,
                                meal.serialized(),
                                &place.key
                            )
                            .as_str(),
                        ),
                    )
                })
                .await?;
        }

        // Delete from place names
        let _: HttpResult<serde_json::Value> = self
            .client
            .make_json_request(|client| {
                client.delete(self.firebase_url(
                    jar,
                    format!("{}/{}", FIREBASE_API_V2_PLACE_NAME_TABLE, place.key).as_str(),
                ))
            })
            .await;

        self.remove_drawn_place(jar, Some(place.clone())).await?;
        HttpResult::Ok(place.clone())
    }

    fn firebase_url(&self, jar: &Jar, path: &str) -> String {
        format!("{}/v2/{}/{}.json", BASE_URL, jar, path)
    }
}

impl FirebaseApiV2 {
    pub fn new(client: reqwest::Client) -> FirebaseApiV2 {
        FirebaseApiV2 { client }
    }

    async fn get_current_draw_name(
        &self,
        jar: &Jar,
        draw_key: String,
    ) -> HttpResult<Option<String>> {
        // Get the current draw name from the key
        let place: Option<String> = self
            .client
            .make_json_request(|client| {
                client.get(self.firebase_url(
                    jar,
                    format!("{}/{}", FIREBASE_API_V2_PLACE_NAME_TABLE, draw_key).as_str(),
                ))
            })
            .await?;
        Ok(place)
    }
}
