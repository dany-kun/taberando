use std::collections::HashMap;

use async_trait::async_trait;
use rand::seq::IteratorRandom;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;

use crate::app::core::{Meal, Place};
use crate::http::{HttpClient, HttpResult};

const BASE_URL: &str = env!("FIREBASE_URL");

const CURRENT_DRAW_PATH: &str = "pending_shop";

const FOLDER_PATH: &str = "./src/gcp";

pub(crate) type Jar = String;

struct OAuth {
    token: String,
    project_id: String,
}

impl From<Meal> for &str {
    fn from(meal: Meal) -> Self {
        match meal {
            Meal::Lunch => "昼だけ",
            Meal::Dinner => "夜だけ",
        }
    }
}

#[async_trait]
pub trait FirebaseApi {
    async fn get_current_draw(&self, jar: &Jar) -> HttpResult<Option<String>>;

    async fn draw(&self, jar: &Jar, meal: Meal) -> HttpResult<Option<String>>;

    async fn add_place(&self, jar: &Jar, place: Place, meal: Vec<Meal>) -> HttpResult<Place>;

    async fn remove_drawn_place(&self, jar: &Jar, place: Option<Place>) -> HttpResult<()>;

    async fn delete_place(&self, jar: &Jar, place: Place) -> HttpResult<Place>;

    async fn get_list_of_places(
        &self,
        jar: &Jar,
        meal: Meal,
    ) -> HttpResult<HashMap<String, String>>;

    fn firebase_url(&self, jar: &Jar, path: &str) -> String {
        format!("{}/{}/{}.json", BASE_URL, jar, path)
    }
}

#[async_trait]
impl FirebaseApi for Client {
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
        result.map(|places| places.unwrap_or(HashMap::new()))
    }
}

pub async fn get_firebase_client() -> Client {
    let oauth = get_oauth_token().await.unwrap();
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

#[derive(Debug)]
struct Error;

async fn get_oauth_token() -> Result<OAuth, yup_oauth2::Error> {
    // Read application secret from a file. Sometimes it's easier to compile it directly into
    // the binary. The clientsecret file contains JSON like `{"installed":{"client_id": ... }}`
    let secret =
        yup_oauth2::read_service_account_key(format!("{}/service_account.json", FOLDER_PATH))
            .await
            .map_err(|_| Error)
            .or(std::env::var("GOOGLE_CREDENTIALS")
                .map_err(|_| Error)
                .and_then(|json| yup_oauth2::parse_service_account_key(json).map_err(|_| Error)))
            .expect("Could not find service account file");

    let auth = yup_oauth2::ServiceAccountAuthenticator::builder(secret.clone())
        // .persist_tokens_to_disk(format!("{}/tokencache.json", FOLDER_PATH))
        .build()
        .await
        .unwrap();

    let scopes = &[
        "https://www.googleapis.com/auth/userinfo.email",
        "https://www.googleapis.com/auth/firebase.database",
    ];

    // token(<scopes>) is the one important function of this crate; it does everything to
    // obtain a token that can be sent e.g. as Bearer token.
    let token = auth.token(scopes).await?;
    std::result::Result::Ok(OAuth {
        token: token.as_str().to_string(),
        project_id: secret.clone().project_id.unwrap().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_draw_meal() {
        let p = get_firebase_client().await.draw(Meal::Lunch).await;
    }

    #[tokio::test]
    async fn it_postpones_meal() {
        let p = get_firebase_client()
            .await
            .remove_drawn_place(Option::None)
            .await;
    }

    #[tokio::test]
    async fn it_adds_place() {
        let p = get_firebase_client()
            .await
            .add_place(
                Place {
                    name: "test_name3".to_string(),
                },
                vec![Meal::Dinner, Meal::Lunch],
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn it_deletes_place() {
        let p = get_firebase_client()
            .await
            .delete(Place {
                name: "test_name2".to_string(),
            })
            .await
            .unwrap();
    }
}
