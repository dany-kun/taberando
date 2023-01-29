use crate::app::core::Coordinates;
use futures::TryFutureExt;
use reqwest::Url;
use serde::Deserialize;
use serde_json::Value;
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display};

const BING_API_KEY: &str = env!("BING_MAP_API_KEY");
const BING_CONTEXT: Option<&str> = option_env!("BING_MAP_API_CONTEXT");

pub fn get_bing_context() -> Vec<(String, String)> {
    BING_CONTEXT.map_or_else(Vec::new, |ctx| {
        serde_json::from_str::<Value>(ctx).map_or(vec![], |bing_json| match bing_json {
            Value::Object(o) => {
                let mut collect: Vec<(String, String)> = vec![];
                for (k, v) in o {
                    collect.push((k, v.as_str().unwrap().to_string()))
                }
                collect
            }
            _ => vec![],
        })
    })
}

#[derive(Debug)]
pub struct BingError(String);

impl Display for BingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bing Error")
    }
}

impl Error for BingError {}

impl From<reqwest::Error> for BingError {
    fn from(value: reqwest::Error) -> Self {
        BingError(format!("{value:?}"))
    }
}

#[derive(Deserialize, Debug)]
struct BingResourceSets {
    #[serde(rename(deserialize = "resourceSets"))]
    resource_sets: Vec<BingResources>,
}

#[derive(Deserialize, Debug)]
struct BingResources {
    resources: Vec<BingResource>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BingResource {
    pub value: Option<Vec<AutoSuggestResourceValue>>,
    point: Option<LocationResourcePoint>,
}

#[derive(Deserialize, Debug, Clone)]
struct LocationResourcePoint {
    coordinates: Vec<f32>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AutoSuggestResourceValue {
    pub address: AutoSuggestResourceValueAddress,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct AutoSuggestResourceValueAddress {
    #[serde(rename(deserialize = "adminDistrict"))]
    admin_district: Option<String>,
    #[serde(rename(deserialize = "adminDistrict2"))]
    admin_district2: Option<String>,
    #[serde(rename(deserialize = "addressLine"))]
    pub address_line: Option<String>,
}

pub struct BingClient(reqwest::Client);

impl Default for BingClient {
    fn default() -> Self {
        BingClient(reqwest::Client::new())
    }
}

impl BingClient {
    pub async fn find_geo_coordinates_from_query(
        &self,
        query: &str,
        location_refinements: &[(String, String)],
    ) -> Result<Coordinates, BingError> {
        self.find_address(query, location_refinements)
            .and_then(|address| async move { self.find_geo_coordinates(&address).await })
            .await
    }

    pub async fn find_address(
        &self,
        query: &str,
        location_refinements: &[(String, String)],
    ) -> Result<String, BingError> {
        self.find_locations(query, location_refinements)
            .await?
            .first()
            .and_then(|bing_resource| bing_resource.value.as_ref())
            .and_then(|resource_values| resource_values.first())
            .and_then(|resource_value| resource_value.address.address_line.clone())
            .ok_or_else(|| BingError("Could not find address".to_string()))
    }

    // See refinement options here
    // https://learn.microsoft.com/en-us/bingmaps/rest-services/autosuggest
    async fn find_locations(
        &self,
        query: &str,
        location_refinements: &[(String, String)],
    ) -> Result<Vec<BingResource>, reqwest::Error> {
        let mut url = Url::parse("https://dev.virtualearth.net/REST/v1/AutoSuggest/?c=ja").unwrap();
        url.query_pairs_mut().append_pair("key", BING_API_KEY);
        url.query_pairs_mut().append_pair("query", query);
        location_refinements.iter().for_each(|(k, v)| {
            url.query_pairs_mut().append_pair(k, v);
        });
        let resource_sets: BingResourceSets = self.0.get(url).send().await?.json().await?;
        let vec = &resource_sets.resource_sets.first().unwrap().resources;
        Ok(vec.clone())
    }

    pub async fn find_geo_coordinates(&self, address: &str) -> Result<Coordinates, BingError> {
        let mut url = Url::parse(
            "http://dev.virtualearth.net/REST/v1/Locations?countryRegion=JP&c=ja&maxResults=1",
        )
        .unwrap();
        url.query_pairs_mut().append_pair("key", BING_API_KEY);
        url.query_pairs_mut().append_pair("addressLine", address);
        let resource_sets: BingResourceSets = self.0.get(url).send().await?.json().await?;
        let bing_coordinates = resource_sets
            .resource_sets
            .first()
            .and_then(|s| s.resources.first())
            .and_then(|r| r.point.as_ref())
            .map(|p| &p.coordinates)
            .ok_or_else(|| BingError("Could not find coordinates".to_string()));

        bing_coordinates.map(|c| Coordinates {
            latitude: *c.first().unwrap(),
            longitude: *c.get(1).unwrap(),
        })
    }
}
