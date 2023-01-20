use crate::app::core::Meal;
use crate::gcp::api::{FirebaseApi, FirebaseApiV2};
use crate::gcp::constants::BASE_URL;
use crate::http::{HttpClient, HttpResult};
use reqwest::Client;
use std::collections::HashMap;

pub async fn migrate_v2(http_client: Client) -> HttpResult<()> {
    let v1 = http_client
        .make_json_request(|client| client.get(format!("{}/.json", BASE_URL)))
        .await?;
    if let serde_json::Value::Object(map) = v1 {
        let _api = FirebaseApiV2::new(http_client);
        for (entry, _values) in map.iter() {
            if entry != "v2" {
                // Don't run as this is not idempotent
                // migrating_entry(entry, values, &api).await
            }
        }
    }
    Ok(())
}

#[allow(dead_code)]
async fn migrating_entry(jar_name: &String, values: &serde_json::Value, api: &FirebaseApiV2) {
    println!("Migrating {:?}", jar_name);
    if let serde_json::Value::Object(jar_entries) = values {
        let mut shops = HashMap::new();
        for (db_key, value) in jar_entries.iter() {
            match db_key.as_str() {
                "pending_shop" => {
                    let _ = api
                        .update_current_draw(jar_name, value.as_str().unwrap())
                        .await;
                }
                "昼だけ" => {
                    value.as_object().unwrap().iter().for_each(|(_, name)| {
                        let shop_name = name.as_str().unwrap();
                        let mut times: Vec<Meal> =
                            (shops.get(shop_name).unwrap_or(&vec![])).clone();
                        times.push(Meal::Lunch);
                        shops.insert(shop_name, times);
                    });
                }
                "夜だけ" => {
                    value.as_object().unwrap().iter().for_each(|(_, name)| {
                        let shop_name = name.as_str().unwrap();
                        let mut times: Vec<Meal> =
                            (shops.get(shop_name).unwrap_or(&vec![])).clone();
                        times.push(Meal::Dinner);
                        shops.insert(shop_name, times);
                    });
                }
                unknown => println!("Unknown entry {:?} for jar {:?}", unknown, jar_name),
            }
        }
        println!("Adding shops {:?}", shops);
        for (name, meals) in shops.iter() {
            let _ = api.add_place(jar_name, name, meals).await;
        }
    } else {
        println!("Unknown entry {:?}", values)
    }
}