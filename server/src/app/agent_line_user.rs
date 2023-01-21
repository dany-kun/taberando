use async_trait::async_trait;

use crate::app::agent::Agent;
use crate::app::core::{Client, Meal, Place};
use crate::gcp::api::{FirebaseApi, Jar};
use crate::http::{Empty, HttpResult};
use crate::line::http::{LineChannel, LineClient};
use crate::line::json::{MessageContent, QuickReply, QuickReplyState};

async fn get_current_draw<T: FirebaseApi + Sync>(
    client: &Client,
    firebase_client: &T,
) -> (Jar, HttpResult<Option<Place>>) {
    let jar: Jar = client.into();
    let draw = firebase_client.get_current_draw(&jar).await;
    (jar, draw)
}

impl Client {
    pub(crate) fn add_place_quick_reply(&self, host: &str) -> QuickReply {
        let (source_type, source_id) = match self {
            Client::Line(channel) => match channel {
                LineChannel::User(id) => ("user", id),
                LineChannel::Room { id, .. } => ("room", id),
                LineChannel::Group { id, .. } => ("group", id),
            },
        };
        let path_and_query = format!(
            "/line/draw?source=line&source_type={}&source_id={}",
            source_type, source_id
        );
        let uri = warp::http::uri::Uri::builder()
            .scheme("https")
            .authority(host)
            .path_and_query(path_and_query)
            .build()
            .unwrap();
        MessageContent::uri_quick_reply("+ 加", &uri.to_string(), None)
    }
}

async fn delete_current<F: FnOnce(String) -> String, T: FirebaseApi + Sync>(
    client: &Client,
    line_client: &LineClient,
    firebase_client: &T,
    host: &str,
    message_formatter: F,
) {
    let (jar, draw) = get_current_draw(client, firebase_client).await;
    match draw {
        Ok(draw) => match draw {
            None => {
                println!(
                    "Something is wrong here; tried to postpone the current shop but got no data"
                );
            }
            Some(draw) => {
                let drawn_place_name = draw.name;
                let _ = firebase_client
                    .delete_place(
                        &jar,
                        &Place {
                            name: drawn_place_name.clone(),
                            key: draw.key,
                        },
                    )
                    .await;
                let _ = line_client
                    .send_to_all_users(
                        client,
                        MessageContent::text(&message_formatter(drawn_place_name.clone()))
                            .with_quick_replies(client, host, QuickReplyState::Idle),
                    )
                    .await;
            }
        },
        Err(e) => {
            let _ = line_client
                .send_to_all_users(client, MessageContent::error_message(&e))
                .await;
        }
    }
}

impl LineClient {
    async fn refresh<F: FnOnce(&Option<String>) -> String, T: FirebaseApi + Sync>(
        &self,
        client: &Client,
        firebase_client: &T,
        host: &str,
        message: F,
    ) {
        // Add count
        let (_jar, draw) = get_current_draw(client, firebase_client).await;
        let message = draw
            .map(|res| {
                let text_message = message(&res.clone().map(|p| p.name));
                res.map(|_draw| {
                    MessageContent::text(&text_message).with_quick_replies(
                        client,
                        host,
                        QuickReplyState::ActiveDraw,
                    )
                })
                .unwrap_or_else(|| {
                    MessageContent::text(&text_message).with_quick_replies(
                        client,
                        host,
                        QuickReplyState::Idle,
                    )
                })
            })
            .unwrap_or_else(|e| MessageContent::error_message(&e));
        let _ = self.send_to_all_users(client, message).await;
    }

    async fn send_to_single_user(
        &self,
        line: &Client,
        message: MessageContent,
    ) -> HttpResult<Empty> {
        let to = match line {
            Client::Line(channel) => match channel {
                LineChannel::User(id) => Some(id),
                LineChannel::Room { user_id, .. } => user_id.as_ref(),
                LineChannel::Group { user_id, .. } => user_id.as_ref(),
            },
        };

        match to {
            None => {
                println!("Could not send to a single user for {:?}", line);
                Ok(Empty {})
            }
            Some(user_id) => self.send_to(user_id, message).await,
        }
    }

    async fn send_to_all_users(&self, line: &Client, message: MessageContent) -> HttpResult<Empty> {
        let to = match line {
            Client::Line(channel) => match channel {
                LineChannel::User(id) => id,
                LineChannel::Room { id, .. } => id,
                LineChannel::Group { id, .. } => id,
            },
        };
        self.send_to(to, message).await
    }
}

#[async_trait]
impl Agent for LineClient {
    async fn whoami(&self, client: &Client) {
        let user_id = match client {
            Client::Line(line) => match line {
                LineChannel::User(id) => Some(id),
                LineChannel::Room { user_id, .. } => user_id.as_ref(),
                LineChannel::Group { user_id, .. } => user_id.as_ref(),
            },
        };
        if let Some(id) = user_id {
            let _ = self
                .send_to_single_user(client, MessageContent::text(id))
                .await;
        }
    }

    async fn refresh<T: FirebaseApi + Sync>(
        &self,
        client: &Client,
        firebase_client: &T,
        host: &str,
    ) {
        // Add count
        self.refresh(client, firebase_client, host, |draw| match draw {
            None => "無予定".to_string(),
            Some(draw) => format!("予定中:{}", draw),
        })
        .await;
    }

    async fn try_draw<T: FirebaseApi + Sync>(
        &self,
        meal: Meal,
        client: &Client,
        firebase_client: &T,
        host: &str,
    ) {
        let (jar, draw) = get_current_draw(client, firebase_client).await;
        match draw {
            Ok(draw) => match draw {
                None => {
                    let draw = firebase_client.draw(&jar, meal).await;
                    let message = draw
                        .map(|res| {
                            res.map(|draw| {
                                MessageContent::text(&format!("「{}」が出ました", draw.name))
                                    .with_quick_replies(client, host, QuickReplyState::ActiveDraw)
                            })
                            .unwrap_or_else(|| {
                                MessageContent::text("何も出ませんでした").with_quick_replies(
                                    client,
                                    host,
                                    QuickReplyState::NoShops,
                                )
                            })
                        })
                        .unwrap_or_else(|e| MessageContent::error_message(&e));
                    let _ = self.send_to_all_users(client, message).await;
                }
                Some(draw) => {
                    let _ = self
                        .send_to_all_users(
                            client,
                            MessageContent::text(&format!("「{}」が既に出ています", draw.name))
                                .with_quick_replies(client, host, QuickReplyState::ActiveDraw),
                        )
                        .await;
                }
            },
            Err(e) => {
                let _ = self
                    .send_to_all_users(client, MessageContent::error_message(&e))
                    .await;
            }
        }
    }

    async fn postpone<T: FirebaseApi + Sync>(
        &self,
        client: &Client,
        firebase_client: &T,
        host: &str,
    ) {
        let (jar, draw) = get_current_draw(client, firebase_client).await;
        match draw {
            Ok(draw) => match draw {
                None => {
                    println!(
                        "Something is wrong here; tried to postpone the current shop but got no data"
                    );
                }
                Some(draw) => {
                    let _ = firebase_client.remove_drawn_place(&jar, Some(&draw)).await;
                    let _ = self
                        .send_to_all_users(
                            client,
                            MessageContent::text(&format!("{}を延期しました", &draw.name))
                                .with_quick_replies(client, host, QuickReplyState::Idle),
                        )
                        .await;
                }
            },
            Err(e) => {
                let _ = self
                    .send_to_all_users(client, MessageContent::error_message(&e))
                    .await;
            }
        }
    }

    async fn delete_current<T: FirebaseApi + Sync>(
        &self,
        client: &Client,
        firebase_client: &T,
        host: &str,
    ) {
        delete_current(client, self, firebase_client, host, |draw| {
            format!("「{}」を削除しました", &draw)
        })
        .await;
    }

    async fn archive_current<T: FirebaseApi + Sync>(
        &self,
        client: &Client,
        firebase_client: &T,
        host: &str,
    ) {
        delete_current(client, self, firebase_client, host, |draw| {
            format!("「{}」は完食になりました", &draw)
        })
        .await;
    }

    async fn add_place<T: FirebaseApi + Sync>(
        &self,
        client: &Client,
        firebase_client: &T,
        place_name: &str,
        meals: Vec<Meal>,
        host: &str,
    ) {
        let jar: Jar = client.into();
        let result = firebase_client.add_place(&jar, place_name, &meals).await;
        match result {
            Ok(_) => {
                self.refresh(client, firebase_client, host, |_| {
                    "新しい店が追加されました".to_string()
                })
                .await;
            }
            Err(e) => {
                let _ = self
                    .send_to_single_user(client, MessageContent::error_message(&e))
                    .await;
            }
        }
    }
}
