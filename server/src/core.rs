use anyhow::Result;
use log::{error, info};
use std::net::SocketAddr;
use tokio;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;

use crate::mongodb::{Command, MongoConnection};
use common::connection::Connection;
use common::error::ServerError;
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

async fn fetch_request(conn: &mut Connection, addr: SocketAddr) -> Request {
    match conn.read().await {
        Err(e) => {
            conn.write(Response::Error(ServerError::InternalServerError))
                .await
                .expect("can't write internal server error to connection");
            error!("{:?}: Can't fetch request: {:?}", addr.ip(), e);
            panic!("Request could not be fetched");
        }
        Ok(request) => request,
    }
}

async fn handler(stream: TcpStream, addr: SocketAddr, mongo_channel: Sender<Command>) {
    let mut conn = Connection::new(stream);
    let request = fetch_request(&mut conn, addr).await;
    process_request(&mut conn, addr, mongo_channel, request)
        .await
        .unwrap();
}

pub async fn accept_new_connections(connection: MongoConnection) -> Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8087").await?;
    loop {
        let (socket, addr) = listener.accept().await?;
        info!("New connection from {:?}", addr);
        let channel = connection.new_channel();
        tokio::task::spawn(async move {
            handler(socket, addr, channel).await;
        });
    }
}
