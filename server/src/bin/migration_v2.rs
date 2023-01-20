use server::gcp;
use server::gcp::migration_v2::migrate_v2;

#[tokio::main]
async fn main() {
    // env_logger::init();
    let firebase_client = gcp::client::get_firebase_client().await;
    migrate_v2(firebase_client).await.expect("Error");
}
