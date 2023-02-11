use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use warp::Filter;

use server::app::core::Action;
use server::gcp::api::FirebaseApi;
use server::gcp::http_api::FirebaseApiV2;
use server::line::http::LineClient;
use server::{app, line};

#[tokio::main]
async fn main() {
    env_logger::init();

    let port = std::env::var("PORT")
        .map(|port| port.parse::<u16>().unwrap())
        .unwrap_or(4001);
    let line_token = std::env::var("LINE_TOKEN").expect("Please specify a LINE_TOKEN env variable");
    let line_client = LineClient::new(&line_token);

    let (tx, rx) = mpsc::channel(32);

    let fc = FirebaseApiV2::default().await;
    let _ = tokio::try_join!(
        launch_server(port, &line_client, tx),
        launch_core_agent(rx, &line_client, &fc)
    );
}

async fn launch_server(
    port: u16,
    line_client: &line::http::LineClient,
    tx: Sender<(String, Action)>,
) -> Result<(), &'static str> {
    warp::serve(
        line::webhook::route(line_client.clone(), tx.clone())
            .or(line::html::route(tx.clone()))
            .with(warp::log("")),
    )
    .run(([0, 0, 0, 0], port))
    .await;
    Result::Ok(())
}

async fn launch_core_agent<T: FirebaseApi + Sync>(
    mut rx: Receiver<(String, Action)>,
    line_client: &line::http::LineClient,
    firebase_client: &T,
) -> Result<(), &'static str> {
    println!("Receiving");
    while let Some(action) = rx.recv().await {
        println!("Got action {action:?}");
        app::core::handle_action(action, line_client, firebase_client).await;
    }
    Result::Ok(())
}
