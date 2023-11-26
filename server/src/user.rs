use anyhow::Result;
use common::user::User;
use mongodb::bson::doc;
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
    pub fn new(client: &Client, database: &str, collection: &str) -> Self {
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

    async fn get_user_sensitive(&self, user_id: ObjectId) -> Result<Option<SensitiveUser>>{
        Ok(self.collection.find_one(doc! {"_id": user_id}, None).await?)
    }

    pub async fn get_user(&self, user_id: ObjectId) -> Result<Option<User>> {
        let option = self.collection.find_one(doc! {"_id": user_id}, None).await?;
        return match option {
            Some(sensitive) => {
                Ok(Some(sensitive.to_user()))
            }
            None => Ok(None)
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

    ///returns true if user was modifided, else false
    pub async fn set_user_status(&self, user_id: ObjectId, status: bool) -> Result<bool> {
        let count = self.collection
            .update_one(doc! {"_id": user_id}, doc! {"$set": {"is_online": status}}, None)
            .await?.modified_count;

        Ok(count == 1)
    }

    ///returns true if user exists, else false
    pub async fn add_user_server(&self, user_id: ObjectId, name: String) -> Result<bool>{
        let user_option = self.get_user_sensitive(user_id).await?;
        match user_option {
            None => Ok(false),
            Some(mut user) => {
                user.servers.push(name);
                self.collection.replace_one(doc! {"_id": user_id}, user, None);
                Ok(true)
            }
        }
    }
    
    pub async fn check_user_credentials(&self, user_id: ObjectId, username: String, password: String) -> Result<bool> {
        let option_user = self.collection.find_one(doc! {"_id": user_id}, None).await?;
        match option_user {
            Some(user) => {
                Ok(user.check_credentials(&password, &username))
            }
            None => return Ok(false)
        }
    }

}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_pwd_check() {
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
}
