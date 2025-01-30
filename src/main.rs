use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber;

pub mod blocks;
pub mod contracts;
pub mod signing_keys;
use blocks::block_watcher::BlockWatcher;
use blocks::blocks_observer::Publisher;
use signing_keys::keys_manager::SigningKeysManager;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let event_publisher = Arc::new(Mutex::new(Publisher::default()));
    let mut block_watcher = BlockWatcher::new(event_publisher.clone());
    let mut signing_keys_manager = SigningKeysManager::new();

    let _ = signing_keys_manager.listen(event_publisher.clone()).await;
    let _ = block_watcher.watch().await;
}
