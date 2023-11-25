use anyhow::Result;
use log::{error, info, warn, debug};
use mongodb::Client;
use std::net::IpAddr;
use tokio::net::TcpStream;

use common::error::ServerError;
use common::messages::{RequestType, Response};

use crate::authentication::{signin, signup, AuthConnection};

async fn create_new_server(
    addr: &IpAddr,
    auth: bool,
    _mongo_client: Client,
    _name: String,
) -> Result<Response> {
    if !auth {
        warn!("{:?} Permission denied because of invalid authenication", addr);
        return Ok(Response::Error(ServerError::PermissionDenied));
    }
    Ok(Response::Success)
}

async fn process_request(
    conn: &mut AuthConnection,
    addr: &IpAddr,
    mongo_client: Client,
    request: RequestType,
    auth: bool,
) -> Result<()> {
    debug!("processing Request...");
    let resp = match request {
        RequestType::Ping(txt) => Ok(Response::Pong(txt)),
        RequestType::NewServer(name) => create_new_server(addr, auth, mongo_client, name).await,
        RequestType::SignUp(username, passwd) => signup(username, passwd, addr, mongo_client).await ,
        RequestType::SignIn(username, passwd) => signin(username, passwd, addr, mongo_client).await ,
    };
    conn.write( match resp {
        Err(e) => {
            error!("Internal Server error: {:?}", e);
            Response::Error(ServerError::InternalServerError)
        }
        Ok(r) => r
    }).await.unwrap();
    debug!("request processing complete");
    Ok(())
}

async fn fetch_request(
    conn: &mut AuthConnection,
    mongo_client: &mut Client,
    addr: &IpAddr,
) -> (bool, RequestType) {
    match conn.read_auth_req(mongo_client).await {
        Err(e) => {
            error!(
                "{:?}: encountered an error trying to fetch the request: {:?}",
                addr, e
            );
            conn.write(Response::Error(ServerError::InternalServerError))
                .await
                .unwrap();
            panic!()
        }
        Ok(val) => val,
    }
}

async fn handler(stream: TcpStream, addr: IpAddr, mut mongo_client: Client) {
    let mut conn = AuthConnection::new(stream);
    let (auth, request) = fetch_request(&mut conn, &mut mongo_client, &addr).await;
    process_request(&mut conn, &addr, mongo_client, request, auth)
        .await
        .unwrap();
}

pub async fn accept_new_connections(mongo_client: Client) -> Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8087").await?;
    loop {
        let (socket, addr) = listener.accept().await?;
        info!("New connection from {:?}", addr);
        let cl = mongo_client.clone();
        tokio::task::spawn(async move {
            handler(socket, addr.ip(), cl).await;
        });
    }
}
