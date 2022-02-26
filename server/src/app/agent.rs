use async_trait::async_trait;

use crate::app::core::{Client, Meal, Place};

#[async_trait]
pub trait Agent {
    async fn whoami(&self, client: &Client);
    async fn refresh(&self, client: &Client, firebase_client: &reqwest::Client, host: &str);
    async fn try_draw(
        &self,
        meal: Meal,
        client: &Client,
        firebase_client: &reqwest::Client,
        host: &str,
    );
    async fn postpone(&self, client: &Client, firebase_client: &reqwest::Client, host: &str);
    async fn delete_current(&self, client: &Client, firebase_client: &reqwest::Client, host: &str);
    async fn archive_current(&self, client: &Client, firebase_client: &reqwest::Client, host: &str);
    async fn add_place(
        &self,
        client: &Client,
        firebase_client: &reqwest::Client,
        place: Place,
        meals: Vec<Meal>,
        host: &str,
    );
}
