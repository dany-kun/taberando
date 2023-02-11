use crate::app::core::Client;
use crate::app::core::Client::Line;
use crate::line::http::LineChannel;
use std::fmt::{Debug, Display, Formatter};

pub struct Jar(String);

impl Debug for Jar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl Display for Jar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

pub struct JarError;

impl From<std::io::Error> for JarError {
    fn from(_: std::io::Error) -> Self {
        JarError
    }
}

impl From<serde_json::Error> for JarError {
    fn from(_: serde_json::Error) -> Self {
        JarError
    }
}

impl From<&Client> for Jar {
    fn from(client: &Client) -> Self {
        let jar_key = match client {
            Line(channel) => match channel {
                LineChannel::User(id) => format!("user_{id}"),
                LineChannel::Room { id, .. } => format!("room_{id}"),
                LineChannel::Group { id, .. } => format!("group_{id}"),
            },
        };
        Jar::new(&jar_key)
    }
}

impl Jar {
    pub fn new(name: &str) -> Self {
        Jar(name.to_string())
    }

    pub fn line_channel(&self) -> Result<LineChannel, JarError> {
        let parts = self.0.split_once('_');
        parts.ok_or(JarError).and_then(|(prefix, id)| {
            let id = id.to_string();
            match prefix {
                "user" => Ok(LineChannel::User(id)),
                "group" => Ok(LineChannel::Group { id, user_id: None }),
                "room" => Ok(LineChannel::Room { id, user_id: None }),
                &_ => Err(JarError),
            }
        })
    }
}
