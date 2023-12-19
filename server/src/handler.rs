use anyhow::Result;
use common::{id::ID, messages::Response};
use mongodb::{bson::oid::ObjectId, Client};

use crate::{server_handler::ServerHandler, session::SessionHandler, user::UserHandler};

#[derive(Clone)]
pub struct Handler {
    pub session_handler: SessionHandler,
    pub user_handler: UserHandler,
}

//authentication
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
            return Ok(Response::Error(
                common::error::ServerError::InvalidCredentials,
            ));
        }
        self.user_handler.set_user_status(oid, true).await?;
        self.session_handler.start_session(oid).await?;
        Ok(Response::Success)
    }

    ///deletes the session from session db and set the user status to inactive
    pub async fn signout(&self, id: ID) -> Result<Response> {
        let oid = ObjectId::parse_str(id.id)?;
        self.session_handler.end_session(oid).await?;
        self.user_handler.set_user_status(oid, false).await?;
        Ok(Response::Success)
    }

    async fn is_authenticated(&self, user_id: ID) -> Result<bool> {
        let oid = ObjectId::parse_str(user_id.id)?;
        Ok(self
            .session_handler
            .check_session_active(oid)
            .await?
            .succeeded())
    }
}

//server handler stuff
impl Handler {
    ///checks authentication and creates a new nicord server(Database)
    pub async fn create_new_server(
        &self,
        mongo_client: &Client,
        user_id: ID,
        name: String,
    ) -> Result<Response> {
        if !self.is_authenticated(user_id.clone()).await? {
            let oid = ObjectId::parse_str(user_id.id)?;
            return self.session_handler.check_session_active(oid).await;
        }
        ServerHandler::new_server(user_id, mongo_client, name).await
    }

    ///checks authentication and deletes a nicord server if the user has the required priviledges
    pub async fn delete_server(
        &self,
        mongo_client: &Client,
        user_id: ID,
        server_id: &ID,
    ) -> Result<Response> {
        if !self.is_authenticated(user_id.clone()).await? {
            let oid = ObjectId::parse_str(user_id.id)?;
            return self.session_handler.check_session_active(oid).await;
        }
        //delete the server database
        ServerHandler::delete_server(&user_id, mongo_client, server_id).await
    }

    ///creates a new channel(Collection) on a server if the user is authenticated and has the
    ///required priviledges
    pub async fn new_channel(
        &self,
        mongo_client: &Client,
        user_id: ID,
        name: &String,
        server_id: &ID,
    ) -> Result<Response> {
        if !self.is_authenticated(user_id.clone()).await? {
            let oid = ObjectId::parse_str(user_id.id)?;
            return self.session_handler.check_session_active(oid).await;
        }
        ServerHandler::new_channel(&user_id, mongo_client, name, server_id).await
    }

    ///deletes the channel if the user is authenticated and has the required priviledges
    ///returns bad request if channel does not exist
    pub async fn delete_channels(
        &self,
        mongo_client: &Client,
        user_id: ID,
        name: &String,
        server_id: &ID,
    ) -> Result<Response> {
        if !self.is_authenticated(user_id.clone()).await? {
            let oid = ObjectId::parse_str(user_id.id)?;
            return self.session_handler.check_session_active(oid).await; 
        }
        ServerHandler::delete_channel(&user_id, mongo_client, name, server_id).await
    }

    ///check authentication and priviledges, return a response containing a vec of the channelnames
    ///if the user has at least user priviledge on the server
    pub async fn get_channels(
        &self,
        mongo_client: &Client,
        user_id: ID,
        server_id: &ID,
    ) -> Result<Response> {
        if !self.is_authenticated(user_id.clone()).await? {
            let oid = ObjectId::parse_str(user_id.id)?;
            return self.session_handler.check_session_active(oid).await;
        }
        ServerHandler::get_channels(mongo_client, server_id, &user_id).await
    }

    ///send a message to the channel if the user is authenticated and has the required priviledges
    pub async fn send_message(
        &self,
        mongo_client: &Client,
        user_id: ID,
        server_id: &ID,
        channel_name: String,
        message_content: String,
    ) -> Result<Response> {
        let oid = ObjectId::parse_str(user_id.id.clone())?;
        if !self.is_authenticated(user_id.clone()).await? {
            return self.session_handler.check_session_active(oid).await;
        }
        let username = self.user_handler.get_user(oid).await?.expect("checked above").username;
        ServerHandler::send_message(mongo_client,  server_id, &channel_name, &user_id, message_content, username).await
    }

    ///get a block of messages from a channel if the user is authenticated and has the required
    ///priviledges. The block is uniquely identified by its id
    pub async fn get_message_block(
        &self, 
        mongo_client: &Client,
        user_id: ID,
        server_id: &ID,
        channel_name: String,
        block_id: u32,
    ) -> Result<Response> {
        if !self.is_authenticated(user_id.clone()).await? {
            let oid = ObjectId::parse_str(user_id.id.clone())?;
            return self.session_handler.check_session_active(oid).await;
        }
        ServerHandler::get_block_content(mongo_client, server_id, &channel_name, &user_id, block_id).await
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
        let uhandler = UserHandler::from_names(&client, "TESTAUTH", "users");
        let shandler = SessionHandler::from_names(&client, "TESTAUTH", "sessions");
        let handler = Handler::new(shandler, uhandler);

        let resp = handler
            .signup("TUser".to_string(), "Password123".to_string())
            .await
            .unwrap();

        let id = match resp {
            Response::SessionCreated(id) => id,
            _other => panic!("invalid response"),
        };
        assert!(handler.is_authenticated(id.clone()).await.unwrap());
        handler.signout(id.clone()).await.unwrap();
        assert!(!handler.is_authenticated(id.clone()).await.unwrap());

        assert!(handler
            .signin_by_id("TUser", "Password123", id.clone())
            .await
            .unwrap()
            .succeeded());
        assert!(!handler
            .signin_by_id("TUser", "Password", id.clone())
            .await
            .unwrap()
            .succeeded());
        assert!(!handler
            .signin_by_id("Us", "Password123", id.clone())
            .await
            .unwrap()
            .succeeded());
        assert!(!handler
            .signin_by_id(
                "TUser",
                "Password123",
                ID {
                    id: "124123123123123123123123".to_string()
                }
            )
            .await
            .unwrap()
            .succeeded());
        handler.signout(id.clone()).await.unwrap();

        client.database("TESTAUTH").drop(None).await.unwrap();
    }
}
