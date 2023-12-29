use anyhow::Result;
use common::connection::Connection;
use common::id::ID;
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
    let resp = send_request(conn, Request::new(ping, None)).await?;
    let d = ts.elapsed().as_micros();
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

pub async fn send_request(conn: &mut Connection, req: Request) -> Result<Response>{
    conn.write(req).await?;
    conn.read().await
}


pub async fn sign_up(conn: &mut Connection, username: String, password: String) -> Result<Response>{
    let req_tp = RequestType::SignUp(username, password);
    send_request(conn, Request::new(req_tp, None)).await
}

pub async fn sign_in(conn: &mut Connection, username: String, password: String, user_id: ID) -> Result<Response>{
    let req_tp = RequestType::SignIn(username, password, user_id);
    send_request(conn, Request::new(req_tp, None)).await
}

pub async fn signout(conn: &mut Connection, session_id: ID) -> Result<Response> {
    let req_tp = RequestType::SignOut();
    send_request(conn, Request::new(req_tp, Some(session_id))).await
}

pub async fn create_new_server(conn: &mut Connection, server_name: String, session_id: ID) -> Result<Response> {
    let req_tp = RequestType::NewServer(server_name);
    send_request(conn, Request::new(req_tp, Some(session_id))).await
}

pub async fn delete_server(conn: &mut Connection, server_id: ID, session_id: ID) -> Result<Response>{
    let req_tp = RequestType::DeleteServer(server_id);
    send_request(conn, Request::new(req_tp, Some(session_id))).await
}

pub async fn new_channel(conn: &mut Connection, server_id: ID, channel_name: String, session_id: ID) -> Result<Response> {
    let req_tp = RequestType::NewChannel(server_id, channel_name);
    send_request(conn, Request::new(req_tp, Some(session_id))).await
}

pub async fn delete_channel(conn: &mut Connection, server_id: ID, channel_name: String, session_id: ID) -> Result<Response> {
    let req_tp = RequestType::DeleteChannel(server_id, channel_name);
    send_request(conn, Request::new(req_tp, Some(session_id))).await
}

pub async fn get_channels(conn: &mut Connection, server_id: ID, session_id: ID) -> Result<Response> {
    let req_tp = RequestType::GetChannels(server_id);
    send_request(conn, Request::new(req_tp, Some(session_id))).await
}

pub async fn send_message(conn: &mut Connection, server_id: ID, channel_name: String, message_content: String, session_id: ID) -> Result<Response> {
    let req_tp = RequestType::SendMessage(server_id, channel_name, message_content);
    send_request(conn, Request::new(req_tp, Some(session_id))).await
}

pub async fn get_messages(conn: &mut Connection, server_id: ID, channel_name: String, block_nr: u32, session_id: ID) -> Result<Response> {
    let req_tp = RequestType::GetMessages(server_id, channel_name, block_nr);
    send_request(conn, Request::new(req_tp, Some(session_id))).await
}

#[cfg(test)]
mod test {
    use std::{sync::Arc, clone, time::Duration};

    use tokio::{test, sync::Mutex};
    use super::*;

    #[test]
    async fn test_ping(){
        let avg = Arc::new(Mutex::new(0));
        for _ in 0..100{
            let av = Arc::clone(&avg);
            tokio::spawn(async move{
                let mut conn = connect_dc_server() .await.unwrap();
                let data = "TEST_DATA123".to_string(); 
                let dp = ping(&mut conn, data).await.unwrap();
                *av.lock().await += dp;
            });
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
        panic!("avg: {:?}", *avg.lock().await / 1000);
    }
}
