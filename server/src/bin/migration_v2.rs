use server::gcp::http_api::FirebaseApiV2;
use server::gcp::migration_v2::migrate_v2;

#[tokio::main]
async fn main() {
    // env_logger::init()
    let firebase_client = FirebaseApiV2::default().await;
    migrate_v2(&firebase_client).await.expect("Error");
}
