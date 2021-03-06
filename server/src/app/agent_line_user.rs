use async_trait::async_trait;

use crate::app::agent::Agent;
use crate::app::core::{Client, Meal, Place};
use crate::gcp::firebase::{FirebaseApi, Jar};
use crate::http::{Empty, HttpResult};
use crate::line::http::{LineChannel, LineClient};
use crate::line::json;
use crate::line::json::{MessageContent, QuickReply};

async fn get_current_draw(
    client: &Client,
    firebase_client: &reqwest::Client,
) -> (Jar, HttpResult<Option<String>>) {
    let jar: Jar = client.into();
    let draw = firebase_client.get_current_draw(&jar).await;
    (jar, draw)
}

impl Client {
    fn quick_replies(&self, host: &str, draw: Option<String>) -> Vec<QuickReply> {
        match draw {
            None => self.default_quick_replies(host),
            Some(_) => self.on_draw_quick_replies(host),
        }
    }

    fn default_quick_replies(&self, host: &str) -> Vec<QuickReply> {
        let add_place = self.add_place_quick_reply(host);
        vec![
            add_place,
            MessageContent::postback_quick_reply("引く(昼)", json::DRAW_LUNCH_ACTION),
            MessageContent::postback_quick_reply("引く(夜)", json::DRAW_DINNER_ACTION),
        ]
    }

    fn on_draw_quick_replies(&self, host: &str) -> Vec<QuickReply> {
        let add_place = self.add_place_quick_reply(host);
        vec![
            add_place,
            MessageContent::postback_quick_reply("完食", json::ARCHIVE_ACTION),
            MessageContent::postback_quick_reply("延期", json::POSTPONE_ACTION),
            MessageContent::postback_quick_reply("削除", json::DELETE_ACTION),
        ]
    }

    fn add_place_quick_reply(&self, host: &str) -> QuickReply {
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
        MessageContent::uri_quick_reply("追加", &uri.to_string())
    }
}

async fn delete_current<F: FnOnce(String) -> String>(
    client: &Client,
    line_client: &LineClient,
    firebase_client: &reqwest::Client,
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
                firebase_client
                    .delete_place(&jar, Place { name: draw.clone() })
                    .await;
                line_client
                    .send_to_all_users(
                        client,
                        MessageContent::text(&message_formatter(draw))
                            .with_quick_replies(client.default_quick_replies(host)),
                    )
                    .await;
            }
        },
        Err(e) => {
            line_client
                .send_to_all_users(client, MessageContent::error_message(&e))
                .await;
        }
    }
}

impl LineClient {
    async fn refresh<F: FnOnce(&Option<String>) -> String>(
        &self,
        client: &Client,
        firebase_client: &reqwest::Client,
        host: &str,
        message: F,
    ) {
        // Add count
        let (_jar, draw) = get_current_draw(client, firebase_client).await;
        let message = draw
            .map(|res| {
                let text_message = message(&res);
                res.map(|_draw| {
                    MessageContent::text(&text_message)
                        .with_quick_replies(client.on_draw_quick_replies(host))
                })
                .unwrap_or(
                    MessageContent::text(&text_message)
                        .with_quick_replies(client.default_quick_replies(host)),
                )
            })
            .unwrap_or_else(|e| MessageContent::error_message(&e));
        self.send_to_all_users(client.into(), message).await;
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
                HttpResult::Ok(Empty {})
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
            self.send_to_single_user(client, MessageContent::text(&id))
                .await;
        }
    }

    async fn refresh(&self, client: &Client, firebase_client: &reqwest::Client, host: &str) {
        // Add count
        self.refresh(client, firebase_client, host, |draw| match draw {
            None => "無予定".to_string(),
            Some(draw) => format!("予定中:{}", draw),
        })
        .await;
    }

    async fn try_draw(
        &self,
        meal: Meal,
        client: &Client,
        firebase_client: &reqwest::Client,
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
                                MessageContent::text(&format!("「{}」が出ました", draw))
                                    .with_quick_replies(client.on_draw_quick_replies(host))
                            })
                            .unwrap_or(
                                MessageContent::text("何も出ませんでした")
                                    .with_quick_replies(vec![client.add_place_quick_reply(host)]),
                            )
                        })
                        .unwrap_or_else(|e| MessageContent::error_message(&e));
                    self.send_to_all_users(client.into(), message).await;
                }
                Some(draw) => {
                    self.send_to_all_users(
                        client.into(),
                        MessageContent::text(&format!("「{}」が既に出ています", draw))
                            .with_quick_replies(client.on_draw_quick_replies(host)),
                    )
                    .await;
                }
            },
            Err(e) => {
                self.send_to_all_users(client.into(), MessageContent::error_message(&e))
                    .await;
            }
        }
    }

    async fn postpone(&self, client: &Client, firebase_client: &reqwest::Client, host: &str) {
        let (jar, draw) = get_current_draw(client, firebase_client).await;
        match draw {
            Ok(draw) => match draw {
                None => {
                    println!(
                        "Something is wrong here; tried to postpone the current shop but got no data"
                    );
                }
                Some(draw) => {
                    firebase_client
                        .remove_drawn_place(&jar, Some(Place { name: draw.clone() }))
                        .await;
                    self.send_to_all_users(
                        client.into(),
                        MessageContent::text(&format!("{}を延期しました", &draw))
                            .with_quick_replies(client.default_quick_replies(host)),
                    )
                    .await;
                }
            },
            Err(e) => {
                self.send_to_all_users(client.into(), MessageContent::error_message(&e))
                    .await;
            }
        }
    }

    async fn delete_current(&self, client: &Client, firebase_client: &reqwest::Client, host: &str) {
        delete_current(client, self, firebase_client, host, |draw| {
            format!("「{}」を削除しました", &draw)
        })
        .await;
    }

    async fn archive_current(
        &self,
        client: &Client,
        firebase_client: &reqwest::Client,
        host: &str,
    ) {
        delete_current(client, self, firebase_client, host, |draw| {
            format!("「{}」は完食になりました", &draw)
        })
        .await;
    }

    async fn add_place(
        &self,
        client: &Client,
        firebase_client: &reqwest::Client,
        place: Place,
        meals: Vec<Meal>,
        host: &str,
    ) {
        let jar: Jar = client.into();
        let result = firebase_client.add_place(&jar, place, meals).await;
        match result {
            Ok(_) => {
                self.refresh(client, firebase_client, host, |_| {
                    "新しい店が追加されました".to_string()
                })
                .await;
            }
            Err(e) => {
                self.send_to_single_user(client, MessageContent::error_message(&e))
                    .await;
            }
        }
    }
}
