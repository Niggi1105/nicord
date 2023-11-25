use crate::error::ServerError;
use serde::{Deserialize, Serialize};

use crate::framing::Frameable;
use macros::Frame;

#[derive(Serialize, Deserialize, Debug, Frame)]
pub struct Password {
    pwd: String,
}

#[derive(Serialize, Deserialize, Debug, Frame)]
pub struct Username {
    name: String,
}

#[derive(Serialize, Deserialize, Debug, Frame)]
pub struct Cookie {
    cookie: String,
}


#[derive(Serialize, Deserialize, Debug, Frame)]
pub enum Request {
    Ping(String),
    NewServer(String),
    SignIn(Username, Password),
    SignUp(Username, Password),
    /*
    SendMessage(Message),
    GetMessages(ChannelId),
    GetChannels(ServerId),
    NewChannel(ServerId, String),
    GetFriends,
    AddFriend(UserId),*/
}

#[derive(Serialize, Deserialize, Debug, Frame)]
pub enum Response {
    Pong(String),
    Error(SnameerverError),
    SignIn(Cookie),
    Success,
}

impl Password {
    pub fn new(pwd: String) -> Self {
        Self { pwd }
    }
}
