use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::rpc::types::{Block, BlockNumberOrTag, BlockTransactionsKind};
use alloy::transports::http::{Client, Http};
use dotenv::dotenv;
use eyre::Result;
use once_cell::sync::Lazy;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

// TODO: Replace expects in requests with something no causing panic

static PROVIDER: Lazy<Arc<RootProvider<Http<Client>>>> = Lazy::new(|| {
    let rpc_url = env::var("NODE_URL")
        .expect("Cannot find NODE_URL")
        .parse()
        .expect("Failed to parse NODE_URL");
    let provider = ProviderBuilder::new().on_http(rpc_url);
    Arc::new(provider)
});

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

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

async fn cold_start(
    blocks: Arc<Mutex<Vec<Block>>>,
) -> Result<(), Box<dyn std::error::Error + Send>> {
    let last_finalized_block = get_block_by_number_or_tag(BlockNumberOrTag::Finalized)
        .await
        .expect("Can't fetch last finalized block");
    let latest_block = get_block_by_number_or_tag(BlockNumberOrTag::Latest)
        .await
        .expect("Can't fetch latest block");

    let start_block_number = last_finalized_block.header.number - 11;
    let end_block_number = latest_block.header.number - 1;

    let mut new_blocks = vec![];

    for block_number in start_block_number..=end_block_number {
        let block = get_block_by_number_or_tag(BlockNumberOrTag::Number(block_number))
            .await
            .expect("Can't fetch block");
        new_blocks.push(block);
    }
    blocks.lock().await.splice(0..0, new_blocks.iter().cloned());
    return Ok(());
}

async fn fetch_new_block(last_block_number: u64) -> Option<alloy::rpc::types::Block> {
    let provider = &*PROVIDER;
    let latest_block_number = provider
        .get_block_number()
        .await
        .expect("Failed to fetch block number");

    if latest_block_number > last_block_number {
        let block = get_block_by_number_or_tag(BlockNumberOrTag::Latest).await;

        if block.is_ok() {
            return Some(block.unwrap());
        }
        return None;
    }

    return None;
}

async fn get_block_by_number_or_tag(tag: BlockNumberOrTag) -> Result<alloy::rpc::types::Block> {
    let provider = &*PROVIDER;

    let block = provider
        .get_block_by_number(tag, BlockTransactionsKind::Hashes)
        .await
        .expect("Failed to fetch block")
        .expect("Block not found");

    return Ok(block);
}
