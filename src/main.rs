use eyre::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};
use tracing::{error, info};
use tracing_subscriber;

pub mod blocks;
pub mod no_registry_contract;
use blocks::{
    check_reorg, cold_start, fetch_new_block, reorganize_blocks, trim_extra_finalized_blocks,
};

mod blocks_observer;
use blocks_observer::{Event, Publisher};
mod signing_keys;
// use signing_keys::get_signing_keys;

// TODO: Replace expects in requests with something no causing panic

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let blocks = Arc::new(Mutex::new(vec![]));
    let event_publisher = Arc::new(Mutex::new(Publisher::default()));

    event_publisher
        .lock()
        .await
        .subscribe(Event::BlocksChanged, |new_blocks| {
            if let Some(new_blocks) = new_blocks {
                info!(
                    "Block numbers: {:?} {}",
                    new_blocks
                        .iter()
                        .map(|b| b.header.number)
                        .collect::<Vec<u64>>(),
                    new_blocks.len()
                );
            }
        });

    event_publisher.lock().await.subscribe(Event::Reorg, |_| {
        info!("Rorg detected");
    });

    // Run cold start task
    let shared_blocks = Arc::clone(&blocks);
    tokio::spawn(cold_start(shared_blocks, event_publisher.clone()));

    // Main block watcher
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        let last_block_number;

        // Acquire the lock
        {
            let blocks_guard = blocks.lock().await;
            last_block_number = if !blocks_guard.is_empty() {
                blocks_guard
                    .last()
                    .expect("Failed to get last block")
                    .header
                    .number
            } else {
                0
            };
        }

        let new_block = fetch_new_block(last_block_number).await;
        if let Some(block) = new_block {
            info!("New block: {}", block.header.number);
            info!("Block hash: {}", block.header.hash);

            // Acquire the lock
            {
                let mut blocks_guard = blocks.lock().await;
                blocks_guard.push(block.clone());
                event_publisher
                    .lock()
                    .await
                    .blocks_changed(Event::BlocksChanged, blocks_guard.clone());
            }

            let shared_blocks = Arc::clone(&blocks);
            if check_reorg(shared_blocks).await {
                event_publisher.lock().await.reorg();

                let shared_blocks = Arc::clone(&blocks);
                tokio::spawn(async move {
                    if let Err(e) = reorganize_blocks(shared_blocks).await {
                        error!("Error during reorganization: {:?}", e);
                    }
                });
            } else {
                info!("No reorg detected");
            }

            // Run trimming extra finalized blocks task
            let shared_blocks = Arc::clone(&blocks);
            tokio::spawn(trim_extra_finalized_blocks(shared_blocks));
        }

        interval.tick().await;
    }
}
