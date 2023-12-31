use crate::{error::ServerError, id::ID};
use serde::{Deserialize, Serialize};

use crate::framing::Frameable;
use macros::Frame;

#[derive(Serialize, Deserialize, Debug, Frame, Clone)]
pub enum RequestType {
    Ping(String),
    /// Username, Password
    SignIn(String, String, ID),
    /// Username, Password
    SignUp(String, String),
    SignOut(),
    NewServer(String),
    DeleteServer(ID),
    NewChannel(ID, String),
    DeleteChannel(ID, String),
    GetChannels(ID),
    SendMessage(ID, String, String), //ServerId, Channelname, Message
    GetMessages(ID, String, u32), //ServerId, Channelname, block id
    /*
    SendMessage(Message),
    GetFriends,
    AddFriend(UserId),*/
}

#[derive(Serialize, Deserialize, Debug, Frame)]
pub struct Request {
    pub tp: RequestType,
    pub session_cookie: Option<ID>,
}

#[derive(Serialize, Deserialize, Debug, Frame)]
pub enum Response {
    Pong(String),
    Error(ServerError),
    SessionCreated(ID),
    ServerCreated(ID),
    ChannelList(Vec<String>),
    MessagesFound(Vec<Message>),
    EndOfChannel,
    Success,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Message {
    pub content: String,
    pub author: String,
}

impl Message {
    pub fn new(content: String, author: String) -> Self {
        Self { content, author }
    }
}

impl Response {
    pub fn succeeded(&self) -> bool {
        match self {
            Response::Success => true,
            _other => false,
        }
    }
}

impl Request {
    pub fn new(tp: RequestType, session_cookie: Option<ID>) -> Self {
        Self { tp, session_cookie }
    }
}
