pub(crate) mod bot;
pub mod config;
pub(crate) mod queue;

use app_tasks::TaskRunner;
use queue::TaskQueueProcessor;
use tracing::{debug, error};

use crate::config::Config;

#[tokio::main]
async fn main() {
    let loaded_dotenv = dotenvy::dotenv();

    app_logger::init();

    match loaded_dotenv {
        Ok(loaded_dotenv) => {
            debug!(path = ?loaded_dotenv, "Loaded dotenv file");
        }
        Err(e) if e.not_found() => {
            debug!("No dotenv file found");
        }
        Err(e) => {
            error!("Failed to load dotenv file: {e:?}");
            panic!("Failed to load dotenv file: {e:?}");
        }
    }

    let config = Config::init_parsed().expect("Failed to init config");

    debug!(config = ?*config, "Running with config");

    tokio::task::spawn(TaskQueueProcessor::run());
    tokio::task::spawn(TaskRunner::run());

    bot::run().await.expect("Failed to run server");
}
