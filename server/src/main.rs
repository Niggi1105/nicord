mod core;
mod authentication;
mod mongodb;
mod user;
mod session;

use authentication::AuthHandler;
use log::{error, info};
use session::SessionHandler;
use user::UserHandler;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .target(env_logger::Target::Stdout)
        .filter_level(log::LevelFilter::Debug)
        .is_test(false)
        .init();
    let client = match  mongodb::connect_mongo(None).await{
        Err(err) => {
            error!("Can't connect to mongodb {:?}", err);
            panic!();
        }
        Ok(cl) => cl,
    };

    let ufrom_names = Arc::new(SessionHandler::from_names(&client, "SESSIONS", "sessions"));
    let sfrom_names = Arc::new(UserHandler::from_names(&client, "USERS", "users"));
    let auth_handler = AuthHandler::new(ufrom_names, sfrom_names);

    match core::accept_new_connections(client, auth_handler).await {
        Ok(_) => {
            println!("no error");
        }
        Err(e) => {
            println!("did error {:?}", e)
        }
    }
}
