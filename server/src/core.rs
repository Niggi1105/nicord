use anyhow::Result;
use common::user::User;
use log::{error, info};
use mongodb::bson::doc;
use mongodb::{Client, Collection};
use std::net::{IpAddr, SocketAddr};
use tokio::net::TcpStream;

use common::error::ServerError;
use common::messages::{Cookie, RequestType, Response};

use crate::authentication::AuthConnection;

async fn create_new_server(
    conn: &mut AuthConnection,
    addr: &IpAddr,
    auth: bool,
    mongo_client: Client,
    name: String,
) -> Result<()> {
    if !auth {
        info!(
            "{:?} Permission denied because of invalid authenication",
            addr
        );
        conn.write(Response::Error(ServerError::PermissionDenied))
            .await
            .unwrap();
        panic!("unauthenticated Server creation request")
    }
    unimplemented!()
}

async fn signup(
    username: String,
    password: String,
    addr: &IpAddr,
    conn: &mut AuthConnection,
    mongo_client: Client,
) -> Result<Cookie> {
    let db = mongo_client.database("Users");
    let coll: Collection<User> = db.collection("users");
    let user = coll.insert_one(User::new(username, password, None), None).await?;
    Ok(Cookie::from_string("Hallo".to_string()))
}

async fn process_request(
    conn: &mut AuthConnection,
    addr: &IpAddr,
    mongo_client: Client,
    request: RequestType,
    auth: bool,
) -> Result<()> {
    info!("processing RequestType...");
    match request {
        RequestType::Ping(txt) => conn.write(Response::Pong(txt)).await?,
        RequestType::NewServer(name) => {
            create_new_server(conn, addr, auth, mongo_client, name).await?;
            conn.write(Response::Success).await?;
        }
        RequestType::SignUp(username, passwd) => {
            signup(username, passwd, addr, conn, mongo_client).await?;
        }
        RequestType::SignIn(username, passwd) => {}
    };
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
