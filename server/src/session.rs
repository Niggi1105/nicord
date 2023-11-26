use anyhow::Result;
use log::error;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Client, Collection, Database,
};
use serde::{Deserialize, Serialize};
use std::time;

#[derive(Debug, Serialize, Deserialize)]
struct Session {
    _id: ObjectId,
    start: time::SystemTime,
}

#[derive(Clone)]
pub struct SessionHandler {
    database: Database,
    collection: Collection<Session>,
    active_sessions: u64,
}

impl Session {
    fn new(user_id: ObjectId) -> Self {
        Self {
            // we use the ID of the user as the id of the Session, this makes queries for both users
            // and sessions easier as we can simply use the cookie provided in the request
            _id: user_id,
            start: time::SystemTime::now(),
        }
    }

    fn is_expired(&self) -> bool {
        match self.start.elapsed() {
            Ok(val) => val.as_secs() > 600,
            Err(e) => {
                error!(
                    "encountered an error trying to get the expiration of session: {:?}",
                    e
                );
                panic!("can't compute elapsed time'")
            }
        }
    }
}

impl SessionHandler {
    pub fn new(client: &Client, database: &str, collection: &str) -> Self {
        let db = client.database(database);
        let coll = db.collection(collection);

        Self {
            database: db,
            collection: coll,
            active_sessions: 0,
        }
    }

    pub async fn start_session(&mut self, user_id: ObjectId) -> Result<bool> {
        if self.check_session_active(user_id).await? {
            return Ok(false)
        }

        let session = Session::new(user_id);

        self.collection
            .insert_one(session, None)
            .await?
            .inserted_id
            .as_object_id()
            .unwrap();
        self.active_sessions += 1;

        Ok(true)
    }

    pub async fn delete_session(&mut self, id: ObjectId) -> Result<bool> {
        let count = self
            .collection
            .delete_one(doc! {"_id": id}, None)
            .await?
            .deleted_count;
        self.active_sessions -= count;

        Ok(count == 1)
    }

    ///check whether session is active, expired sessions are being deleted
    pub async fn check_session_active(&mut self, id: ObjectId) -> Result<bool> {
        let session = self.collection.find_one(doc! {"_id": id}, None).await?;

        if session.is_none() {
            return Ok(false);
        }

        if session.unwrap().is_expired() {
            self.delete_session(id).await?;
            return Ok(false);
        }

        Ok(true)
    }
}

#[cfg(test)]
mod test{
    use super::*;
    use tokio::test;


}
