use futures::TryStreamExt;
use mongodb;
use mongodb::bson::doc;
use mongodb::{Collection, Cursor};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio;

#[derive(Debug, Serialize, Deserialize)]
struct SomeCustomType {
    foo: String,
    bar: Vec<i32>,
}

async fn insert<'a, T>(data: T, collection: &Collection<T>) -> Result<(), mongodb::error::Error>
where
    T: Serialize + DeserializeOwned,
{
    collection.insert_one(data, None).await?;
    Ok(())
}

async fn retrieve<'a, T>(
    collection: &Collection<T>,
    filter: mongodb::bson::Document,
    options: mongodb::options::FindOptions,
) -> Result<Option<T>, mongodb::error::Error>
where
    T: DeserializeOwned + 'a,
{
    let mut cursor: Cursor<T> = collection.find(filter, options).await?;
    let result = cursor.try_next().await?;
    return Ok(result);
}

#[tokio::main]
async fn main() {
    let client_options = mongodb::options::ClientOptions::parse("mongodb://localhost:27017")
        .await
        .unwrap();
    let client = mongodb::Client::with_options(client_options).unwrap();
    let db = client.database("minimal");
    let collection = db.collection::<SomeCustomType>("users");

    insert(
        SomeCustomType {
            foo: String::from("Hello World"),
            bar: Vec::new(),
        },
        &collection,
    )
    .await
    .unwrap();

    println!("result: {:?}", retrieve(&collection, doc! {"foo" : "Hello World"},mongodb::options::FindOptions::default()).await.unwrap().unwrap())
}
