use anyhow::Result;
use common::connection::Connection;
use common::messages::{Request, RequestType, Response};
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
        Response::Error(e) => panic!("serverside error: {:?}", e),
        _other => panic!("invalid response from server"),
    }
    Ok(d)
}

#[cfg(test)]
mod test {
    use common::error::ServerError;

    use super::*;
}
