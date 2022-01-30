
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client};




pub fn get_line_client(line_token: Option<String>) -> Client {
    let line_token = line_token
        .or_else(|| std::env::var("LINE_TOKEN").ok())
        .unwrap();

    let mut header_map = HeaderMap::new();

    let authorization_header = &*format!("Bearer {}", line_token);
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
