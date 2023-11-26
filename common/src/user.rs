use serde::{Serialize, Deserialize};
use macros::Frame;
use crate::framing::Frameable;

#[derive(Debug, Serialize, Deserialize, Frame)]
pub struct User{
    username: String,
    is_online: bool,
    servers: Vec<String>,
}

impl User {
   pub fn new(username: String, is_online: bool, servers: Vec<String>) -> Self {
       Self { username, servers, is_online }
   }
}
