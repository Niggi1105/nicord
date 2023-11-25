use macros::Frame;
use serde::{Serialize, Deserialize};
use crate::framing::Frameable;


#[derive(Serialize, Deserialize, Debug, Frame, Clone)]
pub struct User {
    name: String,
    password: String,
    servers: Option<Vec<String>>,
}

impl User {
    pub fn new(name: String, password: String , servers: Option<Vec<String>>) -> Self {
        Self { name, password, servers }
    }
}
