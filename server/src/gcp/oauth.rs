use crate::gcp;
use constants::Error;
use gcp::constants;
use gcp::constants::FOLDER_PATH;

pub struct OAuth {
    pub(crate) token: String,
    #[allow(dead_code)]
    project_id: String,
}

pub async fn get_oauth_token() -> Result<OAuth, yup_oauth2::Error> {
    // Read application secret from a file. Sometimes it's easier to compile it directly into
    // the binary. The clientsecret file contains JSON like `{"installed":{"client_id": ... }}`
    let secret =
        yup_oauth2::read_service_account_key(format!("{FOLDER_PATH}/service_account.json"))
            .await
            .map_err(|_| Error)
            .or_else(|_| {
                std::env::var("GOOGLE_CREDENTIALS")
                    .map_err(|_| Error)
                    .and_then(|json| yup_oauth2::parse_service_account_key(json).map_err(|_| Error))
            })
            .expect("Could not find service account file");

    let auth = yup_oauth2::ServiceAccountAuthenticator::builder(secret.clone())
        // .persist_tokens_to_disk(format!("{}/tokencache.json", FOLDER_PATH))
        .build()
        .await
        .unwrap();

    let scopes = &[
        "https://www.googleapis.com/auth/userinfo.email",
        "https://www.googleapis.com/auth/firebase.database",
    ];

    // token(<scopes>) is the one important function of this crate; it does everything to
    // obtain a token that can be sent e.g. as Bearer token.
    let token = auth.token(scopes).await?;
    Ok(OAuth {
        token: token.as_str().to_string(),
        project_id: secret.project_id.unwrap(),
    })
}
