use mongodb::{
    bson::{doc, oid::ObjectId},
    Client, Database,
};
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Default, Serialize, Deserialize)]
struct ServerConfig {
    name: String,
    admins: Vec<ObjectId>,
    users: Vec<ObjectId>,
}

/// a clean abstraction for the core server functionalities
pub struct ServerHandler {
    server: Database,
}

impl ServerConfig {
    fn new(name: String, creator: ObjectId) -> Self {
        let mut admins = Vec::new();
        let mut users = Vec::new();
        admins.push(creator);
        users.push(creator);

        Self {
            name,
            admins,
            users,
        }
    }
}

impl ServerHandler {
    pub fn new(server: Database) -> Self {
        Self { server }
    }

    pub async fn new_server(client: &Client, name: String, creator: ObjectId) -> Result<Self> {
        let id = ObjectId::new().to_hex();
        let db = client.database(&id);
        let coll = db.collection(".config");
        let conf = ServerConfig::new(name, creator);
        coll.insert_one(conf, None).await?;

        Ok(Self { server: db })
    }

    pub async fn delete_server(self) -> Result<()> {
        Ok(self.server.drop(None).await?)
    }
}
