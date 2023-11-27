use anyhow::Result;
use common::connection::Connection;
use log::{debug, error, info, warn};
use mongodb::Client;
use tokio::net::TcpStream;

use common::error::ServerError;
use common::messages::{Request, RequestType, Response};

use crate::authentication::AuthHandler;

async fn create_new_server(mongo_client: Client, name: String) -> Result<Response> {
    Ok(Response::Success)
}

///does server intern request processing of the request and returns an appropriate response
async fn process_request(
    mongo_client: Client,
    request: Request,
    auth_handler: AuthHandler,
) -> Result<Response> {
    Ok(match request.tp {
        RequestType::Ping(txt) => Response::Pong(txt),
        RequestType::SignUp(username, password) => {
            let id = auth_handler.signup(username, password).await?;
            Response::SessionCreated(id)
        }
        RequestType::SignIn(username, password, id) => {
            if auth_handler.signin_by_id(username, password, &id).await? {
                Response::Error(ServerError::InvalidCredentials)
            } else {
                Response::SessionCreated(id)
            }
        }
        RequestType::SignOut(id) => {
            auth_handler.signout(id).await?;
            Response::Success
        }
        RequestType::NewServer(name) => create_new_server(mongo_client, name).await?,
    })
}

/// fetch the request from the Connection, if it couldn't be fetched return an error'
async fn fetch_request(conn: &mut Connection) -> Request {
    match conn.read().await {
        Err(e) => {
            error!("encountered an error trying to fetch the request: {:?}", e);
            conn.write(Response::Error(ServerError::BadRequest))
                .await
                .unwrap();
            panic!()
        }
        Ok(val) => val,
    }
}

/// creates new connection from Stream and does all opperations on it
async fn handler(stream: TcpStream, mongo_client: Client, auth_handler: AuthHandler) {
    let mut conn = Connection::new(stream);
    let request = fetch_request(&mut conn).await;
    let response = process_request(mongo_client, request, auth_handler)
        .await
        .unwrap_or(Response::Error(ServerError::InternalServerError));
    conn.write(response).await.unwrap();
}

pub async fn accept_new_connections(mongo_client: Client, auth_handler: AuthHandler) -> Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8087").await?;
    loop {
        let (socket, addr) = listener.accept().await?;
        info!("New connection from {:?}", addr);
        let cl = mongo_client.clone();
        let ah = auth_handler.clone();
        tokio::task::spawn(async move {
            handler(socket, cl, ah).await;
        });
    }
}



