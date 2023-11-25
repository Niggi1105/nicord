use anyhow::Result;
use common::user::User;
use mongodb::bson::doc;
use mongodb::options::{ClientOptions, IndexOptions};
use mongodb::{Client, IndexModel};
use crate::authentication::{USER_COLLECTION, USER_DATABASE};

/// trys to connect to a mongo database with the provided options, if no options
/// are provided default options are used and the functions looks for a localhost
/// instance of mongodb
///
/// the client internally uses connection pooling in order to increase performance
pub async fn connect_mongo(opts: Option<ClientOptions>) -> Result<Client> {
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

pub async fn setup_user_db(client: &Client) -> Result<()> {
    let db = client.database(USER_DATABASE);
    let coll: mongodb::Collection<User> = db.collection(USER_COLLECTION);
    let model = IndexModel::builder()
        .keys(doc! {"username": 1})
        .options(IndexOptions::builder()
                 .unique(true)
                 .build())
        .build();
    coll.create_index(model, None).await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::test;
    #[test]
    async fn make_connection() {
        connect_mongo(None).await.unwrap();
    }
}
