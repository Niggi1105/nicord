use anyhow::Result;
use common::connection::Connection;
use log::error;
use mongodb::Client;
use tokio::net::TcpStream;

use common::error::ServerError;
use common::messages::{Request, RequestType, Response};

use crate::handler::Handler;

///match the request and make appropriate calls to the handler
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

        RequestType::SendMessage(server_id, channel_name, message_content) => match request.session_cookie {
            None => Response::Error(ServerError::PermissionDenied),
            Some(cookie) => {
                handler.send_message(&mongo_client, cookie, &server_id, channel_name, message_content).await?
            }
        }

        RequestType::GetMessages(server_id, channel_name, block_nr) => match request.session_cookie{
            None => Response::Error(ServerError::PermissionDenied),
            Some(cookie) => {
                handler.get_message_block(&mongo_client, cookie, &server_id, channel_name, block_nr).await?
            }
        }
    })
}

///fetch the request from the Connection, if it couldn't be fetched return an error'
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

///creates new connection from Stream and does all opperations on it
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
    use common::messages::{Request, RequestType, Message};
    use tokio::test;
    use super::*;

    use crate::{mongodb::connect_mongo, handler::Handler, user::UserHandler, session::SessionHandler};

    #[test]
    async fn happy_path(){
        let client = connect_mongo(None).await.unwrap();
        let request_type = RequestType::SignUp("TEST User".to_string(), "TEST User Password".to_string());
        let mut request = Request { tp: request_type, session_cookie: None};
        let test_db = client.database("TEST_DB");
        let handler = Handler::new(SessionHandler::from_names(&client, "TEST_DB", "SESSIONS"), UserHandler::from_names(&client, "TEST_DB", "USERS"));
        let resp = process_request(client.clone(), request, handler.clone()).await.unwrap();
        let token = match resp {
            Response::SessionCreated(token) => token,
            other => {
                test_db.drop(None).await.unwrap();
                panic!("unexpected enum variant: {:?}", other);
            }
        };

        let server_name = "TEST_SERVER".to_string();
        request = Request::new(RequestType::NewServer(server_name.clone()), Some(token.clone()));
        let server_id = match process_request(client.clone(), request, handler.clone()).await.unwrap(){
            Response::ServerCreated(sid) => sid,
            other => {
                test_db.drop(None).await.unwrap();
                panic!("unexpected enum variant: {:?}", other);
            }
        };

        let channel_name = "TESTChannel".to_string();
        request = Request::new(RequestType::NewChannel(server_id.clone(), channel_name.clone()), Some(token.clone()));
        assert!(process_request(client.clone(), request, handler.clone()).await.unwrap().succeeded());

        let content = "This is a test message".to_string();
        request = Request::new(RequestType::SendMessage(server_id.clone(), channel_name.clone(), content.clone()), Some(token.clone()));
        assert!(process_request(client.clone(), request, handler.clone()).await.unwrap().succeeded());

        
        request = Request::new(RequestType::GetMessages(server_id.clone(), channel_name.clone(), 0), Some(token.clone()));
        match process_request(client.clone(), request, handler.clone()).await.unwrap() {
            Response::MessagesFound(messages) => {
                assert_eq!(messages.len(), 2);
                assert_eq!(messages[0], Message::new("channel created...".to_string(), "SERVER".to_string()));
                assert_eq!(messages[1], Message::new(content, "TEST User".to_string()));
            }
            other => {
                panic!("unexpected enum variant: {:?}", other);
            }
        }

        request = Request::new(RequestType::DeleteServer(server_id), Some(token.clone()));
        assert!(process_request(client.clone(), request, handler.clone()).await.unwrap().succeeded());

        test_db.drop(None).await.unwrap();
    }
}
