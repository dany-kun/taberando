use server::gcp::http_api::FirebaseApiV2;

#[tokio::main]
async fn main() {
    let _firebase_api = FirebaseApiV2::default().await;
}
