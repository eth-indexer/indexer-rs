use eyre::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

mod blocks;
use blocks::{cold_start, fetch_new_block};

// TODO: Replace expects in requests with something no causing panic

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let blocks = Arc::new(Mutex::new([].to_vec()));
    let shared_blocks = Arc::clone(&blocks);

    tokio::spawn(cold_start(shared_blocks));

    // Main block watcher
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        // Lock the shared state
        let mut blocks = blocks.lock().await;
        let last_block_number = if blocks.len() > 0 {
            blocks
                .get(blocks.len() - 1)
                .expect("Failed to get last block")
                .header
                .number
        } else {
            0
        };

        let new_block = fetch_new_block(last_block_number).await;
        match new_block {
            Some(block) => {
                println!("New block: {}", block.header.number);
                println!("Block hash: {}", block.header.hash);
                blocks.push(block);
                println!(
                    "Block numbers: {:?}",
                    blocks.iter().map(|b| b.header.number).collect::<Vec<u64>>()
                );
            }
            None => {}
        }
        interval.tick().await;
    }
}
