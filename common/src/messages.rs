use crate::error::ServerError;
use serde::{Deserialize, Serialize};

use crate::framing::Frameable;
use macros::Frame;

#[derive(Serialize, Deserialize, Debug, Frame, Clone)]
pub enum RequestType {
    Ping(String),
    NewServer(String),
    /// Username, Password
    SignIn(String, String),
    /// Username, Password
    SignUp(String, String),
    /*
    SendMessage(Message),
    GetMessages(ChannelId),
    GetChannels(ServerId),
    NewChannel(ServerId, String),
    GetFriends,
    AddFriend(UserId),*/
}

#[derive(Serialize, Deserialize, Debug, Frame)]
pub struct Request {
    pub tp: RequestType,
    pub session_cookie: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Frame)]
pub enum Response {
    Pong(String),
    Error(ServerError),
    SessionCreated(String),
    Success,
}

impl Request {
    pub fn new(tp: RequestType, session_cookie: Option<String>) -> Self {
        Self { tp, session_cookie }
    }
}
