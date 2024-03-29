use std::collections::HashMap;

use server::app::jar::Jar;
use server::bing::http::{get_bing_context, BingClient, BingError};
use server::gcp::api::FirebaseApi;
pub(crate) use server::gcp::http_api::FirebaseApiV2;

#[tokio::main]
async fn main() {
    // env_logger::init();
    let firebase_api = FirebaseApiV2::default().await;
    let args: Vec<String> = std::env::args().collect();
    let group = args.get(1);
    if group.is_none() {
        return;
    }

    let db_group = group.unwrap();
    println!("{db_group:?}");
    let jar = &Jar::new(&db_group.to_string());
    let places = firebase_api.get_all_places(jar).await.unwrap();
    let client = BingClient::default();

    let mut results: HashMap<&String, BingError> = HashMap::new();
    for place in places.iter() {
        let result = client
            .find_geo_coordinates_from_query(&place.name, &get_bing_context())
            .await;
        match result {
            Ok(coordinates) => {
                let _ = firebase_api
                    .set_place_coordinates(jar, place, &coordinates)
                    .await;
            }
            Err(e) => {
                results.insert(&place.name, e);
            }
        }
    }
    println!("{:?}", places.len());
    println!("{:?}", results.len());
    println!("{results:?}");
}
