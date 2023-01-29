use crate::app::agent::Agent;
use crate::app::core::Client::Line;
use crate::bing::http::BingClient;
use crate::gcp::api::{FirebaseApi, Jar};
use crate::line::http::{LineChannel, LineClient};

#[derive(Debug)]
pub enum Client {
    Line(LineChannel),
}

#[derive(Debug)]
pub enum Action {
    Add(Client, String, Vec<Meal>),
    Draw(Client, Meal, Option<Coordinates>),
    PostponeCurrent(Client, Option<Coordinates>),
    ArchiveCurrent(Client, Option<Coordinates>),
    RemoveCurrent(Client, Option<Coordinates>),
    Refresh(Client),
    WhoAmI(Client),
    Location(Client, f32, f32),
    ClearLocation(Client),
}

#[derive(Debug, Clone)]
pub enum Meal {
    Lunch,
    Dinner,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct Coordinates {
    pub latitude: f32,
    pub longitude: f32,
}

#[derive(Debug, Clone)]
pub struct Place {
    pub key: String,
    pub name: String,
}

impl From<&Client> for Jar {
    fn from(client: &Client) -> Self {
        match client {
            Line(channel) => match channel {
                LineChannel::User(id) => format!("user_{id}"),
                LineChannel::Room { id, .. } => format!("room_{id}"),
                LineChannel::Group { id, .. } => format!("group_{id}"),
            },
        }
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

pub async fn handle_action<T: FirebaseApi + Sync>(
    action: (String, Action),
    line_client: &LineClient,
    firebase_client: &T,
) {
    let (host, action) = action;
    match action {
        Action::Draw(source, meal, coordinates) => {
            line_client
                .try_draw(meal, &source, firebase_client, &host, &coordinates)
                .await;
        }
        Action::PostponeCurrent(source, coordinates) => {
            line_client
                .postpone(&source, firebase_client, &host, coordinates)
                .await;
        }
        Action::RemoveCurrent(source, coordinates) => {
            line_client
                .delete_current(&source, firebase_client, &host, coordinates)
                .await;
        }
        Action::ArchiveCurrent(source, coordinates) => {
            line_client
                .archive_current(&source, firebase_client, &host, coordinates)
                .await;
        }
        Action::Refresh(source) => {
            line_client.refresh(&source, firebase_client, &host).await;
        }
        Action::WhoAmI(source) => {
            line_client.whoami(&source).await;
        }
        Action::Add(source, place_name, meals) => {
            let place = line_client
                .add_place(&source, firebase_client, &place_name, meals, &host)
                .await;
            match place {
                Ok(place) => {
                    line_client
                        .add_place_coordinates(
                            &source,
                            firebase_client,
                            &place,
                            &host,
                            BingClient::default(),
                        )
                        .await;
                }
                Err(e) => {
                    println!("{e:?}");
                }
            }
        }
        Action::Location(source, latitude, longitude) => {
            line_client
                .update_location(&source, &host, latitude, longitude)
                .await;
        }
        Action::ClearLocation(source) => {
            line_client.clear_location(&source, &host).await;
        }
    }
}
