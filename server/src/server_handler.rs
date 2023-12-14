use std::task::Wake;

use anyhow::{anyhow, Result};
use common::{error::ServerError, id::ID, messages::Response};
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
#[derive(Serialize, Deserialize, Debug)]
pub struct ServerHandler {
    user_id: ID,
    current_server_id: Option<ID>,
    current_channel: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
enum Message {
    ServerInfo(String),          //content
    UserMessage(String, String), //content, author
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

impl Message {
    pub fn new_server_message(content: String) -> Self {
        Self::ServerInfo(content)
    }
}

impl ServerHandler {
    pub fn new(user_id: ID) -> Self {
        Self {
            user_id,
            current_server_id: None,
            current_channel: None,
        }
    }

    ///creates a new Database with the Server ID as name and initializes the server with a config
    ///file, adding the creator as admin and user
    pub async fn new_server(&self, client: &Client, name: String) -> Result<Response> {
        let id = ObjectId::new().to_hex();
        let db = client.database(&id);
        let coll = db.collection(".config");
        let oid = ObjectId::parse_str(self.user_id.to_owned().id).expect("invalid ID provided");
        let conf = ServerConfig::new(name, oid);

        coll.insert_one(conf, None).await?;
        Ok(Response::ServerCreated(ID::new(id).expect("is an object id")))
    }

    pub async fn delete_server(&self, client: &Client) -> Result<Response> {
        if self.current_server_id.is_none() {
            return Ok(Response::Error(ServerError::BadRequest));
        }

        let hex_id = ObjectId::parse_str(
            self.current_server_id
                .to_owned()
                .expect("checked for none")
                .id,
        )
        .expect("is an oid")
        .to_hex();
        let db = client.database(&hex_id);
        let coll: Collection<ServerConfig> = db.collection(".config");
        let conf = coll
            .find_one(doc! {}, None)
            .await?
            .expect("Server has no config");
        let user_oid = &ObjectId::parse_str(self.user_id.to_owned().id).expect("is oid");

        if conf.admins.contains(user_oid) {
            db.drop(None).await?;
            return Ok(Response::Success);
        }
        Ok(Response::Error(ServerError::PermissionDenied))
    }

    pub async fn new_channel(
        &self,
        client: &Client,
        name: String,
    ) -> Result<Response> {
        
        if self.current_server_id.is_none(){
            return Ok(Response::Error(ServerError::BadRequest));
        }
        let server_id = self.current_server_id.to_owned().expect("checked above");
        let db = client.database(&server_id.to_string());
        let conf_coll: Collection<ServerConfig> = db.collection(".config");
        let conf_opt = conf_coll.find_one(None, None).await?;

        if conf_opt.is_none() {
            return Err(anyhow! {"server not inittialized"});
        }

        if !conf_opt
            .expect("checked above")
            .admins
            .contains(&ObjectId::parse_str(self.user_id.id.to_string())?)
        {
            return Ok(Response::Error(ServerError::PermissionDenied));
        }

        let channles = db.list_collection_names(None).await?;
        if channles.contains(&name) {
            return Ok(Response::Error(ServerError::BadRequest));
        }

        let channel: Collection<Message> = db.collection(&name);
        let init_message = Message::new_server_message("channel created...".to_string());
        channel.insert_one(init_message, None).await?;

        Ok(Response::Success)
    }
}
