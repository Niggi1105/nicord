use anyhow::{anyhow, Result};
use common::{error::ServerError, id::ID, messages::Response};
use mongodb::{
    bson::{doc, oid::ObjectId},
    Client, Collection, Database,
};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Default, Serialize, Deserialize)]
struct ServerConfig {
    name: String,
    admins: Vec<ID>,
    users: Vec<ID>,
}

///implements functions for dealing with the the core nicord server functionalities
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerHandler;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Message {
    content: String,
    author: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Block {
    messages: Vec<Message>,
    time_stamp: SystemTime,
    filled: bool,
}


impl Block {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            time_stamp: SystemTime::now(),
            filled: false,
        }
    }

    ///add message to a block, if reaches 50 sets filled flag, if len >= 50 and add message is
    ///called false is returned to signalize an invalid operation
    fn add_message(&mut self, message: Message) -> bool {
        if self.messages.len() >= 50 {
            return false;
        }
        self.messages.push(message);
        if self.messages.len() == 50 {
            self.filled = true
        };
        true
    }
}

impl ServerConfig {
    fn new(name: String, creator: ID) -> Self {
        let mut admins = Vec::new();
        let mut users = Vec::new();
        admins.push(creator.clone());
        users.push(creator);

        Self {
            name,
            admins,
            users,
        }
    }
}

impl Message {
    pub fn new(content: String, author: String) -> Self {
        Self { content, author }
    }
}

impl ServerHandler {
    ///checks whether the user has the required priviledges on the server
    async fn check_priviledge(server: &Database, user_id: &ID) -> Result<Response> {
        let conf_coll: Collection<ServerConfig> = server.collection("config");
        let conf_opt = conf_coll.find_one(None, None).await?;

        if conf_opt.is_none() {
            return Ok(Response::Error(ServerError::BadRequest));
        }

        if !conf_opt.expect("checked above").admins.contains(user_id) {
            return Ok(Response::Error(ServerError::PermissionDenied));
        }
        Ok(Response::Success)
    }

    ///creates a new server and server id, the server is stored with the id as the dbs name and the
    ///name in the config, the user is automatically assigned admin and user status
    pub async fn new_server(user_id: ID, client: &Client, name: String) -> Result<Response> {
        let id = ObjectId::new().to_hex();
        let db = client.database(&id);

        let coll = db.collection("config");
        let conf = ServerConfig::new(name, user_id);

        coll.insert_one(conf, None).await?;
        Ok(Response::ServerCreated(
            ID::new(id).expect("is an object id"),
        ))
    }

    /// deletes the server db if the user has the required priviledges
    pub async fn delete_server(user_id: &ID, client: &Client, server_id: &ID) -> Result<Response> {
        let db = client.database(&server_id.id);

        match Self::check_priviledge(&db, user_id).await? {
            Response::Success => {
                db.drop(None).await?;
                Ok(Response::Success)
            }
            Response::Error(e) => Ok(Response::Error(e)),
            other => panic!("unexpected enum variant: {:?}", other),
        }
    }

    ///returns the servername (String) that is written in the servers config
    ///if the server is not inititalized an error is returned
    ///should only be used when a single or very few names are required, else it might be more
    ///appropriate to do a lookup in the servername table TODO
    pub async fn get_server_name_by_id(mongo_client: &Client, server_id: &ID) -> Result<String> {
        let db = mongo_client.database(&server_id.id);
        let conf_coll: Collection<ServerConfig> = db.collection("config");
        let conf_opt = conf_coll.find_one(None, None).await?;
        if conf_opt.is_none() {
            return Err(anyhow!("server not initialized"));
        }
        let name = conf_opt.expect("checked above").name;
        Ok(name)
    }

    ///creates a new channel in the given server if the user has the required priviledges
    ///returns bad request if the server has no valid config or the channel name is already taken
    pub async fn new_channel(
        user_id: &ID,
        client: &Client,
        name: &String,
        server_id: &ID,
    ) -> Result<Response> {
        let db = client.database(&server_id.to_string());
        match Self::check_priviledge(&db, user_id).await? {
            Response::Success => {}
            Response::Error(e) => return Ok(Response::Error(e)),
            _other => return Ok(Response::Error(ServerError::InternalServerError)),
        }

        let channles = db.list_collection_names(None).await?;
        if channles.contains(name) {
            //duplicate channel name
            return Ok(Response::Error(ServerError::BadRequest));
        }

        //create the channel
        let channel: Collection<Message> = db.collection(name);
        let init_message = Message::new("channel created...".to_string(), "SERVER".to_string());
        //insert the init message into the channel_response' collection in order to create the collection
        channel.insert_one(init_message, None).await?;

        Ok(Response::Success)
    }

    ///delete a channel by its name, returns bad request if channel doesn't exist or the server
    ///does not exist
    pub async fn delete_channel(
        user_id: &ID,
        client: &Client,
        name: &String,
        server_id: &ID,
    ) -> Result<Response> {
        let db = client.database(&server_id.to_string());
        let channel: Collection<Message> = db.collection(name);
        match Self::check_priviledge(&db, user_id).await? {
            Response::Success => {}
            Response::Error(e) => return Ok(Response::Error(e)),
            other => panic!("unexpected enum variant: {:?}", other),
        }
        let channles = db.list_collection_names(None).await?;
        if !channles.contains(name) {
            //channel does not exist
            return Ok(Response::Error(ServerError::BadRequest));
        }
        channel.drop(None).await?;
        Ok(Response::Success)
    }

    ///returns a response containing a vector with all the channel names in the database if the
    ///user is listed as user in the config document
    pub async fn get_channels(client: &Client, server_id: &ID, user_id: &ID) -> Result<Response> {
        let server = client.database(&server_id.id);

        let conf_coll: Collection<ServerConfig> = server.collection("config");
        let conf_opt = conf_coll.find_one(None, None).await?;

        if conf_opt.is_none() {
            return Ok(Response::Error(ServerError::BadRequest));
        }

        if !conf_opt.expect("checked above").users.contains(user_id) {
            return Ok(Response::Error(ServerError::PermissionDenied));
        }

        let collections = server.list_collection_names(None).await?;
        let channel_response = collections
            .iter()
            .filter(|channel| channel.as_str() != "config")
            .cloned()
            .collect();
        Ok(Response::ChannelList(channel_response))
    }

    pub async fn send_message(
        client: &Client,
        server_id: &ID,
        channel_name: &String,
        user_id: &ID,
        content: String,
        author: String,
    ) -> Result<Response> {
        let server = client.database(&server_id.id);
        let conf_coll: Collection<ServerConfig> = server.collection("config");
        let conf_opt = conf_coll.find_one(None, None).await?;

        if conf_opt.is_none() {
            return Ok(Response::Error(ServerError::BadRequest));
        }
        if !conf_opt.expect("checked above").users.contains(user_id) {
            return Ok(Response::Error(ServerError::PermissionDenied));
        }
        if !server
            .list_collection_names(None)
            .await?
            .contains(channel_name)
        {
            return Ok(Response::Error(ServerError::BadRequest));
        }

        let channel: Collection<Block> = server.collection(channel_name);
        let message = Message::new(content, author);

        if let Some(mut block) = channel.find_one(doc! {"filled": false}, None).await? {
            if !block.add_message(message) {
                //full block not marked as full
                return Ok(Response::Error(ServerError::InternalServerError));
            }
            channel
                .find_one_and_replace(doc! {"filled": false}, block, None)
                .await?;
        } else {
            let mut block = Block::new();
            block.add_message(message);
            channel.insert_one(block, None).await?;
        };

        Ok(Response::Success)
    }
}

#[cfg(test)]
mod test {
    use std::sync::mpsc::channel;

    use crate::mongodb::connect_mongo;
    use tokio::test;

    use super::*;

    #[test]
    async fn test_create_server() {
        let user_id = ID {
            id: "123123123123123123123123".to_string(),
        };
        let client = connect_mongo(None).await.unwrap();

        let resp = ServerHandler::new_server(user_id.clone(), &client, "TEST_SERVER1".to_string())
            .await
            .unwrap();
        let id = match resp {
            Response::ServerCreated(id) => id,
            other => panic!("got other: {:?}", other),
        };

        let db = client.database(&id.id);
        let coll: Collection<ServerConfig> = db.collection("config");
        let config = coll.find_one(None, None).await.unwrap().unwrap();

        assert!(config.admins.contains(&user_id));
        assert!(config.users.contains(&user_id));
        db.drop(None).await.unwrap();
    }

    #[test]
    async fn test_delete_server() {
        let user_id = ID {
            id: "123123123123123123123123".to_string(),
        };
        let client = connect_mongo(None).await.unwrap();

        let db = client.database("120129184124124127777154");
        let coll: Collection<ServerConfig> = db.collection("config");
        let conf = ServerConfig::new("TEST SERVER2".to_string(), user_id.clone());

        coll.insert_one(conf, None).await.unwrap();

        let resp = ServerHandler::delete_server(
            &user_id,
            &client,
            &ID {
                id: "120129184124124127777154".to_string(),
            },
        )
        .await
        .unwrap();
        assert!(resp.succeeded());
        assert!(coll.find_one(None, None).await.unwrap().is_none());
        assert!(!client
            .list_database_names(None, None)
            .await
            .unwrap()
            .contains(&"120129184124124127777154".to_string()));
        db.drop(None).await.unwrap();
    }

    #[test]
    async fn test_create_channel() {
        let user_id = ID {
            id: "123123123123123123123123".to_string(),
        };
        let client = connect_mongo(None).await.unwrap();
        let server_id = ID {
            id: "120129184124124127777155".to_string(),
        };

        let db = client.database(&server_id.id);
        let conf_coll: Collection<ServerConfig> = db.collection("config");
        let conf = ServerConfig::new("TEST SERVER3".to_string(), user_id.clone());

        conf_coll.insert_one(conf, None).await.unwrap();

        let resp =
            ServerHandler::new_channel(&user_id, &client, &"TEST_CHANNEL".to_string(), &server_id)
                .await
                .unwrap();
        match resp {
            Response::Success => {}
            other => panic!("unexpected enum variant: {:?}", other),
        }

        let channel: Collection<Message> = db.collection("TEST_CHANNEL");
        let message = channel.find_one(None, None).await.unwrap().unwrap();
        assert_eq!(message.content, "channel created...");
        db.drop(None).await.unwrap();
        assert_eq!(message.author, "SERVER");
    }

    #[test]
    async fn test_delete_channel() {
        let user_id = ID {
            id: "123123123123123123123123".to_string(),
        };
        let client = connect_mongo(None).await.unwrap();
        let server_id = ID {
            id: "120129184124124127777156".to_string(),
        };
        let db = client.database(&server_id.id);
        let conf_coll: Collection<ServerConfig> = db.collection("config");
        let conf = ServerConfig::new("TEST SERVER4".to_string(), user_id.clone());
        conf_coll.insert_one(conf, None).await.unwrap();

        let channel = db.collection("TEST_CHANNEL");
        channel
            .insert_one(
                Message::new("starting...".to_string(), "SERVER".to_string()),
                None,
            )
            .await
            .unwrap();

        let mut collections = db.list_collection_names(None).await.unwrap();
        assert!(collections.contains(&"TEST_CHANNEL".to_string()));

        assert!(ServerHandler::delete_channel(
            &user_id,
            &client,
            &"TEST_CHANNEL".to_string(),
            &server_id
        )
        .await
        .unwrap()
        .succeeded());

        collections = db.list_collection_names(None).await.unwrap();
        assert!(!collections.contains(&"TEST_CHANNEL".to_string()));

        db.drop(None).await.unwrap();
    }

    #[test]
    async fn test_get_channels_one_channel() {
        let user_id = ID {
            id: "123123123123123123123123".to_string(),
        };
        let client = connect_mongo(None).await.unwrap();
        let server_id = ID {
            id: "120129184124124127777157".to_string(),
        };
        let db = client.database(&server_id.id);
        let conf_coll: Collection<ServerConfig> = db.collection("config");
        let conf = ServerConfig::new("TEST SERVER5".to_string(), user_id.clone());
        conf_coll.insert_one(conf, None).await.unwrap();

        let channel = db.collection("TEST_CHANNEL");
        let mut block = Block::new();
        block.add_message(Message::new(
            "starting...".to_string(),
            "SERVER".to_string(),
        ));
        channel.insert_one(block, None).await.unwrap();

        let channel_response = ServerHandler::get_channels(&client, &server_id, &user_id)
            .await
            .unwrap();
        db.drop(None).await.unwrap();
        match channel_response {
            Response::ChannelList(channels) => {
                assert_eq!(channels.len(), 1);
                assert_eq!(channels[0], "TEST_CHANNEL");
            }
            other => {
                panic!("unexpected enum variant: {:?}", other)
            }
        }
    }

    #[test]
    async fn test_get_channels_multiple_channels() {
        let user_id = ID {
            id: "123123123123123123123123".to_string(),
        };
        let client = connect_mongo(None).await.unwrap();
        let server_id = ID {
            id: "120129184124124127777158".to_string(),
        };
        let db = client.database(&server_id.id);
        let conf_coll: Collection<ServerConfig> = db.collection("config");
        let conf = ServerConfig::new("TEST SERVER6".to_string(), user_id.clone());
        conf_coll.insert_one(conf, None).await.unwrap();

        let mut channel = db.collection("TEST_CHANNEL1");
        let mut block = Block::new();
        block.add_message(Message::new(
            "starting...".to_string(),
            "SERVER".to_string(),
        ));
        channel.insert_one(&block, None).await.unwrap();

        channel = db.collection("TEST_CHANNEL2");
        channel.insert_one(block, None).await.unwrap();

        let channel_response = ServerHandler::get_channels(&client, &server_id, &user_id)
            .await
            .unwrap();
        db.drop(None).await.unwrap();
        match channel_response {
            Response::ChannelList(channels) => {
                assert_eq!(channels.len(), 2);
                assert!(channels.contains(&"TEST_CHANNEL1".to_string()));
                assert!(channels.contains(&"TEST_CHANNEL2".to_string()));
            }
            other => panic!("unexpected enum variant: {:?}", other),
        }
    }

    #[test]
    async fn test_send_message() {
        let user_id = ID {
            id: "123123123123123123123123".to_string(),
        };
        let client = connect_mongo(None).await.unwrap();
        let server_id = ID {
            id: "120129184124124127777159".to_string(),
        };
        let db = client.database(&server_id.id);
        db.drop(None).await.unwrap();
        let conf_coll: Collection<ServerConfig> = db.collection("config");
        let conf = ServerConfig::new("TEST SERVER7".to_string(), user_id.clone());
        conf_coll.insert_one(conf, None).await.unwrap();

        let channel = db.collection("TEST_CHANNEL1");
        let mut block = Block::new();
        block.add_message(Message::new(
            "starting...".to_string(),
            "SERVER".to_string(),
        ));
        channel.insert_one(&block, None).await.unwrap();

        let content = "I'm a message".to_string();
        let author = "Some Dude".to_string();
        assert!(ServerHandler::send_message(
            &client,
            &server_id,
            &"TEST_CHANNEL1".to_string(),
            &user_id,
            content.clone(),
            author.clone()
        )
        .await
        .unwrap()
        .succeeded());

        block.add_message(Message::new(content, author));
        let blk: Block = channel.find_one(doc! {"filled": false}, None).await.unwrap().unwrap();
        assert_eq!(blk, block);
        db.drop(None).await.unwrap();
    }
}
