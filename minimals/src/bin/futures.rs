// This trait is required to use `try_next()` on the cursor
use futures::stream::TryStreamExt;
use mongodb::{bson::doc, options::FindOptions};
use serde::{Deserialize, Serialize};
use tokio;

#[derive(Debug, Serialize, Deserialize)]
struct Book {
    title: String,
    author: String,
}

#[tokio::main]
async fn main() {
    // Query the books in the collection with a filter and an option.
    let client_options = mongodb::options::ClientOptions::parse("mongodb://localhost:27017")
        .await
        .unwrap();
    let client = mongodb::Client::with_options(client_options).unwrap();
    println!("{:?}", client.list_database_names(None, None).await);
    let db = client.database("minimal");
    println!("{:?}", db.list_collection_names(None).await);
    let collection = db.collection::<Book>("books");

    let books = vec![
        Book {
            title: "The Grapes of Wrath".to_string(),
            author: "John Steinbeck".to_string(),
        },
        Book {
            title: "To Kill a Mockingbird".to_string(),
            author: "Harper Lee".to_string(),
        },
    ];

    // Insert the books into "mydb.books" collection, no manual conversion to BSON necessary.
    collection.insert_many(books, None).await.unwrap();

    let filter = doc! { "author": "George Orwell" };
    let find_options = FindOptions::builder().sort(doc! { "title": 1 }).build();
    let mut cursor = collection.find(filter, find_options).await.unwrap();

    // Iterate over the results of the cursor.
    while let Some(book) = cursor.try_next().await.unwrap() {
        println!("title: {}", book.title);
    }
}
