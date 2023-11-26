use anyhow::Result;
use common::{id::ID};
use mongodb::bson::oid::ObjectId;

use crate::{user::UserHandler, session::SessionHandler};

#[derive(Clone)]
pub struct AuthHandler{
    pub session_handler: SessionHandler,
    pub user_handler: UserHandler,
}

impl AuthHandler{
    pub fn new(session_handler: SessionHandler, user_handler: UserHandler) -> Self {
        Self { session_handler, user_handler }
    }
    /// creates new user and starts a session with ID
    pub async fn signup(&mut self, username: String, password: String) -> Result<ID>{
        let oid = self.user_handler.create_new_user(username, password, true).await?;
        self.session_handler.start_session(oid).await?;
        let id = ID::new(oid.to_hex()).unwrap();
        Ok(id)
    }

    // using id to sign in is not optimal, TODO: make email sign in
    /// returns true if user exists and credentials are correct
    pub async fn signin_by_id(&mut self, username: String, password: String, id: &ID) -> Result<bool>{
        let oid = ObjectId::parse_str(id.id.clone())?;
        if !self.user_handler.check_user_credentials(oid, username, password).await?{
            return Ok(false);
        }
        self.user_handler.set_user_status(oid, true).await?;
        self.session_handler.start_session(oid).await
    } 

    /// returns false if no session exitst with ID
    /// panics if Session DB and User DB are out of sync
    pub async fn signout(&mut self, id: ID) -> Result<bool> {
        let oid = ObjectId::parse_str(id.id)?;
        if !self.session_handler.delete_session(oid).await? {
            return Ok(false);
        }
        self.user_handler.set_user_status(oid, false).await.expect("should not be None as Session exists for user id");
        Ok(true)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::test;
    #[test]
    async fn test_auth_req() {}
}
