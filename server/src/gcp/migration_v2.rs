use crate::app::core::Meal;
use crate::app::jar::Jar;

use crate::gcp::api::FirebaseApi;
use crate::gcp::constants::BASE_URL;
use crate::gcp::http_api::FirebaseApiV2;
use crate::http::HttpResult;
use std::collections::HashMap;

pub async fn migrate_v2(http_client: &FirebaseApiV2) -> HttpResult<()> {
    let v1 = http_client
        .make_json_request(|client| client.get(format!("{BASE_URL}/.json")))
        .await?;
    if let serde_json::Value::Object(map) = v1 {
        let _api = FirebaseApiV2::default();
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
async fn migrating_entry(jar: &Jar, values: &serde_json::Value, api: &FirebaseApiV2) {
    println!("Migrating {jar:?}");
    if let serde_json::Value::Object(jar_entries) = values {
        let mut shops = HashMap::new();
        for (db_key, value) in jar_entries.iter() {
            match db_key.as_str() {
                "pending_shop" => {
                    let _ = api.update_current_draw(jar, value.as_str().unwrap()).await;
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
                unknown => println!("Unknown entry {unknown:?} for jar {jar:?}"),
            }
        }
        println!("Adding shops {shops:?}");
        for (name, meals) in shops.iter() {
            let _ = api.add_place(jar, name, meals).await;
        }
    } else {
        println!("Unknown entry {values:?}")
    }
}
