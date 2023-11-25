use std::io::Write;

use crate::error::ServerError;
use serde::{Deserialize, Serialize};

use crate::framing::Frameable;
use macros::Frame;

#[derive(Serialize, Deserialize, Debug, Frame, Clone)]
pub struct Cookie {
    cookie: String,
}

#[derive(Serialize, Deserialize, Debug, Frame, Clone)]
pub enum RequestType {
    Ping(String),
    NewServer(String),
    /// Usernaem, Password
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
    session_cookie: Option<Cookie>,
}

#[derive(Serialize, Deserialize, Debug, Frame)]
pub enum Response {
    Pong(String),
    Error(ServerError),
    SignIn(Cookie),
    Success,
}

impl Request {
    pub fn new(tp: RequestType, session_cookie: Option<Cookie>) -> Self {
        Self { tp, session_cookie }
    }

    pub fn get_type_ref(&self) -> &RequestType {
        &self.tp
    }

    pub fn get_type(&self) -> RequestType {
        self.tp.clone()
    }

    pub fn get_cookie_ref(&self) -> &Option<Cookie> {
        &self.session_cookie
    }

    pub fn get_cookie(&self) -> Option<Cookie> {
        self.session_cookie.clone()
    }
}

impl Cookie {
    pub fn to_string(&self) -> &String {
        &self.cookie
    }

    pub fn from_string(s: String) -> Self {
        Self { cookie: s }
    }
}
