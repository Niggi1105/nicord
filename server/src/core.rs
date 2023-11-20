use anyhow::Result;
use log::{error, info};
use std::net::SocketAddr;
use tokio;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;

use crate::mongodb::Command;
use common::error::ServerError;
use common::framing::{Connection, Frameable};
use common::messages::{Request, Response};

async fn create_new_server(mongo_channel: Sender<Command>, name: String) -> Result<()> {
    mongo_channel.send(Command::NewServer(name)).await?;
    Ok(())
}

async fn process_request(
    conn: &mut Connection,
    addr: SocketAddr,
    mongo_channel: Sender<Command>,
    request: Request,
) -> Result<()> {
    info!("processing Request...");
    match request {
        Request::Ping(txt) => conn.write(Response::Pong(txt)).await?,
        Request::NewServer(name) => {
            create_new_server(mongo_channel, name).await?;
            conn.write(Response::Success).await?;
        }
    };
    Ok(())
}

async fn fetch_request(conn: &mut Connection, _addr: SocketAddr) -> Result<Request> {
    let request = conn.read().await?;
    Ok(request)
}

async fn handler(stream: TcpStream, addr: SocketAddr, mongo_channel: Sender<Command>) {
    let mut conn = Connection::new(stream);
    let request = match fetch_request(&mut conn, addr).await {
        Err(e) => {
            conn.write(Response::Error(ServerError::InternalServerError))
                .await
                .expect("can't write internal server error to connection");
            error!("{:?}: Can't fetch request: {:?}", addr.ip(), e);
            panic!();
        }
        Ok(request) => request,
    };
    process_request(&mut conn, addr, mongo_channel, request)
        .await
        .unwrap();
}

pub async fn accept_new_connections(mongo_channel: Sender<Command>) -> Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8087").await?;
    loop {
        let (socket, addr) = listener.accept().await?;
        let m_channel = mongo_channel.clone();
        info!("New connection from {:?}", addr);
        tokio::task::spawn(async move { handler(socket, addr, m_channel).await });
    }
}
