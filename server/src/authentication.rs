use anyhow::Result;
use common::{id::ID};
use mongodb::bson::oid::ObjectId;
use std::sync::Arc;

use crate::{user::UserHandler, session::SessionHandler};

#[derive(Clone)]
pub struct AuthHandler{
    pub session_handler: Arc<SessionHandler>,
    pub user_handler: Arc<UserHandler>,
}

impl AuthHandler{
    pub fn new(session_handler: Arc<SessionHandler>, user_handler: Arc<UserHandler>) -> Self {
        Self { session_handler, user_handler }
    }

    /// creates new user and starts a session with ID
    pub async fn signup(&self, username: String, password: String) -> Result<ID>{
        let oid = self.user_handler.create_new_user(username, password, true).await?;
        self.session_handler.start_session(oid).await?;
        let id = ID::new(oid.to_hex()).unwrap();
        Ok(id)
    }

    // using id to sign in is not optimal, TODO: make email sign in
    ///returns faslse if credentials are wrong or user doesn't exist'
    pub async fn signin_by_id(&self, username: String, password: String, id: &ID) -> Result<bool>{
        let oid = ObjectId::parse_str(id.id.clone())?;
        if !self.user_handler.check_user_credentials(oid, username, password).await?{
            return Ok(false);
        }
        self.user_handler.set_user_status(oid, true).await?;
        self.session_handler.start_session(oid).await?;
        Ok(true)
    } 

    pub async fn signout(&self, id: ID) -> Result<()> {
        let oid = ObjectId::parse_str(id.id)?;
        self.session_handler.delete_session(oid).await?;
        self.user_handler.set_user_status(oid, false).await.expect("should not be None as Session exists for user id");
        Ok(())
    }

    pub async fn check_authentication(&self, cookie: ID) -> Result<bool>{
        let oid = ObjectId::parse_str(cookie.id)?;
        self.session_handler.check_session_active(oid).await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::test;
    #[test]
    async fn test_signup() {
        
    }
}
