use anyhow::Result;
use mongodb::options::ClientOptions;
use mongodb::Client;

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

#[cfg(test)]
mod test {
    use super::*;
    use tokio::test;
    #[test]
    async fn make_connection_wtihout_options() {
        connect_mongo(None).await.unwrap();
    }
    #[test]
    async fn make_connection_wtih_options() {
        connect_mongo(Some(ClientOptions::parse("mongodb://localhost:27017").await.unwrap())).await.unwrap();
    }
    #[test]
    async fn make_connection_invalid_connection_string() {
        connect_mongo(Some(ClientOptions::parse("mongodb://localhost:27000").await.unwrap())).await.unwrap_err();
    }
}
