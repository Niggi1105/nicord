use anyhow::Result;
use common::connection::Connection;
use common::messages::{Request, Response, RequestType};
use std::time;
use tokio::net::TcpStream;

pub async fn connect_dc_server() -> Result<Connection> {
    let stream = TcpStream::connect("127.0.0.1:8087").await?;
    Ok(Connection::new(stream))
}

pub async fn ping(conn: &mut Connection, data: String) -> Result<u128> {
    let ping = RequestType::Ping(data.clone());
    let ts = time::Instant::now();
    conn.write(Request::new(ping, None)).await?;
    let resp = conn.read().await?;
    let d = ts.elapsed().as_millis();
    match resp {
        Response::Pong(tx) => {
            if tx != data {
                println!("invalid responsedata: {:?}", tx);
            }
        }
        Response::Error(e) => println!("serverside error: {:?}", e),
        Response::SignIn(cookie) => {}
        Response::Success => {}
        _other => unimplemented!()
    }
    Ok(d)
}

#[cfg(test)]
mod test {
    use super::*;
    #[tokio::test]
    async fn test_dc_connect() {
        let mut conn = connect_dc_server().await.unwrap();
        conn.shutdown().await.unwrap();
    }
    #[tokio::test]
    async fn test_ping() {
        let mut conn = connect_dc_server().await.unwrap();
        let t = ping(&mut conn, "hallo welt".to_string()).await.unwrap();
        conn.shutdown().await.unwrap();
        println!("test took {:?}ms", t);
    }
}
