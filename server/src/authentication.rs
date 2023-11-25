use anyhow::{anyhow, Result, bail};
use common::{
    connection::Connection,
    messages::{Cookie, Request, RequestType, Response},
};
use log::{error, info};
use mongodb::{bson::doc, Client, Collection};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, time};
use tokio::net::TcpStream;

pub struct AuthConnection {
    conn: Connection,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    _id: Cookie,
    user_name: String,
    start: time::SystemTime,
}

pub enum AuthError{
    SessionExpired,
    InvalidCookie
}

impl Session {
    pub fn new(user_name: String) -> Self {
        Self {
            _id: Cookie::from_string(mongodb::bson::oid::ObjectId::new().to_hex()),
            user_name,
            start: time::SystemTime::now(),
        }
    }

    fn is_expired(&self) -> bool {
        match self.start.elapsed() {
            Ok(val) => {
                return val.as_secs() > 600;
            }
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
        if r.get_cookie_ref().is_none() {
            return Ok((false, r.get_type()));
        }
        let coll: Collection<Session> = cl
            .database("Sessions")
            .collection(&r.get_cookie().expect("checked for none before").to_string()[0..4]);
        let rs = coll
            .find_one(
                doc! {"_id": r.get_cookie().expect("checked for none before").to_string()},
                None,
            )
            .await?;
        match rs {
            Some(session) => {
                if session.is_expired() {
                    info!("found expired session key");
                    coll.delete_one(
                        doc! {"_id": r.get_cookie().expect("checked for none before").to_string()},
                        None,
                    ).await?;
                    return Ok((false, r.get_type()));
                };
            }
            None => {
                return Ok((false, r.get_type()));
            }
        };
        Ok((true, r.get_type()))
    }
}

#[cfg(test)]
mod test{
    use super::*;
    use tokio::test;
    #[test]
    async fn test_establish_connection() {}
}
