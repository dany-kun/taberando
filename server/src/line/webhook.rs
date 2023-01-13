use ring::hmac;
use tokio::sync::mpsc::Sender;
use warp::http::{HeaderMap, StatusCode};
use warp::hyper::body::Bytes;
use warp::{Filter, Rejection, Reply};

use crate::app;
use crate::app::core::Action;
use crate::line::http::LineClient;
use crate::line::json;
use crate::line::json::{Event, Payload};

use super::bot;

#[derive(Debug)]
struct InvalidWebhookError;

impl warp::reject::Reject for InvalidWebhookError {}

#[allow(opaque_hidden_inferred_bound)]
pub fn route(
    line_client: LineClient,
    tx: Sender<(String, Action)>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone + Sync + Send {
    let key = hmac::Key::new(
        hmac::HMAC_SHA256,
        std::env::var("LINE_CHANNEL_SECRET")
            .expect("Needs to have a LINE_CHANNEL_SECRET env variables to verify incoming webhook")
            .as_bytes(),
    );
    warp::path!("line" / "webhook")
        .and(warp::filters::path::full())
        .and(warp::header::headers_cloned())
        .and(warp::post())
        .and(
            warp::header::<String>("X-Line-Signature")
                .or(warp::header::<String>("X-Line-Signature"))
                .unify(),
        )
        .and(warp::body::bytes())
        .and(warp::any().map(move || key.clone()))
        .and_then(
            |path: warp::path::FullPath,
             headers: HeaderMap,
             header: String,
             payload: Bytes,
             key: hmac::Key| async move {
                println!("{:?}, {:?}", path, headers);
                std::str::from_utf8(&payload)
                    .map_err(|_| warp::reject::custom(InvalidWebhookError))
                    .and_then(|text| {
                        let signature = ring::hmac::sign(&key, text.as_bytes());
                        let encoded = base64::encode(signature.as_ref());
                        if header == encoded {
                            serde_json::from_str::<Payload>(text)
                                .map_err(|_| warp::reject::custom(InvalidWebhookError))
                        } else {
                            Err(warp::reject::custom(InvalidWebhookError))
                        }
                    })
            },
        )
        .and(warp::header::<String>("host"))
        .and(warp::any().map(move || line_client.clone()))
        .and(warp::any().map(move || tx.clone()))
        .map(
            |json: Payload, host: String, client: LineClient, tx: Sender<(String, Action)>| {
                println!(
                    "Got {} webhook event(s) from bot {} @ {}",
                    json.events.len(),
                    json.destination,
                    host
                );
                tokio::spawn(async move {
                    let actions = parse_webhook_events(client, json).await;
                    for action in actions {
                        println!("Send {:?}", action);
                        let _ = tx.send((host.clone(), action)).await;
                    }
                });
                StatusCode::OK
            },
        )
}

async fn parse_webhook_events(line_client: LineClient, payload: Payload) -> Vec<app::core::Action> {
    let mut vec: Vec<app::core::Action> = Vec::new();
    for event in payload.events {
        let mode = event.clone().mode;
        println!("{:?}", mode);
        let action = match mode.as_str() {
            "active" => {
                let event_type = event.event_type.as_str();
                println!("{:?}", event_type);
                action(&line_client, event.clone(), event_type).await
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
    #[allow(clippy::needless_return)]
    return vec;
}

async fn action(
    line_client: &LineClient,
    event: Event,
    event_type: &str,
) -> Option<app::core::Action> {
    match event_type {
        "join" => {
            let _ = bot::setup(line_client, event.source).await;
        }
        "follow" => {
            let _ = bot::setup(line_client, event.source).await;
        }
        "message" => {
            let command = event
                .message
                .filter(|m| m.message_type == "text")
                .and_then(|m| m.text)?
                .to_lowercase();

            return if let Some(c) = event.source.to_client() {
                match command.trim().to_lowercase().as_str() {
                    "refresh" => Some(app::core::Action::Refresh(c)),
                    "whoami" => Some(app::core::Action::WhoAmI(c)),
                    _ => None,
                }
            } else {
                None
            };
        }
        "postback" => {
            if let Some(postback) = event.postback {
                if let Some(client) = event.source.to_client() {
                    return match postback.data.as_str() {
                        json::DRAW_LUNCH_ACTION => {
                            Some(app::core::Action::Draw(client, app::core::Meal::Lunch))
                        }
                        json::DRAW_DINNER_ACTION => {
                            Some(app::core::Action::Draw(client, app::core::Meal::Dinner))
                        }
                        json::POSTPONE_ACTION => Some(app::core::Action::PostponeCurrent(client)),
                        json::DELETE_ACTION => Some(app::core::Action::RemoveCurrent(client)),
                        json::ARCHIVE_ACTION => Some(app::core::Action::ArchiveCurrent(client)),
                        json::REFRESH_ACTION => Some(app::core::Action::Refresh(client)),
                        _ => {
                            println!("Unhandled postback: {}", postback.data);
                            None
                        }
                    };
                }
            }
        }
        event => {
            println!("Unhandled event {}", event);
        }
    }
    #[allow(clippy::needless_return)]
    return Option::None;
}
