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

    #[tokio::test]
    async fn test_signup() {
        let rt = RequestType::SignUp("Nikitsa".to_string(), "Passwort123".to_string());
        let r = Request::new(rt.clone(), None);
        let mut conn = connect_dc_server().await.unwrap();
        conn.write(r).await.unwrap();
        let rsp = conn.read::<Response>().await.unwrap();
        match rsp {
            Response::SessionCreated(cookie) => {
                println!("cookie: {:?}", cookie);
            }
            Response::Error(e) => {
                match e{
                    common::error::ServerError::InvalidCredentials => {}
                    _other => panic!("unexpected Error response from Server")
                }
            },
            _other => panic!("Invalid response from Server"),
        }
    }

    #[tokio::test]
    async fn test_signin() {
        let rt = RequestType::SignIn("Niklas".to_string(), "Passwort123".to_string());
        let r = Request::new(rt.clone(), None);
        let mut conn = connect_dc_server().await.unwrap();
        conn.write(r).await.unwrap();
        let rsp = conn.read::<Response>().await.unwrap();
        match rsp {
            Response::SessionCreated(cookie) => {
                println!("cookie: {:?}", cookie);
            }
            Response::Error(e) => panic!("serverside error: {:?}", e),
            _other => panic!("Invalid response from Server"),
        }
    }

    #[tokio::test]
    async fn test_auth_connection() {
        let rt = RequestType::SignIn("test_use".to_string(), "Passwort123".to_string());
        let r = Request::new(rt.clone(), None);
        let mut conn = connect_dc_server().await.unwrap();
        conn.write(r).await.unwrap();
        let rsp = conn.read::<Response>().await.unwrap();
        let cookie = match &rsp {
            Response::SessionCreated(cookie) => {
                cookie
            } 
            _other => {
                panic!("Invalid response from Server: {:?}", &rsp);
            }
        };
        let rqt = RequestType::NewServer("My Server".to_string());
        let rq = Request::new(rqt, Some(cookie.clone()));
        conn = connect_dc_server().await.unwrap();
        conn.write(rq).await.unwrap();
        let resp: Response = conn.read().await.unwrap();
        match &resp {
            Response::Success => {},
            _other => panic!("Invaid response from Server: {:?}", resp) 
        }
    }
}
