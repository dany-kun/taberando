use std::collections::HashMap;
use std::fmt::Formatter;

use async_trait::async_trait;
use rand::seq::IteratorRandom;
use reqwest::Client;
use serde::de::{DeserializeOwned, Error, Visitor};

use crate::app::core::{Meal, Place};
use crate::gcp::constants::BASE_URL;
use crate::http::{HttpClient, HttpResult};

const CURRENT_DRAW_PATH: &str = "pending_shop";
const FIREBASE_API_V2_CURRENT_DRAW_KEY: &str = "current_draw";
const FIREBASE_API_V2_PLACES_KEY: &str = "places";
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

pub struct FirebaseApiV1 {
    client: reqwest::Client,
}

pub struct FirebaseApiV2 {
    client: reqwest::Client,
}

pub struct FirebaseApiComposite {
    apis: Vec<Box<dyn FirebaseApi + Sync>>,
}

impl FirebaseApiComposite {
    pub fn new(client: reqwest::Client) -> FirebaseApiComposite {
        FirebaseApiComposite {
            apis: vec![
                Box::new(FirebaseApiV2 {
                    client: client.clone(),
                }),
                Box::new(FirebaseApiV1 { client }),
            ],
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ApiV2Place {
    name: String,
    timeslot: Vec<Meal>,
}

impl serde::Serialize for Meal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let serialized = match self {
            Meal::Lunch => "昼",
            Meal::Dinner => "夜",
        };
        serializer.serialize_str(serialized)
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

    async fn get_current_draw(&self, jar: &Jar) -> HttpResult<Option<String>>;

    async fn draw(&self, jar: &Jar, meal: Meal) -> HttpResult<Option<String>>;

    async fn add_place(&self, jar: &Jar, place: Place, meal: Vec<Meal>) -> HttpResult<Place>;

    async fn remove_drawn_place(&self, jar: &Jar, place: Option<Place>) -> HttpResult<()>;

    async fn delete_place(&self, jar: &Jar, place: Place) -> HttpResult<Place>;

    fn firebase_url(&self, jar: &Jar, path: &str) -> String {
        format!("{}/{}/{}.json", BASE_URL, jar, path)
    }
}

impl FirebaseApiV1 {
    async fn make_json_request<T: DeserializeOwned, O: FnOnce(&Client) -> reqwest::RequestBuilder>(
        &self,
        to_request: O,
    ) -> HttpResult<T>
    where
        O: Send,
    {
        self.client.make_json_request(to_request).await
    }

    async fn get_list_of_places(
        &self,
        jar: &Jar,
        meal: Meal,
    ) -> HttpResult<HashMap<String, String>> {
        let result: HttpResult<Option<HashMap<String, String>>> = self
            .make_json_request(|client| client.get(self.firebase_url(jar, meal.into())))
            .await;
        // If not places [no entry in DB]; might return null as node is no longer existing;
        // default to empty map
        result.map(|places| places.unwrap_or_default())
    }
}

#[derive(Debug, serde::Deserialize, Clone)]
struct AppendedKey {
    #[serde(rename(deserialize = "name"))]
    key: String,
}

#[async_trait]
impl FirebaseApi for FirebaseApiComposite {
    async fn add_label(&self, jar: &Jar, label: &str) -> HttpResult<String> {
        let apis = self.apis.iter();
        for api in apis {
            api.add_label(jar, label).await?;
        }
        Ok(label.to_string())
    }

    async fn get_current_draw(&self, jar: &Jar) -> HttpResult<Option<String>> {
        let api = self.apis.first().unwrap();
        api.get_current_draw(jar).await
    }

    async fn draw(&self, jar: &Jar, meal: Meal) -> HttpResult<Option<String>> {
        // Just change state on the first API
        let api = self.apis.first().unwrap();
        api.draw(jar, meal).await
    }

    async fn add_place(&self, jar: &Jar, place: Place, meal: Vec<Meal>) -> HttpResult<Place> {
        let apis = self.apis.iter();
        for api in apis {
            api.add_place(jar, place.clone(), meal.clone()).await?;
        }
        Ok(place)
    }

    async fn remove_drawn_place(&self, jar: &Jar, place: Option<Place>) -> HttpResult<()> {
        let apis = self.apis.iter();
        for api in apis {
            api.remove_drawn_place(jar, place.clone()).await?;
        }
        Ok(())
    }

    async fn delete_place(&self, jar: &Jar, place: Place) -> HttpResult<Place> {
        let apis = self.apis.iter();
        for api in apis {
            api.delete_place(jar, place.clone()).await?;
        }
        Ok(place)
    }

    fn firebase_url(&self, _jar: &Jar, _path: &str) -> String {
        panic!("No firebase url to define on composite api")
    }
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

    async fn get_current_draw(&self, jar: &Jar) -> HttpResult<Option<String>> {
        // Get the current draw key
        let key: Option<String> = self
            .client
            .make_json_request(|client| client.get(self.firebase_url(jar, "current_draw")))
            .await?;
        // Get the current draw name from the key
        let place = match key {
            Some(key) => {
                let place: String = self
                    .client
                    .make_json_request(|client| {
                        client.get(self.firebase_url(
                            jar,
                            format!("{}/{}", FIREBASE_API_V2_PLACE_NAME_TABLE, key).as_str(),
                        ))
                    })
                    .await?;
                Some(place)
            }
            None => None,
        };
        Ok(place)
    }

    async fn draw(&self, _jar: &Jar, _meal: Meal) -> HttpResult<Option<String>> {
        todo!()
    }

    async fn add_place(&self, jar: &Jar, place: Place, meal: Vec<Meal>) -> HttpResult<Place> {
        let place_name = &place.name;
        let response: AppendedKey = self
            .client
            .make_json_request(|client| {
                client
                    .post(self.firebase_url(jar, FIREBASE_API_V2_PLACES_KEY))
                    .json::<ApiV2Place>(&ApiV2Place {
                        name: place_name.clone(),
                        timeslot: meal,
                    })
            })
            .await?;

        // Store the generated key to some indexing table
        // Should be done in a transaction + need monitoring..if this fails we corrupt our DB data...
        let added_place_key = response.key;
        let _place_name = self
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
            name: added_place_key,
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
        let place_name = &place.name;
        self.client
            .make_request(|client| {
                client.delete(self.firebase_url(
                    jar,
                    format!("{}/{}", FIREBASE_API_V2_PLACES_KEY, place_name).as_str(),
                ))
            })
            .await?;
        let _: HttpResult<serde_json::Value> = self
            .client
            .make_json_request(|client| {
                client.delete(self.firebase_url(
                    jar,
                    format!("{}/{}", FIREBASE_API_V2_PLACE_NAME_TABLE, place_name).as_str(),
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

#[async_trait]
impl FirebaseApi for FirebaseApiV1 {
    async fn add_label(&self, jar: &Jar, label: &str) -> HttpResult<String> {
        let _ = self
            .make_json_request(|client| client.put(self.firebase_url(jar, LABEL_PATH)).json(label))
            .await?;
        Ok(label.to_string())
    }

    async fn get_current_draw(&self, jar: &Jar) -> HttpResult<Option<String>> {
        self.make_json_request(|client| client.get(self.firebase_url(jar, CURRENT_DRAW_PATH)))
            .await
    }

    async fn draw(&self, jar: &Jar, meal: Meal) -> HttpResult<Option<String>> {
        let shops: HashMap<String, String> = self.get_list_of_places(jar, meal).await?;
        // Pick randomly a shop
        let shop = shops.values().choose(&mut rand::thread_rng());
        if let Some(picked) = shop {
            let _place: String = self
                .make_json_request(|client| {
                    client
                        .put(self.firebase_url(jar, CURRENT_DRAW_PATH))
                        .json(picked)
                })
                .await?;
        }
        Result::Ok(shop.map(|s| s.to_string()))
    }

    async fn add_place(&self, jar: &Jar, place: Place, meals: Vec<Meal>) -> HttpResult<Place> {
        let place_name = &place.clone().name;
        // TODO improve by running those futures concurrently
        for meal in meals {
            let _: HashMap<String, String> = self
                .make_json_request(|client| {
                    client
                        .post(self.firebase_url(jar, meal.into()))
                        .json::<String>(place_name)
                })
                .await?;
        }
        HttpResult::Ok(place)
    }

    async fn remove_drawn_place(&self, jar: &Jar, place: Option<Place>) -> HttpResult<()> {
        let remove_current = self
            .make_json_request(|client| client.delete(self.firebase_url(jar, CURRENT_DRAW_PATH)));
        match place {
            None => remove_current.await?,
            Some(place) => {
                if let Some(drawn_place) = self.get_current_draw(jar).await? {
                    if place.name == drawn_place {
                        remove_current.await?;
                    }
                }
            }
        }

        HttpResult::Ok(())
    }

    async fn delete_place(&self, jar: &Jar, place: Place) -> HttpResult<Place> {
        // As we are using a very bad data structure we need to loop on all "rows" to find the one to delete
        // Besides we are too lazy to import new crate and do it asynchronosuly or/and with a functional pattern
        // mapping meal to entries, filtering and folding into a list of keys to delete...
        let mut paths_to_delete: Vec<String> = Vec::new();
        for meal in vec![Meal::Lunch, Meal::Dinner] {
            let meal_path: &str = meal.clone().into();
            let places = self.get_list_of_places(jar, meal).await?;
            for (k, v) in places {
                if v == place.name {
                    paths_to_delete.push(format!("{}/{}", meal_path, k));
                }
            }
        }
        for path in paths_to_delete {
            self.make_json_request(|client| client.delete(self.firebase_url(jar, path.as_str())))
                .await?;
        }
        self.remove_drawn_place(jar, Some(place.clone())).await?;
        HttpResult::Ok(place)
    }
}
