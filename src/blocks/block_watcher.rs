use alloy::rpc::types::Block;
use eyre::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};
use tracing::{error, info};

use crate::blocks::blocks_observer::{Event, Publisher};
use crate::blocks::utils::{
    check_reorg, cold_start, fetch_new_block, reorganize_blocks, trim_extra_finalized_blocks,
};

#[derive(Default)]
pub struct BlockWatcher {
    blocks: Arc<Mutex<Vec<Block>>>,
    event_publisher: Arc<Mutex<Publisher>>,
}

impl BlockWatcher {
    pub fn new(event_publisher: Arc<Mutex<Publisher>>) -> Self {
        BlockWatcher {
            blocks: Arc::new(Mutex::new(vec![])),
            event_publisher,
        }
    }

    pub async fn watch(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Run cold start task
        let shared_blocks = Arc::clone(&self.blocks);
        tokio::spawn(cold_start(shared_blocks, self.event_publisher.clone()));

        // Main block watcher
        let mut interval = time::interval(Duration::from_secs(5));
        loop {
            let last_block_number;

            // Acquire the lock
            {
                let blocks_guard = self.blocks.lock().await;
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
                    let mut blocks_guard = self.blocks.lock().await;
                    blocks_guard.push(block.clone());
                    self.event_publisher
                        .lock()
                        .await
                        .blocks_changed(blocks_guard.clone());
                }

                let shared_blocks = Arc::clone(&self.blocks);
                if check_reorg(shared_blocks).await {
                    self.event_publisher.lock().await.reorg();

                    let shared_blocks = Arc::clone(&self.blocks);
                    tokio::spawn(async move {
                        if let Err(e) = reorganize_blocks(shared_blocks).await {
                            error!("Error during reorganization: {:?}", e);
                        }
                    });
                } else {
                    info!("No reorg detected");
                }

                // Run trimming extra finalized blocks task
                let shared_blocks = Arc::clone(&self.blocks);
                tokio::spawn(trim_extra_finalized_blocks(shared_blocks));
            }

            interval.tick().await;
        }
    }
}
