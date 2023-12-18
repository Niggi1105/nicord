use anyhow::Result;
use common::connection::Connection;
use log::error;
use mongodb::Client;
use tokio::net::TcpStream;

use common::error::ServerError;
use common::messages::{Request, RequestType, Response};

use crate::handler::Handler;

///does server intern processing of the request and returns an appropriate response
async fn process_request(
    mongo_client: Client,
    request: Request,
    handler: Handler,
) -> Result<Response> {
    Ok(match request.tp {
        RequestType::Ping(txt) => Response::Pong(txt),

        RequestType::SignUp(username, password) => {
            handler.signup(username, password).await?
        }

        RequestType::SignIn(username, password, id) => {
            handler.signin_by_id(&username, &password, id.clone()).await?
        }

        RequestType::SignOut() => match request.session_cookie{
            None => Response::Error(ServerError::BadRequest),
            Some(cookie) => handler.signout(cookie).await?
        }

        RequestType::NewServer(name) => match request.session_cookie {
            None => Response::Error(ServerError::PermissionDenied),
            Some(cookie) => {
                handler.create_new_server(&mongo_client, cookie, name).await?
            }
        },

        RequestType::DeleteServer(server_id) => match request.session_cookie {
            None => Response::Error(ServerError::PermissionDenied),
            Some(cookie) => {
                handler.delete_server(&mongo_client, cookie, &server_id).await?
            }
        },

        RequestType::NewChannel(server_id, name) => match request.session_cookie {
            None => Response::Error(ServerError::PermissionDenied),
            Some(cookie) => {
                handler.new_channel(&mongo_client, cookie, &name, &server_id).await?
            }
        }

        RequestType::DeleteChannel(server_id, name) => match request.session_cookie{
            None => Response::Error(ServerError::PermissionDenied),
            Some(cookie) => {
                handler.delete_channels(&mongo_client, cookie, &name, &server_id).await?
            }
        }

        RequestType::GetChannels(server_id) => match request.session_cookie{
            None => Response::Error(ServerError::PermissionDenied),
            Some(cookie) => {
                handler.get_channels(&mongo_client, cookie, &server_id).await?
            }
        }
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
async fn handler_fn(stream: TcpStream, mongo_client: Client, handler: Handler) {
    let mut conn = Connection::new(stream);
    let request = fetch_request(&mut conn).await;
    let response = process_request(mongo_client, request, handler)
        .await
        .unwrap_or(Response::Error(ServerError::InternalServerError));
    conn.write(response).await.unwrap();
}

pub async fn accept_new_connections(mongo_client: Client, handler: Handler) -> Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8087").await?;
    loop {
        let (socket, _addr) = listener.accept().await?;
        let cl = mongo_client.clone();
        let ah = handler.clone();
        tokio::task::spawn(async move {
            handler_fn(socket, cl, ah).await;
        });
    }
}

#[cfg(test)]
mod test {
    use tokio::test;

    #[test]
    async fn happy_path_server_creation_and_deletion(){

    }
}
