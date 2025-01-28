use eyre::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

mod blocks;
use blocks::{cold_start, fetch_new_block, trim_extra_finalized_blocks};

// TODO: Replace expects in requests with something no causing panic

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let blocks = Arc::new(Mutex::new(vec![]));

    // Run cold start task
    let shared_blocks = Arc::clone(&blocks);
    tokio::spawn(cold_start(shared_blocks));

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
            println!("New block: {}", block.header.number);
            println!("Block hash: {}", block.header.hash);

            // Acquire the lock
            {
                let mut blocks_guard = blocks.lock().await;
                blocks_guard.push(block.clone()); // Push the new block into the vector

                println!(
                    "Block numbers: {:?} {}",
                    blocks_guard
                        .iter()
                        .map(|b| b.header.number)
                        .collect::<Vec<u64>>(),
                    blocks_guard.len()
                );
            }

            // Run trimming extra finalized blocks task
            let shared_blocks = Arc::clone(&blocks);
            tokio::spawn(trim_extra_finalized_blocks(shared_blocks));
        }

        interval.tick().await;
    }
}
