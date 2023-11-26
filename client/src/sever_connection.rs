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
        let rt = RequestType::SignUp("Test".to_string(), "Passwort123".to_string());
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
        let rt = RequestType::SignIn("Test1".to_string(), "Test123".to_string() );
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
    async fn bad_signin_request(){
        let rt = RequestType::SignIn("Test3".to_string(), "Test123".to_string());
        let r = Request::new(rt.clone(), None);
        let mut conn = connect_dc_server().await.unwrap();
        conn.write(r).await.unwrap();
        let rsp = conn.read::<Response>().await.unwrap();
        match &rsp {
            Response::Error(e) => {
                match &e{
                    common::error::ServerError::BadRequest => {},
                    _other => panic!("Unexpected server error: {:?}", e),
                }
            },
            _other => panic!("Invalid response from Server: {:?}", rsp),
        }
    }

    #[tokio::test]
    async fn test_authentication() {
        let rt = RequestType::SignIn("Test2".to_string(), "Test123".to_string());
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

    #[tokio::test]
    async fn test_signout(){
        let rt = RequestType::SignIn("Test4".to_string(), "Test123".to_string());
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
        let rqt = RequestType::SignOut(cookie.to_string());
        let rq = Request::new(rqt, Some(cookie.to_string()));
        conn = connect_dc_server().await.unwrap();
        conn.write(rq).await.unwrap();
        let resp: Response = conn.read().await.unwrap();
        match &resp {
            Response::Success => {},
            _other => panic!("Invaid response from Server: {:?}", resp) 
        }
    }

    #[tokio::test]
    async fn test_bad_signout_invalid_cookie(){
        let cookie = "3524dfe290c";
        let rqt = RequestType::SignOut(cookie.to_string());
        let rq = Request::new(rqt, Some(cookie.to_string()));
        let mut conn = connect_dc_server().await.unwrap();
        conn.write(rq).await.unwrap();
        let resp: Response = conn.read().await.unwrap();
        match &resp {
            Response::Error(e) => {
                match e {
                    ServerError::BadRequest => { }
                    _other => panic!("unexpected server Error: {:?}", e)
                }
            },
            _other => panic!("Invaid response from Server: {:?}", resp) 
        }
    }
}
