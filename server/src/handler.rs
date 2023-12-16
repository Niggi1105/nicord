use anyhow::Result;
use common::{id::ID, messages::Response, error::ServerError};
use mongodb::{bson::oid::ObjectId, Client};

use crate::{session::SessionHandler, user::UserHandler, server_handler::{self, ServerHandler}};

#[derive(Clone)]
pub struct Handler {
    pub session_handler: SessionHandler,
    pub user_handler: UserHandler,
}

impl Handler {
    pub fn new(session_handler: SessionHandler, user_handler: UserHandler) -> Self {
        Self {
            session_handler,
            user_handler,
        }
    }

    /// creates new user and starts a session with ID
    pub async fn signup(&self, username: String, password: String) -> Result<Response> {
        let oid = self
            .user_handler
            .create_new_user(username, password, true)
            .await?;
        self.session_handler.start_session(oid).await?;
        let id = ID::new(oid.to_hex()).unwrap();
        Ok(Response::SessionCreated(id))
    }

    // using id to sign in is not optimal, TODO: make email sign in
    ///returns Response Error InvalidCredentials if credentials are wrong or user doesn't exist
    ///else returns Response Success
    pub async fn signin_by_id(&self, username: &str, password: &str, id: ID) -> Result<Response> {
        let oid = ObjectId::parse_str(id.clone().id)?;
        if !self
            .user_handler
            .check_user_credentials(oid, username, password)
            .await?
        {
            return Ok(Response::Error(common::error::ServerError::InvalidCredentials));
        }
        self.user_handler.set_user_status(oid, true).await?;
        self.session_handler.start_session(oid).await?;
        Ok(Response::Success)
    }

    pub async fn signout(&self, id: ID) -> Result<Response> {
        let oid = ObjectId::parse_str(id.id)?;
        self.session_handler.end_session(oid).await?;
        self.user_handler
            .set_user_status(oid, false)
            .await?;
        Ok(Response::Success)
    }

    async fn is_authenticated(&self, user_id: ID) -> Result<bool> {
        let oid = ObjectId::parse_str(user_id.id)?;
        Ok(self.session_handler.check_session_active(oid).await?.succeeded())
    }

    pub async fn create_new_server(&self, mongo_client: &Client, user_id: ID, name: String) -> Result<Response> {
        if !self.is_authenticated(user_id.clone()).await? {
            let oid = ObjectId::parse_str(user_id.id)?;
            return self.session_handler.check_session_active(oid).await;
        }
        ServerHandler::new_server(user_id, mongo_client, name).await?;
        Ok(Response::Success)
    }

    pub async fn delete_server(&self, mongo_client: &Client, user_id: ID, server_id: ID) -> Result<Response> {
        if !self.is_authenticated(user_id.clone()).await? {
            let oid = ObjectId::parse_str(user_id.id)?;
            return self.session_handler.check_session_active(oid).await;
        }
        //delete the server database
        let resp = ServerHandler::delete_server(user_id, mongo_client, server_id).await?;

        Ok(resp)
    }

    pub async fn new_channel(&self, mongo_client: &Client, user_id: ID, name: String, server_id: ID) -> Result<Response> {
        if !self.is_authenticated(user_id.clone()).await? {
            let oid = ObjectId::parse_str(user_id.id)?;
            return self.session_handler.check_session_active(oid).await;
        }
        ServerHandler::new_channel(user_id, mongo_client, name, server_id).await
    }
}

#[cfg(test)]
mod test {
    use crate::mongodb::connect_mongo;

    use super::*;
    use tokio::test;

    #[test]
    async fn test_auth() {
        let client = connect_mongo(None).await.unwrap();
        let uhandler = UserHandler::from_names(&client, "TEST", "users");
        let shandler = SessionHandler::from_names(&client, "TEST", "sessions");
        let handler = Handler::new(shandler, uhandler);

        let resp = handler
            .signup("TUser".to_string(), "Password123".to_string())
            .await
            .unwrap();

        let id = match resp {
            Response::SessionCreated(id) => {
                id
            }
            _other => panic!("invalid response")
        };
        assert!(handler.is_authenticated(id.clone()).await.unwrap());
        handler.signout(id.clone()).await.unwrap();
        assert!(!handler.is_authenticated(id.clone()).await.unwrap());

        println!("{:?}", handler.signin_by_id("TUser", "Password123", id.clone()).await);
        assert!(handler
            .signin_by_id("TUser", "Password123", id.clone())
            .await
            .unwrap().succeeded());
        assert!(!handler
            .signin_by_id("TUser", "Password", id.clone())
            .await
            .unwrap().succeeded());
        assert!(!handler
            .signin_by_id("Us", "Password123", id.clone())
            .await
            .unwrap().succeeded());
        assert!(!handler
            .signin_by_id(
                "TUser",
                "Password123",
                ID {
                    id: "124123123123123123123123".to_string()
                }
            )
            .await
            .unwrap().succeeded());
        handler.signout(id.clone()).await.unwrap();

        client.database("TEST").drop(None).await.unwrap();
    }
}
