use base64::Engine;
use ring::hmac;
use tokio::sync::mpsc::Sender;
use warp::http::{HeaderMap, StatusCode};
use warp::hyper::body::Bytes;
use warp::{Filter, Rejection, Reply};

use crate::app::core::Action;
use crate::app::user_action::UserAction;
use crate::line::http::LineClient;
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
                println!("{path:?}, {headers:?}");
                std::str::from_utf8(&payload)
                    .map_err(|_| warp::reject::custom(InvalidWebhookError))
                    .and_then(|text| {
                        let signature = hmac::sign(&key, text.as_bytes());
                        let encoded =
                            base64::engine::general_purpose::STANDARD.encode(signature.as_ref());
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
                        println!("Send {action:?}");
                        let _ = tx.send((host.clone(), action)).await;
                    }
                });
                StatusCode::OK
            },
        )
}

async fn parse_webhook_events(line_client: LineClient, payload: Payload) -> Vec<Action> {
    let mut vec: Vec<Action> = Vec::new();
    for event in payload.events {
        let mode = event.clone().mode;
        println!("{mode:?}");
        let action = match mode.as_str() {
            "active" => {
                let event_type = event.event_type.as_str();
                println!("{event_type:?}");
                action(&line_client, &event, event_type).await
            }
            _ => {
                println!("Unknown event mode {mode:?}");
                None
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

async fn action(line_client: &LineClient, event: &Event, event_type: &str) -> Option<Action> {
    match event_type {
        "join" => {
            let _ = bot::setup(line_client, &event.source).await;
        }
        "follow" => {
            let _ = bot::setup(line_client, &event.source).await;
        }
        "message" => return message_to_action(event),
        "postback" => {
            if let (Some(client), Some(postback)) = (event.source.to_client(), &event.postback) {
                if let Ok(user_action) = serde_json::from_str(postback.data.as_str()) {
                    return match user_action {
                        UserAction::Draw(meal, coordinates) => {
                            Some(Action::Draw(client, meal, coordinates))
                        }
                        UserAction::Postpone(coordinates) => {
                            Some(Action::PostponeCurrent(client, coordinates))
                        }
                        UserAction::DeleteCurrent(coordinates) => {
                            Some(Action::RemoveCurrent(client, coordinates))
                        }
                        UserAction::ArchiveCurrent(coordinates) => {
                            Some(Action::ArchiveCurrent(client, coordinates))
                        }
                        UserAction::ClearLocation => Some(Action::ClearLocation(client)),
                        UserAction::Add => {
                            println!("Unhandled postback event: {:?}", &event);
                            None
                        }
                        UserAction::Refresh => Some(Action::Refresh(client)),
                    };
                }
            }
        }
        event => {
            println!("Unhandled event {event}");
        }
    }
    #[allow(clippy::needless_return)]
    return None;
}

fn message_to_action(event: &Event) -> Option<Action> {
    let message = event.message.as_ref()?;
    let client = event.source.to_client()?;
    match message.message_type.as_str() {
        "text" => match message.text.as_ref()?.to_lowercase().trim() {
            "refresh" => Some(Action::Refresh(client)),
            "更新" => Some(Action::Refresh(client)),
            "whoami" => Some(Action::WhoAmI(client)),
            _ => None,
        },
        "location" => {
            if let (Some(lat), Some(long)) = (message.latitude, message.longitude) {
                Some(Action::Location(client, lat, long))
            } else {
                None
            }
        }
        _ => None,
    }
}
