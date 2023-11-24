use anyhow::Result;
use log::{error, info};
use mongodb::bson::Document;
use mongodb::error::Error;
use mongodb::options::{self, ClientOptions, FindOptions, ListDatabasesOptions, CollectionOptions, CreateCollectionOptions};
use mongodb::{Client, Collection, Database};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::{mpsc, oneshot};

use common::configs::ServerConfig;

pub enum CommandType {
    NewDatabase(String),
    ListDatbases(Option<Document>, Option<ListDatabasesOptions>),
    NewCollection(String, Database, CreateCollectionOptions),
    GetClient,
}

pub enum MongoResponse {
    Error(anyhow::Error),
    ListDatabaseResponse(Vec<String>),
    NewDatabaseResponse(Database),
    CollectionResponse(Collection<Document>),
    GetClientResponse(Client),
}

pub struct Command {
    tp: CommandType,
    resp: oneshot::Sender<MongoResponse>,
}

pub struct MongoConnection {
    sender: mpsc::Sender<Command>,
}

impl MongoConnection {

    /// connect to 
    pub async fn start(client_options: Option<ClientOptions>) -> Result<Self> {
        let cl = Self::connect_mongo(client_options).await?;
        let (sx, rx) = mpsc::channel(50);
        let s = Self { sender: sx };
        Self::listen(cl, rx);
        Ok(s)
    }

    /// trys to connect to a mongo database with the provided options, if no options
    /// are provided default options are used and the functions looks for a localhost
    /// instance of mongodb
    ///
    /// the client internally uses connection pooling in order to increase performance
    async fn connect_mongo(opts: Option<ClientOptions>) -> Result<Client> {
        let client_options = match opts {
            Some(opt) => opt,
            None => ClientOptions::parse("mongodb://localhost:27017").await?,
        };
        let client = Client::with_options(client_options)?;

        let _ = tokio::time::timeout(
            tokio::time::Duration::new(5, 0),
            client.list_database_names(None, None),
        )
        .await??;
        Ok(client)
    }

    pub fn new_channel(&self) -> mpsc::Sender<Command> {
        self.sender.clone()
    }

    /// retrieves the first match to the given filter in the Collection
    pub async fn retrieve<'a, T>(
        collection: &Collection<T>,
        filter: Document,
        options: FindOptions,
    ) -> Result<Vec<T>>
    where
        T: DeserializeOwned + 'a,
    {
        let mut cursor = collection.find(filter, options).await?;
        let mut result = Vec::new();
        while cursor.advance().await? {
            result.push(cursor.deserialize_current()?);
        }
        Ok(result)
    }

    /// insert the data into the collection
    async fn insert<'a, T>(data: &T, collection: &Collection<T>) -> Result<()>
    where
        T: Serialize + DeserializeOwned,
    {
        collection.insert_one(data, None).await?;
        Ok(())
    }

    fn listen(client: Client, mut reciever: mpsc::Receiver<Command>) {
        let spawn = tokio::spawn(async move {
            let cmd = reciever.recv().await.unwrap();
            let r = match cmd.tp {
                CommandType::ListDatbases(filter, opt) => {
                    let l = client.list_database_names(filter, opt).await;
                    match l {
                        Err(e) => {
                            MongoResponse::Error(e.into())
                        }
                        Ok(val) => {
                            MongoResponse::ListDatabaseResponse(val)
                        }
                    }
                }
                CommandType::NewDatabase(name) => {
                    let db = client.database(&name);
                    MongoResponse::NewDatabaseResponse(db)
                }
                CommandType::NewCollection(name, db, opt) => {
                    let coll = db.create_collection(name, opt).await;
                    match coll {
                        Err(e) => {
                            MongoResponse::Error(e.into())
                        }
                        Ok(_) => {
                            let col = db.collection(&name);
                            
                            MongoResponse::CollectionResponse(col)
                        }
                    }
                }
                CommandType::GetClient => {
                    MongoResponse::GetClientResponse(client)
                }
                _other => {
                    unimplemented!()
                }
            };
            cmd.resp.send(r);
        });
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::test;

    #[test]
    async fn make_connection() {
        MongoConnection::start(None).await.unwrap();
    }

    #[test]
    async fn new_server() {}
}
