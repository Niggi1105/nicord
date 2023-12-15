use serde::{Serialize, Deserialize};
use macros::Frame;
use crate::framing::Frameable;

#[derive(Debug, Serialize, Deserialize, Frame)]
pub struct User{
    pub username: String,
    pub is_online: bool,
}

impl User {
   pub fn new(username: String, is_online: bool) -> Self {
       Self { username,  is_online }
   }
}
