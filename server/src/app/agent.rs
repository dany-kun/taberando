use async_trait::async_trait;

use crate::app::core::{Client, Coordinates, Meal};
use crate::gcp::api::FirebaseApi;

#[async_trait]
pub trait Agent {
    async fn whoami(&self, client: &Client);
    async fn refresh<T: FirebaseApi + Sync>(
        &self,
        client: &Client,
        firebase_client: &T,
        host: &str,
    );
    async fn try_draw<T: FirebaseApi + Sync>(
        &self,
        meal: Meal,
        client: &Client,
        firebase_client: &T,
        host: &str,
        coordinates: &Option<Coordinates>,
    );
    async fn postpone<T: FirebaseApi + Sync>(
        &self,
        client: &Client,
        firebase_client: &T,
        host: &str,
        coordinates: Option<Coordinates>,
    );
    async fn delete_current<T: FirebaseApi + Sync>(
        &self,
        client: &Client,
        firebase_client: &T,
        host: &str,
        coordinates: Option<Coordinates>,
    );
    async fn archive_current<T: FirebaseApi + Sync>(
        &self,
        client: &Client,
        firebase_client: &T,
        host: &str,
        coordinates: Option<Coordinates>,
    );
    async fn add_place<T: FirebaseApi + Sync>(
        &self,
        client: &Client,
        firebase_client: &T,
        place_name: &str,
        meals: Vec<Meal>,
        host: &str,
    );

    async fn update_location(&self, client: &Client, host: &str, latitude: f32, longitude: f32);

    async fn clear_location(&self, client: &Client, host: &str);
}
