use std::env::Args;
use std::process::Output;

use log::{error, info};
use mongodb::bson::Document;
use mongodb::error::Error;
use mongodb::options::{self, ClientOptions, FindOptions};
use mongodb::{Client, Collection};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::{mpsc, oneshot};
use tokio::time::error::Elapsed;

use common::configs::ServerConfig;

type Result<T> = std::result::Result<T, DatabaseConncectionError>;

/// Enum encapsulating tokio Timeout error, in case the mongodb server doesn'tmo
/// respond and mongodb errors.
#[derive(Debug)]
pub enum DatabaseConncectionError {
    MongDbError(Error),
    ConnectionTimeOutError(Elapsed),
}

impl From<Error> for DatabaseConncectionError {
    fn from(err: Error) -> DatabaseConncectionError {
        DatabaseConncectionError::MongDbError(err)
    }
}

impl From<Elapsed> for DatabaseConncectionError {
    fn from(el: Elapsed) -> DatabaseConncectionError {
        DatabaseConncectionError::ConnectionTimeOutError(el)
    }
}

pub enum Command_type {
    NewServer(String),
    ListServers,
    ListDatbases,
}

pub struct Command {
    t: Command_type,
    resp: oneshot::Sender,
}

pub struct MongoConnection {
    sender: mpsc::Sender<Command>,
}

impl MongoConnection {
    pub async fn start(client_options: Option<ClientOptions>) -> Result<Self> {
        let cl = Self::connect_mongo(client_options).await?;
        cl.clone();
        let (sx, rx) = mpsc::channel(50);
        let s = Self { sender: sx };
        Self::listen(cl, rx);
        Ok(s)
    }

    ///trys to connect to a mongo database with the provided options, if no options
    ///are provided default options are used and the functions looks for a localhost
    ///instance of mongodb
    ///
    ///the client internally uses connection pooling in order to increase performance
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

    fn listen(mut client: Client, mut reciever: mpsc::Receiver<Command>) {
        tokio::spawn(async move {});
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
