use serde::{Deserialize, Serialize};
use tokio;

#[derive(Debug, Deserialize, Serialize)]
struct Cat<'a> {
    #[serde(borrow)]
    name: &'a str,
}

impl Cat<'_>{
    pub fn new() -> Self {
        Cat { name: "Tom" }
    }
}


#[tokio::main]
async fn main() {
    let client_options = mongodb::options::ClientOptions::parse("mongodb://localhost:27017")
        .await
        .unwrap();
    let client = mongodb::Client::with_options(client_options).unwrap();
    let db = client.database("minimal");
    let coll = db.collection::<Cat>("cat");
    coll.insert_one(Cat::new(), None).await.unwrap();    
    let mut cursor = coll.find(None, None).await.unwrap();
    while cursor.advance().await.unwrap() {
        println!("got something");
        println!("{:?}", cursor.deserialize_current().unwrap());
    }
}
