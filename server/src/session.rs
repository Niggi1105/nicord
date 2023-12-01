use anyhow::Result;
use log::error;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Client, Collection, Database,
};
use serde::{Deserialize, Serialize};
use std::time::{self, SystemTime};

#[derive(Debug, Serialize, Deserialize)]
struct Session {
    _id: ObjectId,
    start: time::SystemTime,
}

#[derive(Clone)]
pub struct SessionHandler {
    database: Database,
    collection: Collection<Session>,
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

    /// Session expires after 10 min
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
    pub fn new(database: Database, collection: Collection<Session>) -> Self {
        Self {
            database,
            collection,
        }
    }
    pub fn from_names(client: &Client, database: &str, collection: &str) -> Self {
        let db = client.database(database);
        let coll = db.collection(collection);

        Self {
            database: db,
            collection: coll,
        }
    }

    pub async fn start_session(&self, user_id: ObjectId) -> Result<()> {
        if self.check_session_active(user_id).await? {
            return Ok(());
        }

        let session = Session::new(user_id);

        self.collection
            .insert_one(session, None)
            .await?
            .inserted_id
            .as_object_id()
            .expect("is object id");

        Ok(())
    }

    pub async fn delete_session(&self, id: ObjectId) -> Result<()> {
        self.collection.delete_one(doc! {"_id": id}, None).await?;
        Ok(())
    }

    ///check whether session is active, expired sessions are being deleted
    pub async fn check_session_active(&self, id: ObjectId) -> Result<bool> {
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
mod test {
    use std::time::Duration;

    use crate::mongodb::connect_mongo;

    use super::*;
    use tokio::test;
    use tokio::time::sleep;

    #[test]
    async fn test_start_new_session() {
        let client = connect_mongo(None).await.unwrap();
        let db = client.database("TEST");
        let coll: Collection<Session> = db.collection("sessions");
        let handler = SessionHandler::new(db.clone(), coll.clone());
        let oid = ObjectId::parse_str("123123123123123123123123").unwrap();

        handler.start_session(oid).await.unwrap();
        coll.find_one(doc! {"_id": oid}, None)
            .await
            .unwrap()
            .unwrap();

        db.drop(None).await.unwrap();
    }

    #[test]
    async fn test_start_session_with_active_session() {
        let client = connect_mongo(None).await.unwrap();
        let db = client.database("TEST");
        let coll: Collection<Session> = db.collection("sessions");
        let handler = SessionHandler::new(db.clone(), coll.clone());
        let oid = ObjectId::parse_str("123123123123123123123123").unwrap();

        coll.insert_one(Session::new(oid), None).await.unwrap();
        handler.start_session(oid).await.unwrap();
        coll.find_one(doc! {"_id": oid}, None)
            .await
            .unwrap()
            .unwrap();

        db.drop(None).await.unwrap();
    }

    #[test]
    async fn test_delete_exitsting_session() {
        let client = connect_mongo(None).await.unwrap();
        let db = client.database("TEST");
        let coll: Collection<Session> = db.collection("sessions");
        let handler = SessionHandler::new(db.clone(), coll.clone());
        let oid = ObjectId::parse_str("123123123123123123123123").unwrap();

        coll.insert_one(Session::new(oid), None).await.unwrap();
        assert!(coll
            .find_one(doc! {"_id": oid}, None)
            .await
            .unwrap()
            .is_some());
        handler.delete_session(oid).await.unwrap();
        assert!(coll
            .find_one(doc! {"_id": oid}, None)
            .await
            .unwrap()
            .is_none());

        db.drop(None).await.unwrap();
    }

    #[test]
    async fn test_delete_not_exitsting_session() {
        let client = connect_mongo(None).await.unwrap();
        let db = client.database("TEST");
        let coll: Collection<Session> = db.collection("sessions");
        let handler = SessionHandler::new(db.clone(), coll.clone());
        let oid = ObjectId::parse_str("123123123123123123123123").unwrap();

        handler.delete_session(oid).await.unwrap();
        assert!(coll
            .find_one(doc! {"_id": oid}, None)
            .await
            .unwrap()
            .is_none());

        db.drop(None).await.unwrap();
    }

    #[test]
    async fn test_session_active_with_active_session(){
        let client = connect_mongo(None).await.unwrap();
        let db = client.database("TEST");
        let coll: Collection<Session> = db.collection("sessions");
        let handler = SessionHandler::new(db.clone(), coll.clone());
        let oid = ObjectId::parse_str("123123123123123123123123").unwrap();

        coll.insert_one(Session::new(oid), None).await.unwrap();
        assert!(handler.check_session_active(oid).await.unwrap());
        db.drop(None).await.unwrap();
    }

    #[test]
    async fn test_session_active_with_inactive_session(){
        let client = connect_mongo(None).await.unwrap();
        let db = client.database("TEST");
        let coll: Collection<Session> = db.collection("sessions");
        let handler = SessionHandler::new(db.clone(), coll.clone());
        let oid = ObjectId::parse_str("123123123123123123123123").unwrap();
        let session = Session{start: SystemTime::now().checked_sub(Duration::new(601, 0)).unwrap(), _id: oid };
        
        coll.insert_one(session, None).await.unwrap();
        assert!(!handler.check_session_active(oid).await.unwrap());
        db.drop(None).await.unwrap();
    }
}
