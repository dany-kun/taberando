use reqwest::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use crate::gcp::oauth;

pub async fn get_firebase_client() -> Client {
    let oauth = oauth::get_oauth_token().await.unwrap();
    let _ = env_logger::try_init();
    let mut header_map = HeaderMap::new();

    let authorization_header = &*format!("Bearer {}", oauth.token);
    let mut auth_value = HeaderValue::from_str(authorization_header).unwrap();
    auth_value.set_sensitive(true);
    header_map.append(AUTHORIZATION, auth_value);

    header_map.append(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    Client::builder()
        .default_headers(header_map)
        .connection_verbose(true)
        .build()
        .unwrap()
}
