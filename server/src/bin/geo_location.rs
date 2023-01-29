use std::collections::HashMap;

use server::bing::http::{get_bing_context, BingClient, BingError};
use server::gcp::api::FirebaseApiV2;
use server::gcp::client::get_firebase_client;

#[tokio::main]
async fn main() {
    // env_logger::init();
    let firebase_client = get_firebase_client().await;
    let firebase_api = FirebaseApiV2::new(firebase_client);
    let args: Vec<String> = std::env::args().collect();
    let group = args.get(1);
    if group.is_none() {
        return;
    }
    let db_group = group.unwrap();
    let places = firebase_api
        .get_all_places_name(&db_group.to_string())
        .await
        .unwrap();
    let client = BingClient::default();

    let mut results: HashMap<&String, BingError> = HashMap::new();
    for place in places.iter() {
        let result = client
            .find_geo_coordinates_from_query(place, &get_bing_context())
            .await;
        if let Err(e) = result {
            results.insert(place, e);
        }
    }
    println!("{:?}", places.len());
    println!("{:?}", results.len());
    println!("{:?}", results);
}
