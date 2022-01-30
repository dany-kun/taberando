use std::error::Error;

use reqwest::Client;
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use warp::http::StatusCode;
use warp::Filter;

use crate::app;
use crate::app::core::Action;
use crate::line::menu;

use super::bot;

#[derive(Debug, Deserialize, Clone)]
struct Payload {
    destination: String,
    events: Vec<Event>,
}

#[derive(Debug, Deserialize, Clone)]
struct Event {
    #[serde(rename(deserialize = "type"))]
    event_type: String,
    mode: String,
    source: bot::EventSource,
    postback: Option<bot::Postback>,
}

pub fn route(
    http_client: Client,
    tx: Sender<Action>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("line" / "webhook")
        .and(warp::any().map(move || http_client.clone()))
        .and(warp::any().map(move || tx.clone()))
        .and(warp::post())
        .and(warp::body::json::<Payload>())
        .map(|client: Client, tx: Sender<Action>, json: Payload| {
            println!(
                "Got {} webhook event(s) from {}",
                json.events.len(),
                json.destination
            );
            tokio::spawn(async move {
                let actions = parse_webhook_events(client, json).await;
                for action in actions {
                    tx.send(action);
                }
            });
            StatusCode::OK
        })
}

async fn parse_webhook_events(http_client: Client, payload: Payload) -> Vec<app::core::Action> {
    let mut vec: Vec<app::core::Action> = Vec::new();
    for event in payload.events {
        let mode = event.clone().mode;
        println!("{:?}", mode);
        let action = match mode.as_str() {
            "active" => {
                let event_type = event.event_type.as_str();
                println!("{:?}", event_type);
                action(&http_client, event.clone(), event_type).await
            }
            _ => {
                println!("Unknown event mode {:?}", mode);
                Option::None
            }
        };
        match action {
            None => {}
            Some(result) => vec.push(result),
        }
    }
    return vec;
}

async fn action(http_client: &Client, event: Event, event_type: &str) -> Option<app::core::Action> {
    match event_type {
        "join" => {
            bot::setup(&http_client, event.source).await;
        }
        "follow" => {
            bot::setup(&http_client, event.source).await;
        }
        "message" => {
            bot::setup(&http_client, event.source).await;
        }
        "postback" => {
            if let Some(postback) = event.postback {
                match postback.data.as_str() {
                    menu::DRAW_LUNCH_ACTION => {
                        return event
                            .source
                            .to_client()
                            .map(|client| app::core::Action::Draw(client, app::core::Meal::Lunch));
                    }
                    menu::DRAW_DINNER_ACTION => {
                        return event.source.to_client().map(|client| {
                            app::core::Action::Draw(client, app::core::Meal::Dinner)
                        });
                    }
                    _ => {
                        println!("Unhandled postback: {}", postback.data);
                    }
                }
            }
        }
        event => {
            println!("Unhandled event {}", event);
        }
    }
    return Option::None;
}
