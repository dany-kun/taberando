use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use warp::Filter;

use crate::app::core::{Action, Client, Meal, Place};
use crate::line::http::LineChannel;

#[derive(Deserialize, Serialize, Debug)]
struct Source {
    source: String,
    source_type: String,
    source_id: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct Entry {
    place: String,
    time: String,
}

pub fn route(
    sender: Sender<(String, Action)>,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone + Sync + Send {
    let source = warp::query::<Source>().and_then(|source: Source| async move {
        if source.source == "line" {
            Ok(source)
        } else {
            Err(warp::reject::not_found())
        }
    });
    let form_get = source
        .and(warp::get())
        .map(|_| ())
        .untuple_one()
        .and(warp::fs::file("./src/line/add.html"));
    let form_post = warp::post()
        .and(source)
        .and(warp::body::form::<Entry>())
        .and(warp::header::<String>("host"))
        .and(warp::any().map(move || sender.clone()))
        .then(
            |source: Source, body: Entry, host: String, sender: Sender<(String, Action)>| async move {
                let action = to_action(&source, &body);

                if let Some(action) = action {
                    tokio::spawn(async move {
                        sender.send((host, action)).await;
                    });
                } else {
                    println!("Could not handle {:?} {:?}", source, body);
                }
                ()
            },
        )
        .untuple_one()
        .and(warp::fs::file("./src/line/autoclose.html"));

    warp::path!("line" / "draw").and(form_get.or(form_post))
}

fn to_action(source: &Source, body: &Entry) -> Option<Action> {
    let source_id = source.source_id.clone();
    let client = match source.source_type.as_str() {
        "user" => Some(LineChannel::User(source_id)),
        "group" => Some(LineChannel::Group {
            id: source_id,
            user_id: None,
        }),
        "room" => Some(LineChannel::Room {
            id: source_id,
            user_id: None,
        }),
        _ => None,
    }
    .map(|channel| Client::Line(channel));

    client.and_then(|c| {
        let meals = match body.time.as_str() {
            "both" => Some(vec![Meal::Lunch, Meal::Dinner]),
            "lunch" => Some(vec![Meal::Lunch]),
            "dinner" => Some(vec![Meal::Dinner]),
            _unknown => None,
        };
        meals.map(|m| {
            Action::Add(
                c,
                Place {
                    name: body.place.to_string(),
                },
                m,
            )
        })
    })
}
