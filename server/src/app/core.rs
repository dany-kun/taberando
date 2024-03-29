use crate::app::agent::Agent;
use crate::app::coordinates::Coordinates;
use crate::bing::http::BingClient;
use crate::gcp::api::FirebaseApi;
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

#[derive(Debug, Clone)]
pub struct Place {
    pub key: String,
    pub name: String,
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
