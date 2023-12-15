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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerHandler; 

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
        Self {}
    }

    ///creates a new Database with the Server ID as name and initializes the server with a config
    ///file, adding the creator as admin and user
    pub async fn new_server(user_id: ID, client: &Client, name: String) -> Result<Response> {
        let id = ObjectId::new().to_hex();
        let db = client.database(&id);
        let coll = db.collection(".config");
        let oid = ObjectId::parse_str(user_id.id).expect("invalid ID provided");
        let conf = ServerConfig::new(name, oid);

        coll.insert_one(conf, None).await?;
        Ok(Response::ServerCreated(
            ID::new(id).expect("is an object id"),
        ))
    }

    /// deletes the server db if the user has the required priviledges
    pub async fn delete_server(user_id: ID, client: &Client, server_id: ID) -> Result<Response> {
        let server_hex_id = ObjectId::parse_str(
            server_id.id,
        )
        .expect("is an oid")
        .to_hex();

        let db = client.database(&server_hex_id);
        let coll: Collection<ServerConfig> = db.collection(".config");

        let conf_opt = coll
            .find_one(None, None)
            .await?;
        if conf_opt.is_none(){
            return Ok(Response::Error(ServerError::BadRequest));
        }
        let conf = conf_opt.expect("checked for none");

        let user_oid = &ObjectId::parse_str(user_id.id).expect("is oid");

        if conf.admins.contains(user_oid) {
            db.drop(None).await?;
            return Ok(Response::Success);
        }
        Ok(Response::Error(ServerError::PermissionDenied))
    }


    ///returns the servername (String) that is written in the servers config
    pub async fn get_server_name_by_id(mongo_client: &Client, server_id: &ID) -> Result<String> {
        let db = mongo_client.database(&server_id.id);
        let conf_coll: Collection<ServerConfig> = db.collection(".config");
        let conf_opt = conf_coll.find_one(None, None).await?;
        if conf_opt.is_none(){
            return Err(anyhow!("server not initialized"));
        }

        let name = conf_opt.expect("checked above").name;
    
        Ok(name)
    }

    ///creates a new channel in the given server if the user has the required priviledges
    pub async fn new_channel(user_id: ID, client: &Client, name: String, server_id: ID) -> Result<Response> {
        let db = client.database(&server_id.to_string());
        let conf_coll: Collection<ServerConfig> = db.collection(".config");
        let conf_opt = conf_coll.find_one(None, None).await?;

        if conf_opt.is_none() {
            return Ok(Response::Error(ServerError::BadRequest));
        }

        if !conf_opt
            .expect("checked above")
            .admins
            .contains(&ObjectId::parse_str(user_id.id)?)
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
