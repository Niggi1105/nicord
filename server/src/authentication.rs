use anyhow::Result;
use common::{
    connection::Connection,
    error::ServerError,
    messages::{Request, RequestType, Response},
    user::User,
};
use log::{error, warn, debug};
use mongodb::{bson::doc, Client, Collection};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, time, net::IpAddr};
use tokio::net::TcpStream;

pub static SESSION_DATABASE: &str = "SESSIONS";
pub static SESSION_COLLECTION: &str = "sessions";
pub static USER_DATABASE: &str = "USERS";
pub static USER_COLLECTION: &str = "users";

pub struct AuthConnection {
    conn: Connection,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    _id: String,
    user_name: String,
    start: time::SystemTime,
}

pub async fn signup(
    username: String,
    password: String,
    _addr: &IpAddr,
    mongo_client: Client,
) -> Result<Response> {
    debug!("processing sign up request: {}, {}", username, password);
    let db = mongo_client.database(USER_DATABASE);
    let coll: Collection<User> = db.collection(USER_COLLECTION);
    coll.insert_one(User::new(username.clone(), password, None), None)
        .await?;
    let cookie = Session::start(mongo_client, username).await?;
    Ok(Response::SessionCreated(cookie))
}

pub async fn signin(
    username: String,
    password: String,
    _addr: &IpAddr,
    mongo_client: Client,
) -> Result<Response> {
    debug!("processing sign in request: {}, {}", username, password);
    let db = mongo_client.database(USER_DATABASE);
    let coll: Collection<User> = db.collection(USER_COLLECTION);
    if let Some(user) = coll.find_one(doc! {"username": &username}, None).await? {
        if user.check_correct_pwd(&password) {
            return Ok(Response::SessionCreated(
                Session::start(mongo_client, username).await?,
            ));
        }
    }
    Ok(Response::Error(ServerError::InvalidCredentials))
}

impl Session {
    pub fn new(user_name: String) -> Self {
        Self {
            _id: mongodb::bson::oid::ObjectId::new().to_hex(),
            user_name,
            start: time::SystemTime::now(),
        }
    }

    pub async fn start(client: Client, user_name: String) -> Result<String> {
        let db = client.database(SESSION_DATABASE);
        let coll = db.collection(SESSION_COLLECTION);
        let id = coll
            .insert_one(Session::new(user_name), None)
            .await?
            .inserted_id;
        Ok(id.as_str().unwrap().to_string())
    }

    fn is_expired(&self) -> bool {
        match self.start.elapsed() {
            Ok(val) => val.as_secs() > 600,
            Err(e) => {
                error!(
                    "encountered an error trying to get the expiration of session: {:?}",
                    e
                );
                panic!("can't compute elapsed time'")
            }
        }
    }
}

impl AuthConnection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            conn: Connection::new(stream),
        }
    }

    pub async fn write(&mut self, resp: Response) -> Result<()> {
        self.conn.write(resp).await?;
        Ok(())
    }

    pub async fn read_auth_req(&mut self, cl: &mut Client) -> Result<(bool, RequestType)> {
        let r = self.conn.read::<Request>().await?;
        if r.session_cookie.is_none() {
            debug!("no session cookie provided");
            return Ok((false, r.tp));
        }
        let coll: Collection<Session> = cl.database(SESSION_DATABASE).collection(SESSION_COLLECTION);
        let rs = coll
            .find_one(
                doc! {"_id": r.session_cookie.clone().expect("checked for none before")},
                None,
            )
            .await?;
        match rs {
            Some(session) => {
                if session.is_expired() {
                    warn!("session key has expired");
                    coll.delete_one(
                        doc! {"_id": r.session_cookie.expect("checked for none before")},
                        None,
                    )
                    .await?;
                    return Ok((false, r.tp));
                };
            }
            None => {
                warn!("session not found in database: {:?}", r.session_cookie);
                return Ok((false, r.tp));
            }
        };
        Ok((true, r.tp))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::test;
    #[test]
    async fn test_auth_req() {}
}
