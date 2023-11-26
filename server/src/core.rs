use anyhow::Result;
use common::connection::Connection;
use log::{error, info, warn, debug};
use mongodb::Client;
use std::net::IpAddr;
use tokio::net::TcpStream;

use common::error::ServerError;
use common::messages::{RequestType, Response, Request};

use crate::authentication::AuthHandler;


async fn create_new_server(
    mongo_client: Client,
    name: String,
) -> Result<Response> {
    Ok(Response::Success)
}

async fn process_request(
    conn: &mut Connection,
    mongo_client: Client,
    request: Request,
    mut auth_handler: AuthHandler,
) -> Result<()> {
    let resp = match request.tp {
        RequestType::Ping(txt) => Ok(Response::Pong(txt)),
        RequestType::NewServer(name) => create_new_server(mongo_client, name).await,
        RequestType::SignUp(username, password) => {
            let id = auth_handler.signup(username, password).await?;
            Ok(Response::SessionCreated(id))
        } ,
        RequestType::SignIn(username, password, id) => {
            if auth_handler.signin_by_id(username, password, &id).await?{
                Ok(Response::Error(ServerError::InvalidCredentials))
            }else {
                Ok(Response::SessionCreated(id))
            }
        },
        RequestType::SignOut(id) => {
            if auth_handler.signout(id).await? {
                Ok(Response::Success)
            }else {
                Ok(Response::Error(ServerError::BadRequest))
            }
        },
    };

    conn.write( match resp {
        Err(e) => {
            error!("Internal Server error: {:?}", e);
            Response::Error(ServerError::InternalServerError)
        }
        Ok(r) => r
    }).await.unwrap();
    Ok(())
}

async fn fetch_request(
    conn: &mut Connection,
    addr: &IpAddr,
) -> Request {
    match conn.read().await {
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

async fn handler(stream: TcpStream, addr: IpAddr, mongo_client: Client, auth_handler: AuthHandler) {
    let mut conn = Connection::new(stream);
    let request = fetch_request(&mut conn, &addr).await;
    process_request(&mut conn, mongo_client, request, auth_handler).await.unwrap();
}

pub async fn accept_new_connections(mongo_client: Client, auth_handler: AuthHandler ) -> Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8087").await?;
    loop {
        let (socket, addr) = listener.accept().await?;
        info!("New connection from {:?}", addr);
        let cl = mongo_client.clone();
        let ah = auth_handler.clone();
        tokio::task::spawn(async move {
            handler(socket, addr.ip(), cl, ah).await;
        });
    }
}
