use std::fmt::Debug;

use async_trait::async_trait;
use rand::seq::IteratorRandom;

use serde_json::Value;

use crate::app::coordinates::Coordinates;
use crate::app::core::{Meal, Place};
use crate::app::jar::Jar;
use crate::gcp::constants::{
    BASE_URL, FIREBASE_API_V2_CURRENT_DRAW_KEY, FIREBASE_API_V2_PLACES_KEY,
    FIREBASE_API_V2_PLACE_COORDINATES_TABLE, FIREBASE_API_V2_PLACE_NAME_TABLE,
    FIREBASE_API_V2_SLOTS_KEY, LABEL_PATH,
};
use crate::gcp::http_api::FirebaseApiV2;
use crate::http::HttpResult;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ApiV2Place {
    pub(crate) name: String,
    timeslot: Vec<Meal>,
}

#[async_trait]
pub trait FirebaseApi {
    async fn add_label(&self, jar: &Jar, label: &str) -> HttpResult<String>;

    async fn get_current_draw(&self, jar: &Jar) -> HttpResult<Option<Place>>;

    async fn draw(
        &self,
        jar: &Jar,
        meal: &Meal,
        coordinates: &Option<Coordinates>,
    ) -> HttpResult<Option<Place>>;

    async fn add_place(&self, jar: &Jar, place_name: &str, meal: &[Meal]) -> HttpResult<Place>;

    async fn set_place_coordinates(
        &self,
        jar: &Jar,
        place: &Place,
        coordinates: &Coordinates,
    ) -> HttpResult<()>;

    async fn remove_drawn_place(&self, jar: &Jar, place: Option<&Place>) -> HttpResult<()>;

    async fn delete_place(&self, jar: &Jar, place: &Place) -> HttpResult<Place>;

    fn firebase_url(&self, jar: &Jar, path: &str) -> String {
        format!("{BASE_URL}/{jar}/{path}.json")
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
            .make_json_request(|client| client.put(self.firebase_url(jar, LABEL_PATH)).json(label))
            .await?;
        Ok(label.to_string())
    }

    async fn get_current_draw(&self, jar: &Jar) -> HttpResult<Option<Place>> {
        // Get the current draw key
        let key: Option<String> = self
            .make_json_request(|client| {
                client.get(self.firebase_url(jar, FIREBASE_API_V2_CURRENT_DRAW_KEY))
            })
            .await?;
        let current_place = if let Some(k) = key {
            let name = self.get_current_draw_name(jar, &k).await?;
            Some(Place {
                name: name.unwrap_or_else(|| "Could not find place name".to_string()),
                key: k,
            })
        } else {
            None
        };
        Ok(current_place)
    }

    async fn draw(
        &self,
        jar: &Jar,
        meal: &Meal,
        coordinates: &Option<Coordinates>,
    ) -> HttpResult<Option<Place>> {
        let places = self.get_list_of_places_keys(jar, meal).await?;
        let maybe_drawn_place_key = match places {
            None => None,
            Some(meal_places) => {
                let place_keys: Vec<String> = match coordinates {
                    None => meal_places.keys().map(|k| k.to_string()).collect(),
                    Some(origin) => self.find_close_places(jar, meal_places, origin).await?,
                };
                place_keys
                    .iter()
                    .choose(&mut rand::thread_rng())
                    .map(|v| v.to_string())
            }
        };

        if let Some(drawn_place_key) = &maybe_drawn_place_key {
            self.update_current_draw(jar, drawn_place_key).await?;
            let maybe_name = self.get_current_draw_name(jar, drawn_place_key).await?;
            return Ok(Some(Place {
                name: maybe_name.unwrap_or_default(),
                key: drawn_place_key.to_string(),
            }));
        }
        Ok(None)
    }

    async fn add_place(&self, jar: &Jar, place_name: &str, meals: &[Meal]) -> HttpResult<Place> {
        let response: AppendedKey = self
            .make_json_request(|client| {
                client
                    .post(self.firebase_url(jar, FIREBASE_API_V2_PLACES_KEY))
                    .json::<ApiV2Place>(&ApiV2Place {
                        name: place_name.to_string(),
                        timeslot: meals.to_vec(),
                    })
            })
            .await?;

        // Store the timeslot to tables
        let _: Vec<Value> = futures::future::try_join_all(meals.iter().map(|meal| {
            self.make_json_request(|client| {
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
                    .json(&Value::Bool(true))
            })
        }))
        .await?;

        // Store the generated key to some indexing table
        // Should be done in a transaction + need monitoring..if this fails we corrupt our DB data...
        let added_place_key = response.key;
        let _place_name: String = self
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

        Ok(Place {
            name: place_name.to_string(),
            key: added_place_key,
        })
    }

    async fn set_place_coordinates(
        &self,
        jar: &Jar,
        place: &Place,
        coordinates: &Coordinates,
    ) -> HttpResult<()> {
        self.make_json_request(|client| {
            client
                .put(self.firebase_url(
                    jar,
                    format!("{}/{}", FIREBASE_API_V2_PLACE_COORDINATES_TABLE, place.key).as_str(),
                ))
                .json(coordinates)
        })
        .await?;
        Ok(())
    }

    async fn remove_drawn_place(&self, jar: &Jar, _place: Option<&Place>) -> HttpResult<()> {
        // TODO use the passed parameter
        if let Some(_drawn_place) = self.get_current_draw(jar).await? {
            self.make_json_request(|client| {
                client.delete(self.firebase_url(jar, FIREBASE_API_V2_CURRENT_DRAW_KEY))
            })
            .await?;
        }
        Ok(())
    }

    // https://firebase.google.com/docs/database/rest/save-data#section-conditional-requests
    async fn delete_place(&self, jar: &Jar, place: &Place) -> HttpResult<Place> {
        let lunch = format!("{}/{}", FIREBASE_API_V2_SLOTS_KEY, Meal::Lunch.serialized());
        let dinner = format!(
            "{}/{}",
            FIREBASE_API_V2_SLOTS_KEY,
            Meal::Dinner.serialized()
        );
        let buckets = vec![
            FIREBASE_API_V2_PLACES_KEY,
            lunch.as_str(),
            dinner.as_str(),
            FIREBASE_API_V2_PLACE_NAME_TABLE,
            FIREBASE_API_V2_PLACE_COORDINATES_TABLE,
        ];

        for bucket in buckets {
            self.make_request(|client| {
                client.delete(self.firebase_url(jar, format!("{}/{}", bucket, &place.key).as_str()))
            })
            .await?;
        }

        self.remove_drawn_place(jar, Some(place)).await?;
        Ok(place.clone())
    }

    fn firebase_url(&self, jar: &Jar, path: &str) -> String {
        format!("{BASE_URL}/v2/{jar}/{path}.json")
    }
}
