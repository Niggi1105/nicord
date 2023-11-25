mod core;
mod authentication;
mod mongodb;

use log::{error, info};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .target(env_logger::Target::Stdout)
        .filter_level(log::LevelFilter::Debug)
        .is_test(false)
        .init();
    info!("connecting to database...");
    let client = match  mongodb::connect_mongo(None).await{
        Err(err) => {
            error!("Can't connect to mongodb {:?}", err);
            panic!();
        }
        Ok(cl) => cl,
    };
    info!("connection established");
    info!("setting up userdb...");
    match mongodb::setup_user_db(&client).await{
        Ok(_) => {}
        Err(e) => {
            error!("Can't setup user db: {:?}", e);
            panic!("Crittical Database Error")
        }
    }
    info!("setup finished");
    info!("start listening for connections...");
    match core::accept_new_connections(client).await {
        Ok(_) => {
            println!("no error");
        }
        Err(e) => {
            println!("did error {:?}", e)
        }
    }
}
