

use reqwest::Client;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use warp::Filter;

use server::app;
use server::app::core::Action;
use server::line::webhook;
use server::line::{http};

#[tokio::main]
async fn main() {
    env_logger::init();

    let port = 4001;
    let http_client = http::get_line_client(None);

    let (tx, rx) = mpsc::channel(32);

    tokio::try_join!(launch_server(port, http_client, tx), launch_core_agent(rx));
}

async fn launch_server(
    port: u16,
    http_client: Client,
    tx: Sender<Action>,
) -> Result<(), &'static str> {
    warp::serve(webhook::route(http_client, tx.clone()).with(warp::log("")))
        .run(([127, 0, 0, 1], port))
        .await;
    Result::Ok(())
}

async fn launch_core_agent(mut rx: Receiver<app::core::Action>) -> Result<(), &'static str> {
    while let Some(_action) = rx.recv().await {
        println!("Got action")
    }
    Result::Ok(())
}
