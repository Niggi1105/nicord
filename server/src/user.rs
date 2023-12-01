use anyhow::Result;
use common::error::ServerError;
use common::user::User;
use log::debug;
use mongodb::bson::doc;
use mongodb::options::UpdateModifications;
use mongodb::{bson::oid::ObjectId, Client, Collection, Database};
use serde::{Deserialize, Serialize};

//TODO add email address and email address sign in option
#[derive(Debug, Serialize, Deserialize)]
struct SensitiveUser {
    _id: ObjectId,
    is_online: bool,
    username: String,
    password: String,
    servers: Vec<String>,
}

#[derive(Clone)]
pub struct UserHandler {
    database: Database,
    collection: Collection<SensitiveUser>,
}

pub enum FindError<T> {
    Ok(T),
    Err(ServerError),
}

impl SensitiveUser {
    pub fn new(
        _id: ObjectId,
        is_online: bool,
        username: String,
        password: String,
        servers: Vec<String>,
    ) -> Self {
        Self {
            _id,
            is_online,
            username,
            password,
            servers,
        }
    }

    fn check_credentials(&self, pwd: &str, username: &str) -> bool {
        self.password == pwd && self.username == username
    }

    fn to_user(&self) -> User {
        User::new(self.username.clone(), self.is_online, self.servers.clone())
    }
}

impl UserHandler {
    pub fn new(database: Database, collection: Collection<SensitiveUser>) -> Self {
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

    pub async fn create_new_user(
        &self,
        username: String,
        password: String,
        is_online: bool,
    ) -> Result<ObjectId> {
        let oid = ObjectId::new();
        let user = SensitiveUser::new(oid, is_online, username, password, Vec::new());

        self.collection.insert_one(user, None).await?;

        Ok(oid)
    }

    async fn get_user_sensitive(&self, user_id: ObjectId) -> Result<Option<SensitiveUser>> {
        Ok(self
            .collection
            .find_one(doc! {"_id": user_id}, None)
            .await?)
    }

    pub async fn get_user(&self, user_id: ObjectId) -> Result<Option<User>> {
        let option = self.get_user_sensitive(user_id).await?;
        match option {
            Some(sensitive) => Ok(Some(sensitive.to_user())),
            None => Ok(None),
        }
    }

    pub async fn find_user_by_name(&self, username: String) -> Result<Vec<User>> {
        let mut cursor = self
            .collection
            .find(doc! {"username": username}, None)
            .await?;
        let mut users = Vec::new();

        while cursor.advance().await? {
            let sensitive = cursor.deserialize_current()?;
            users.push(sensitive.to_user());
        }

        Ok(users)
    }

    pub async fn set_user_status(&self, user_id: ObjectId, status: bool) -> Result<()> {
        let modif = UpdateModifications::Document(doc! {"is_online": status});
        self.collection
            .update_one(
                doc! {"_id": user_id},
                doc! {"$set": doc! {"is_online": status}},
                None,
            )
            .await?;

        Ok(())
    }

    pub async fn add_user_server(&self, user_id: ObjectId, name: String) -> Result<()> {
        let mut servers = self
            .get_user_sensitive(user_id)
            .await?
            .expect("if session exists, user has to exist")
            .servers;
        servers.push(name);
        self.collection
            .update_one(
                doc! {"_id": user_id},
                doc! {"$set": doc! {"servers": servers}},
                None,
            )
            .await?;
        Ok(())
    }

    ///returns true if the user exitsts, and the credentials are correct
    pub async fn check_user_credentials(
        &self,
        user_id: ObjectId,
        username: &str,
        password: &str,
    ) -> Result<bool> {
        let opt_usr = self.get_user_sensitive(user_id).await?;
        match opt_usr {
            Some(user) => Ok(user.check_credentials(password, username)),
            None => Ok(false),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::mongodb::{self, connect_mongo};
    use tokio::test;

    use super::*;
    #[test]
    async fn test_pwd_check() {
        let u = SensitiveUser {
            _id: ObjectId::new(),
            is_online: false,
            username: "Bob".to_string(),
            password: "#Passwort123".to_string(),
            servers: Vec::new(),
        };

        assert!(u.check_credentials("#Passwort123", "Bob"));
        assert!(!u.check_credentials("falsches Passwort", "Bob"));
        assert!(!u.check_credentials("#Passwort123", "Paul"));
    }

    async fn setup_test_database(client: &Client) {
        let mut db = client.database("TEST");
        db.drop(None).await.unwrap();
        db = client.database("TEST");
        let coll: Collection<SensitiveUser> = db.collection("user");
        let mut u = SensitiveUser::new(
            ObjectId::parse_str("123123123123123123123123").unwrap(),
            false,
            "Max".to_string(),
            "Passwort".to_string(),
            Vec::new(),
        );
        coll.insert_one(u, None).await.unwrap();
        u = SensitiveUser::new(
            ObjectId::parse_str("123123123123123123123124").unwrap(),
            false,
            "Moritz".to_string(),
            "Passwort".to_string(),
            Vec::new(),
        );
        coll.insert_one(u, None).await.unwrap();
        u = SensitiveUser::new(
            ObjectId::parse_str("123123123123123123123125").unwrap(),
            false,
            "Max".to_string(),
            "Passwort123".to_string(),
            Vec::new(),
        );
        coll.insert_one(u, None).await.unwrap();
        u = SensitiveUser::new(
            ObjectId::parse_str("123123123123123123123126").unwrap(),
            true,
            "Moritz".to_string(),
            "Passwort".to_string(),
            Vec::new(),
        );
        coll.insert_one(u, None).await.unwrap();
        u = SensitiveUser::new(
            ObjectId::parse_str("123123123123123123123127").unwrap(),
            true,
            "Malte".to_string(),
            "Passwort".to_string(),
            vec!["MyServer".to_string()],
        );
        coll.insert_one(u, None).await.unwrap();
    }

    #[test]
    async fn test_create_new_user() {
        let client = mongodb::connect_mongo(None).await.unwrap();
        let db = client.database("TEST");
        let coll = db.collection("user");
        let handler = UserHandler::new(db.clone(), coll.clone());
        let id = handler
            .create_new_user("User123".to_string(), "Password123".to_string(), true)
            .await
            .unwrap();
        let found = coll
            .find_one(doc! {"_id": id}, None)
            .await
            .unwrap()
            .unwrap();
        assert!(found.servers == Vec::<String>::new());
        assert!(found.username == *"User123".to_string());
        assert!(found.password == *"Password123".to_string());
        assert!(found.is_online);
        db.drop(None).await.unwrap();
    }

    #[test]
    async fn test_get_user_sensitive() {
        let client = connect_mongo(None).await.unwrap();
        setup_test_database(&client).await;
        let db = client.database("TEST");
        let coll = db.collection("user");
        let handler = UserHandler::new(db.clone(), coll.clone());
        let u = handler
            .get_user_sensitive(ObjectId::parse_str("123123123123123123123127").unwrap())
            .await
            .unwrap()
            .unwrap();
        assert!(u._id == ObjectId::parse_str("123123123123123123123127").unwrap());
        assert!(u.servers == vec!["MyServer".to_string()]);
        assert!(u.username == *"Malte".to_string());
        assert!(u.password == *"Passwort".to_string());
        assert!(u.is_online);
        db.drop(None).await.unwrap();
    }

    #[test]
    async fn test_get_user() {
        let client = connect_mongo(None).await.unwrap();
        setup_test_database(&client).await;
        let db = client.database("TEST");
        let coll = db.collection("user");
        let handler = UserHandler::new(db.clone(), coll.clone());
        let u = handler
            .get_user(ObjectId::parse_str("123123123123123123123127").unwrap())
            .await
            .unwrap()
            .unwrap();
        assert!(u.servers == vec!["MyServer".to_string()]);
        assert!(u.username == *"Malte".to_string());
        assert!(u.is_online);
        db.drop(None).await.unwrap();
    }

    #[test]
    async fn test_find_user_by_name() {
        let client = connect_mongo(None).await.unwrap();
        setup_test_database(&client).await;
        let db = client.database("TEST");
        let coll = db.collection("user");
        let handler = UserHandler::new(db.clone(), coll.clone());
        let binding = handler
            .find_user_by_name("Moritz".to_string())
            .await
            .unwrap();
        let mut u = binding.iter();
        if u.next().unwrap().is_online {
            assert!(!u.next().unwrap().is_online)
        } else {
            assert!(u.next().unwrap().is_online)
        }
        db.drop(None).await.unwrap();
    }

    #[test]
    async fn test_set_user_status() {
        let client = connect_mongo(None).await.unwrap();
        setup_test_database(&client).await;
        let db = client.database("TEST");
        let coll = db.collection("user");
        let handler = UserHandler::new(db.clone(), coll.clone());
        handler
            .set_user_status(
                ObjectId::parse_str("123123123123123123123127").unwrap(),
                false,
            )
            .await
            .unwrap();
        assert!(
            !coll
                .find_one(
                    doc! {"_id": ObjectId::parse_str("123123123123123123123127").unwrap()},
                    None
                )
                .await
                .unwrap()
                .unwrap()
                .is_online
        );
        handler
            .set_user_status(
                ObjectId::parse_str("123123123123123123123127").unwrap(),
                true,
            )
            .await
            .unwrap();
        assert!(
            coll.find_one(
                doc! {"_id": ObjectId::parse_str("123123123123123123123127").unwrap()},
                None
            )
            .await
            .unwrap()
            .unwrap()
            .is_online
        );
        db.drop(None).await.unwrap();
    }

    #[test]
    async fn test_add_user_server() {
        let client = connect_mongo(None).await.unwrap();
        setup_test_database(&client).await;
        let db = client.database("TEST");
        let coll = db.collection("user");
        let handler = UserHandler::new(db.clone(), coll.clone());
        let oid = ObjectId::parse_str("123123123123123123123127").unwrap();
        handler
            .add_user_server(oid, "MyServer2".to_string())
            .await
            .unwrap();
        let u = coll
            .find_one(doc! {"_id": oid}, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            u.servers,
            vec!["MyServer".to_string(), "MyServer2".to_string()]
        );
        db.drop(None).await.unwrap();
    }

    #[test]
    async fn test_check_user_credentials() {
        let client = connect_mongo(None).await.unwrap();
        setup_test_database(&client).await;
        let db = client.database("TEST");
        let coll = db.collection("user");
        let handler = UserHandler::new(db.clone(), coll.clone());

        //correct login
        let mut oid = ObjectId::parse_str("123123123123123123123127").unwrap();
        assert!(handler
            .check_user_credentials(oid, "Malte", "Passwort")
            .await
            .unwrap());

        //correct login
        oid = ObjectId::parse_str("123123123123123123123125").unwrap();
        assert!(handler
            .check_user_credentials(oid, "Max", "Passwort123")
            .await
            .unwrap());

        //password false
        oid = ObjectId::parse_str("123123123123123123123125").unwrap();
        assert!(!handler
            .check_user_credentials(oid, "Max", "Passwort")
            .await
            .unwrap());

        //username false
        oid = ObjectId::parse_str("123123123123123123123125").unwrap();
        assert!(!handler
            .check_user_credentials(oid, "Ma", "Passwort123")
            .await
            .unwrap());

        //oid false
        oid = ObjectId::parse_str("123123123123123123123120").unwrap();
        assert!(!handler
            .check_user_credentials(oid, "Max", "Passwort123")
            .await
            .unwrap());
        db.drop(None).await.unwrap();
    }
}
