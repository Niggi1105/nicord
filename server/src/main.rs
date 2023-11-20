mod core;
mod mongodb;
mod nc_server;

use env_logger;
use log::error;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .target(env_logger::Target::Stdout)
        .filter_level(log::LevelFilter::Info)
        .is_test(false)
        .init();
    let (connection_handler, channel) = match mongodb::MongoConnectionHandler::new(None).await {
        Err(err) => {
            error!("Can't connect to mongodb {:?}", err);
            panic!();
        }
        Ok(cl) => cl,
    };

    match core::accept_new_connections(channel).await {
        Ok(_) => {
            println!("no error");
        }
        Err(e) => {
            println!("did error {:?}", e)
        }
    }
}
