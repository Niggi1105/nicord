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
    admins: Vec<ID>,
    users: Vec<ID>,
}

///implements functions for dealing with the the core nicord server functionalities
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerHandler;

#[derive(Serialize, Deserialize, Debug)]
enum MessageAuthor {
    Server,
    User(String),
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    _id: ObjectId,
    content: String,
    author: MessageAuthor,
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
    pub fn new_server_message(content: String) -> Self {
        let oid = ObjectId::new();
        Self {
            _id: oid,
            content,
            author: MessageAuthor::Server,
        }
    }
    pub fn new_user_message(username: String, content: String) -> Self {
        let oid = ObjectId::new();
        Self {
            _id: oid,
            content,
            author: MessageAuthor::User(username),
        }
    }
}

impl ServerHandler {
    ///checks whether the user has the required priviledges on the server
    async fn check_priviledge(
        server: &Database,
        client: &Client,
        user_id: &ID,
    ) -> Result<Response> {
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

        match Self::check_priviledge(&db, client, user_id).await? {
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
        match Self::check_priviledge(&db, client, user_id).await? {
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
        let init_message = Message::new_server_message("channel created...".to_string());
        //insert the init message into the channels' collection in order to create the collection
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
        match Self::check_priviledge(&db, client, user_id).await? {
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
}

#[cfg(test)]
mod test {
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
            user_id,
            &client,
            ID {
                id: "120129184124124127777154".to_string(),
            },
        )
        .await
        .unwrap();
        assert!(resp.succeeded());
        assert!(coll.find_one(None, None).await.unwrap().is_none());
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
        match message.author {
            MessageAuthor::Server => {}
            other => panic!("unexpected enum variant: {:?}", other),
        }
        db.drop(None).await.unwrap();
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
            .insert_one(Message::new_server_message("starting...".to_string()), None)
            .await
            .unwrap();

        let mut collections = db.list_collection_names(None).await.unwrap();
        assert!(collections.contains(&"TEST_CHANNEL".to_string()));

        assert!(ServerHandler::delete_channel(
            user_id,
            &client,
            &"TEST_CHANNEL".to_string(),
            server_id
        )
        .await
        .unwrap()
        .succeeded());

        collections = db.list_collection_names(None).await.unwrap();
        assert!(!collections.contains(&"TEST_CHANNEL".to_string()));

        db.drop(None).await.unwrap();
    }
}
