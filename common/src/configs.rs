use crate::framing::Frameable;
use macros::Frame;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Frame)]
pub struct ServerConfig {
    priviledges: Vec<Priviledge>,
}

#[derive(Debug, Serialize, Deserialize, Frame)]
pub struct Priviledge {
    name: String,
    level: u8,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let mut priveledges = Vec::new();
        priveledges.push(Priviledge {
            name: "admin".to_string(),
            level: 0,
        });
        Self {
            priviledges: priveledges,
        }
    }
}
