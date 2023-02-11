use std::collections::HashMap;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::app::coordinates::Coordinates;
use crate::app::core::{Meal, Place};
use crate::app::jar::Jar;
use crate::gcp::api::ApiV2Place;
use crate::gcp::api::FirebaseApi;
use crate::gcp::constants::BASE_URL;
use crate::gcp::constants::CLOSE_PLACE_RADIUS_METER;
use crate::gcp::constants::FIREBASE_API_V2_CURRENT_DRAW_KEY;
use crate::gcp::constants::FIREBASE_API_V2_PLACES_KEY;
use crate::gcp::constants::FIREBASE_API_V2_PLACE_COORDINATES_TABLE;
use crate::gcp::constants::FIREBASE_API_V2_PLACE_NAME_TABLE;
use crate::gcp::constants::FIREBASE_API_V2_SLOTS_KEY;
use crate::gcp::oauth;
use crate::http::{ApiError, HttpClient, HttpResult};

pub struct FirebaseApiV2 {
    client: Client,
}

impl FirebaseApiV2 {
    pub async fn default() -> Self {
        Self::new(Self::get_firebase_client().await)
    }

    pub fn new(client: Client) -> Self {
        FirebaseApiV2 { client }
    }

    async fn get_firebase_client() -> Client {
        let oauth = oauth::get_oauth_token().await.unwrap();
        let _ = env_logger::try_init();
        let mut header_map = HeaderMap::new();

        let authorization_header = &*format!("Bearer {}", oauth.token);
        let mut auth_value = HeaderValue::from_str(authorization_header).unwrap();
        auth_value.set_sensitive(true);
        header_map.append(AUTHORIZATION, auth_value);

        header_map.append(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        Client::builder()
            .default_headers(header_map)
            .connection_verbose(true)
            .build()
            .unwrap()
    }

    pub async fn get_current_draw_name(
        &self,
        jar: &Jar,
        draw_key: &str,
    ) -> HttpResult<Option<String>> {
        // Get the current draw name from the key
        let place: Option<String> = self
            .make_json_request(|client| {
                client.get(self.firebase_url(
                    jar,
                    format!("{FIREBASE_API_V2_PLACE_NAME_TABLE}/{draw_key}").as_str(),
                ))
            })
            .await?;
        Ok(place)
    }

    pub async fn get_all_places(&self, jar: &Jar) -> HttpResult<Vec<Place>> {
        let places: HashMap<String, ApiV2Place> = self
            .make_json_request(|client| {
                client.get(self.firebase_url(jar, FIREBASE_API_V2_PLACES_KEY))
            })
            .await?;
        Ok(places
            .iter()
            .map(|(key, place)| Place {
                key: key.clone(),
                name: place.name.clone(),
            })
            .collect())
    }

    pub async fn update_current_draw(&self, jar: &Jar, drawn_place_key: &str) -> HttpResult<()> {
        let _: Value = self
            .make_json_request(|client| {
                client
                    .put(self.firebase_url(jar, FIREBASE_API_V2_CURRENT_DRAW_KEY))
                    .json(drawn_place_key)
            })
            .await?;
        Ok(())
    }

    pub(crate) async fn get_list_of_places_keys(
        &self,
        jar: &Jar,
        meal: &Meal,
    ) -> HttpResult<Option<HashMap<String, Value>>> {
        // Very un-efficient since we are retrieving the whole set of places for a meal...
        let place_response: Option<HashMap<String, Value>> = self
            .make_json_request(|client| {
                client
                    .get(self.firebase_url(
                        jar,
                        format!("{}/{}", FIREBASE_API_V2_SLOTS_KEY, meal.serialized()).as_str(),
                    ))
                    .query(&[("shallow", "true")])
            })
            .await?;
        Ok(place_response)
    }

    pub(crate) async fn find_close_places(
        &self,
        jar: &Jar,
        meal_places: HashMap<String, Value>,
        origin: &Coordinates,
    ) -> HttpResult<Vec<String>> {
        // First get all places with coordinates....(not efficient...)
        let located_places = self
            .make_json_request::<HashMap<String, Coordinates>, _>(|client| {
                client.get(self.firebase_url(jar, FIREBASE_API_V2_PLACE_COORDINATES_TABLE))
            })
            .await?;
        // Filter all places with coordinates close enough
        let closed_places = located_places
            .into_iter()
            .filter_map(|(key, c)| {
                (c.distance(origin) <= CLOSE_PLACE_RADIUS_METER && meal_places.contains_key(&key))
                    .then_some(key)
            })
            .into_iter()
            .collect();
        Ok(closed_places)
    }

    pub async fn get_all_groups(&self) -> HttpResult<Vec<Jar>> {
        self.make_json_request::<HashMap<String, Value>, _>(|client| {
            client
                .get(format!("{BASE_URL}/v2.json"))
                .query(&[("shallow", "true")])
        })
        .await
        .map(|raw| raw.keys().map(|k| Jar::new(k)).collect::<Vec<_>>())
    }

    pub(crate) async fn make_json_request<
        T: DeserializeOwned,
        O: FnOnce(&Client) -> reqwest::RequestBuilder,
    >(
        &self,
        to_request: O,
    ) -> HttpResult<T>
    where
        O: Send,
    {
        self.client
            .make_request(to_request)
            .await?
            .json()
            .await
            .map_err(|e| ApiError::JsonParsing { error: e })
    }

    pub(crate) async fn make_request<O: FnOnce(&Client) -> reqwest::RequestBuilder>(
        &self,
        to_request: O,
    ) -> HttpResult<Response>
    where
        O: Send,
    {
        self.client.make_request(to_request).await
    }
}
