use common::{connection::Connection, framing::Frameable};
use tokio::net::TcpStream;
use anyhow::Result;
use serde::de::DeserializeOwned;

pub struct AuthConnection{
    auth: bool,
    conn: Connection,
}

impl AuthConnection{
    pub fn new(stream: TcpStream) -> Self{
        Self{auth: false, conn: Connection::new(stream)}
    }

    pub fn authenticate(&mut self) {
        self.auth = true;
    }

    pub async fn read<T>(&mut self) ->Result<T> 
    where T: DeserializeOwned + Frameable{
        self.conn.read().await?;
        unimplemented!()
    }
}
