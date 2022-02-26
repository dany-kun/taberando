use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use crate::app::agent::Agent;
use crate::app::core::Client::Line;
use crate::gcp;
use crate::gcp::firebase::Jar;
use crate::line::http::{LineChannel, LineClient};

#[derive(Debug)]
pub enum Client {
    Line(LineChannel),
}

#[derive(Debug)]
pub enum Action {
    Add(Client, Place, Vec<Meal>),
    Draw(Client, Meal),
    PostponeCurrent(Client),
    ArchiveCurrent(Client),
    RemoveCurrent(Client),
    Refresh(Client),
    WhoAmI(Client),
}

#[derive(Debug, Clone)]
pub enum Meal {
    Lunch,
    Dinner,
}

#[derive(Debug, Clone)]
pub struct Place {
    pub name: String,
}

impl From<&Client> for Jar {
    fn from(client: &Client) -> Self {
        let id = match client {
            Line(channel) => match channel {
                LineChannel::User(id) => format!("user_{}", id),
                LineChannel::Room { id, .. } => format!("room_{}", id),
                LineChannel::Group { id, .. } => format!("group_{}", id),
            },
        };
        map_jar_to_alias(&id)
    }
}

struct JarError;

impl From<std::io::Error> for JarError {
    fn from(_: std::io::Error) -> Self {
        JarError
    }
}

impl From<serde_json::Error> for JarError {
    fn from(_: serde_json::Error) -> Self {
        JarError
    }
}

pub(crate) fn map_jar_to_alias(jar_key: &str) -> gcp::firebase::Jar {
    // TODO improve this flow by better leveraging into/from traits
    jar_to_alias_overrides()
        .and_then(|json: HashMap<String, String>| {
            json.get(jar_key)
                .map(|value| value.to_string())
                .ok_or_else(|| JarError)
        })
        .unwrap_or(jar_key.to_string())
}

#[cfg(not(debug_assertions))]
fn jar_to_alias_overrides() -> Result<HashMap<String, String>, JarError> {
    option_env!("FIREBASE_JAR_OVERRIDES")
        .ok_or(JarError)
        .and_then(|file| serde_json::from_reader(BufReader::new(file)).map_err(|_| JarError))
}

#[cfg(debug_assertions)]
fn jar_to_alias_overrides() -> Result<HashMap<String, String>, JarError> {
    File::open("./src/gcp/jars.json")
        .map_err(|_| JarError)
        .and_then(|file| serde_json::from_reader(BufReader::new(file)).map_err(|_| JarError))
}

pub async fn handle_action(
    action: (String, Action),
    line_client: &LineClient,
    firebase_client: &reqwest::Client,
) {
    let (host, action) = action;
    match action {
        Action::Draw(source, meal) => {
            line_client
                .try_draw(meal, &source, firebase_client, &host)
                .await;
        }
        Action::PostponeCurrent(source) => {
            line_client.postpone(&source, firebase_client, &host).await;
        }
        Action::RemoveCurrent(source) => {
            line_client
                .delete_current(&source, firebase_client, &host)
                .await;
        }
        Action::ArchiveCurrent(source) => {
            line_client
                .archive_current(&source, firebase_client, &host)
                .await;
        }
        Action::Refresh(source) => {
            line_client.refresh(&source, firebase_client, &host).await;
        }
        Action::WhoAmI(source) => {
            line_client.whoami(&source).await;
        }
        Action::Add(source, place, meals) => {
            line_client
                .add_place(&source, firebase_client, place, meals, &host)
                .await;
        }
    }
}
