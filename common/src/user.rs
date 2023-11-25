use crate::framing::Frameable;
use macros::Frame;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Frame, Clone)]
pub struct User {
    username: String,
    password: String,
    servers: Option<Vec<String>>,
}

impl User {
    pub fn new(username: String, password: String, servers: Option<Vec<String>>) -> Self {
        Self {
            username,
            password,
            servers,
        }
    }

    pub fn check_correct_pwd(&self, pwd: &str) -> bool {
        self.password == pwd
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_pwd_check() {
        let u = User {
            username: "Bob".to_string(),
            password: "#Passwort123".to_string(),
            servers: None,
        };
        assert!(u.check_correct_pwd("#Passwort123"));
        assert!(!u.check_correct_pwd("falsches Passwort"));
    }
}
