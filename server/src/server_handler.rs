use anyhow::Result;
use common::id::ID;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Client, Collection, Database,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
struct ServerConfig {
    name: String,
    admins: Vec<ObjectId>,
    users: Vec<ObjectId>,
}

/// a clean abstraction for the core server functionalities
pub struct ServerHandler;

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
    pub async fn new_server(client: &Client, name: String, creator: ID) -> Result<ID> {
        let id = ObjectId::new().to_hex();
        let db = client.database(&id);
        let coll = db.collection(".config");
        let oid = ObjectId::parse_str(creator.id).expect("invalid ID provided");
        let conf = ServerConfig::new(name, oid);

        coll.insert_one(conf, None).await?;
        Ok(ID::new(id).expect("is an object id"))
    }

    pub async fn delete_server(client: &Client, server_id: ID, user_id: ID) -> Result<bool> {
        let hex_id = ObjectId::parse_str(server_id.id)
            .expect("is an oid")
            .to_hex();
        let db = client.database(&hex_id);
        let coll: Collection<ServerConfig> = db.collection(".config");
        let conf = coll
            .find_one(doc! {}, None)
            .await?
            .expect("Server has no config");
        let user_oid = &ObjectId::parse_str(user_id.id).expect("is oid");

        for admin in conf.admins.iter() {
            if admin == user_oid {
                db.drop(None).await?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}
