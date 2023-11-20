use crate::error::ServerError;
use serde::{Deserialize, Serialize};

use crate::framing::Frameable;
use macros::Frame;

#[derive(Serialize, Deserialize, Debug, Frame)]
pub enum Request {
    Ping(String),
    NewServer(String),
    /*
    SendMessage(Message),
    GetMessages(ChannelId),
    GetChannels(ServerId),
    NewChannel(ServerId, String),
    GetFriends,
    AddFriend(UserId),
    SignIn(UserId, Password),
    SignUp(String, Password),*/
}

#[derive(Serialize, Deserialize, Debug, Frame)]
pub enum Response {
    Pong(String),
    Error(ServerError),
    Success,
}
