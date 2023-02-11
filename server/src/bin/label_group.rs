use server::gcp::api::FirebaseApi;
use server::gcp::http_api::FirebaseApiV2;
use server::line::api::LineApi;
use server::line::http::LineClient;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let line_token = std::env::var("LINE_TOKEN").unwrap();
    let line_client = LineClient::new(&line_token);
    let firebase_api = FirebaseApiV2::default().await;
    let jars = firebase_api.get_all_groups().await?;
    for jar in jars.iter() {
        let info = line_client.get_jar_info(jar).await;
        if let Ok(info) = info {
            let _ = firebase_api.add_label(jar, &info).await;
        }
    }
    Ok(())
}
