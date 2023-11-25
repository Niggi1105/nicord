mod core;
mod authentication;
mod mongodb;

use log::error;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .target(env_logger::Target::Stdout)
        .filter_level(log::LevelFilter::Info)
        .is_test(false)
        .init();
    let client = match  mongodb::connect_mongo(None).await{
        Err(err) => {
            error!("Can't connect to mongodb {:?}", err);
            panic!();
        }
        Ok(cl) => cl,
    };

    match core::accept_new_connections(client).await {
        Ok(_) => {
            println!("no error");
        }
        Err(e) => {
            println!("did error {:?}", e)
        }
    }
}
