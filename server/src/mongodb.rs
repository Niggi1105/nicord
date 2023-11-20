use futures::TryStreamExt;
use mongodb::bson::Document;
use mongodb::error::Error;
use mongodb::options::{self, ClientOptions, FindOptions};

use mongodb::{Client, Collection};
use serde::de::DeserializeOwned;
use serde::Serialize;

use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::error::Elapsed;

type Result<T> = std::result::Result<T, DatabaseConncectionError>;

/// Enum encapsulating tokio Timeout error, in case the mongodb server doesn't
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

///Command wraper for communication with Mmongodb connection layer through channels
pub enum Command {
    NewServer(String),
}

pub struct MongoConnection {
    client: Client,
    sender: mpsc::Sender<Command>,
}

impl MongoConnection {
    pub async fn new(client_options: Option<ClientOptions>) -> Result<Self> {
        let cl = Self::connect_mongo(client_options).await?;
        let (sx, rx) = mpsc::channel(50);
        let mut s = Self {
            client: cl,
            sender: sx,
        };
        s.listen(rx);
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

    /// retrieves the first match to the given filter in the Collection
    pub async fn retrieve<'a, T>(
        collection: &Collection<T>,
        filter: Document,
        options: FindOptions,
    ) -> Result<Option<Vec<T>>>
    where
        T: DeserializeOwned + 'a,
    {
        let mut cursor = collection.find(filter, options).await?;
        let mut result = Vec::new();
        while cursor.advance().await? {
            result.push(cursor.deserialize_current()?);
        }
        Ok(Some(result))
    }

    /// insert the data into the collection
    async fn insert<'a, T>(data: &T, collection: &Collection<T>) -> Result<()>
    where
        T: Serialize + DeserializeOwned,
    {
        collection.insert_one(data, None).await?;
        Ok(())
    }

    async fn execute_cmd(client: &mut mongodb::Client, cmd: &Command) -> Result<()> {
        match cmd {
            Command::NewServer(name) => {
                let db = client.database(&name);
                db.create_collection(".config", options::CreateCollectionOptions::default())
                    .await?;
                Self::insert(&serer_configs, &db.collection(".config")).await?;
            }
        }
        Ok(())
    }
    fn listen(&mut self, reciever: mpsc::Receiver<Command>) -> Result<()> {
        tokio::spawn(async move {
            if let Some(cmd) = reciever.recv().await {
                Self::execute_cmd(&mut self.client, &cmd);
            }
        });
        Ok(())
    }
}
