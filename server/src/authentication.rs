use anyhow::Result;
use common::id::ID;
use mongodb::bson::oid::ObjectId;
use std::sync::Arc;

use crate::{session::SessionHandler, user::UserHandler};

#[derive(Clone)]
pub struct AuthHandler {
    pub session_handler: Arc<SessionHandler>,
    pub user_handler: Arc<UserHandler>,
}

impl AuthHandler {
    pub fn new(session_handler: Arc<SessionHandler>, user_handler: Arc<UserHandler>) -> Self {
        Self {
            session_handler,
            user_handler,
        }
    }

    /// creates new user and starts a session with ID
    pub async fn signup(&self, username: String, password: String) -> Result<ID> {
        let oid = self
            .user_handler
            .create_new_user(username, password, true)
            .await?;
        self.session_handler.start_session(oid).await?;
        let id = ID::new(oid.to_hex()).unwrap();
        Ok(id)
    }

    // using id to sign in is not optimal, TODO: make email sign in
    ///returns faslse if credentials are wrong or user doesn't exist'
    pub async fn signin_by_id(&self, username: &str, password: &str, id: ID) -> Result<bool> {
        let oid = ObjectId::parse_str(id.id)?;
        if !self
            .user_handler
            .check_user_credentials(oid, username, password)
            .await?
        {
            return Ok(false);
        }
        self.user_handler.set_user_status(oid, true).await?;
        self.session_handler.start_session(oid).await?;
        Ok(true)
    }

    pub async fn signout(&self, id: ID) -> Result<()> {
        let oid = ObjectId::parse_str(id.id)?;
        self.session_handler.delete_session(oid).await?;
        self.user_handler
            .set_user_status(oid, false)
            .await
            .expect("should not be None as Session exists for user id");
        Ok(())
    }

    pub async fn check_authentication(&self, cookie: ID) -> Result<bool> {
        let oid = ObjectId::parse_str(cookie.id)?;
        self.session_handler.check_session_active(oid).await
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
        let uhandler = Arc::new(UserHandler::from_names(&client, "TEST", "users"));
        let shandler = Arc::new(SessionHandler::from_names(&client, "TEST", "sessions"));
        let handler = AuthHandler::new(shandler, uhandler);

        let id = handler
            .signup("TUser".to_string(), "Password123".to_string())
            .await
            .unwrap();
        let oid = ObjectId::parse_str(id.id.clone()).unwrap();
        assert!(handler.check_authentication(id.clone()).await.unwrap());
        handler.signout(id.clone()).await.unwrap();
        assert!(!handler.check_authentication(id.clone()).await.unwrap());

        assert!(handler
            .signin_by_id("TUser", "Password123", id.clone())
            .await
            .unwrap());
        assert!(!handler
            .signin_by_id("TUser", "Password", id.clone())
            .await
            .unwrap());
        assert!(!handler
            .signin_by_id("Us", "Password123", id.clone())
            .await
            .unwrap());
        assert!(!handler
            .signin_by_id(
                "TUser",
                "Password123",
                ID {
                    id: "124123123123123123123123".to_string()
                }
            )
            .await
            .unwrap());
        handler.signout(id.clone()).await.unwrap();

        client.database("TEST").drop(None).await.unwrap();
    }
}
