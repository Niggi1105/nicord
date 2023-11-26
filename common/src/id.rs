use crate::framing::Frameable;
use macros::Frame;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Frame, Clone)]
pub struct ID{
    pub id: String
}

impl ToString for ID{
    fn to_string(&self) -> String {
        self.id.clone()
    }
}

impl ID{
    pub fn new(id: String) -> Option<Self>{
        if id.len() != 24{
            return None;
        }

        let valid = ['0','1','2','3','4','5','6','7','8','9','a','b','c','d','e','f'];
        'outer: for c in id.chars(){
            for a in valid{
                if c == a {
                    continue 'outer;
                }
            }
            return None;
        }

        Some(Self { id })
    }
}
