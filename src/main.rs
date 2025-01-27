use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::rpc::types::{BlockNumberOrTag, BlockTransactionsKind};
use alloy::transports::http::{Client, Http};
use dotenv::dotenv;
use eyre::Result;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

// TODO: Replace expects in requests with something no causing panic

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let rpc_url = env::var("NODE_URL")
        .expect("Cannot find NODE_URL")
        .parse()?;

    let provider = ProviderBuilder::new().on_http(rpc_url);
    let cold_start_blocks = cold_start(provider.clone()).await.unwrap_or([].to_vec());
    let blocks = Arc::new(Mutex::new(cold_start_blocks.clone()));
    let mut interval = time::interval(Duration::from_secs(12));

    // Main block watcher
    loop {
        interval.tick().await;

        // Lock the shared state
        let mut blocks = blocks.lock().await;
        let last_block_number = blocks
            .get(blocks.len() - 1)
            .expect("Failed to get last block")
            .header
            .number;

        let new_block = fetch_new_block(provider.clone(), last_block_number).await;
        match new_block {
            Some(block) => {
                println!("New block: {}", block.header.number);
                println!("Block hash: {}", block.header.hash);
                blocks.push(block);
                println!("Blocks count: {:?}", blocks.len());
            }
            None => {}
        }
    }
}

async fn cold_start(provider: RootProvider<Http<Client>>) -> Result<Vec<alloy::rpc::types::Block>> {
    let last_finalized_block =
        get_block_by_number_or_tag(provider.clone(), BlockNumberOrTag::Finalized).await?;

    let latest_block =
        get_block_by_number_or_tag(provider.clone(), BlockNumberOrTag::Latest).await?;

    let start_block_number = last_finalized_block.header.number - 11;
    let end_block_number = latest_block.header.number - 1;

    let mut blocks = vec![];

    for block_number in start_block_number..=end_block_number {
        let block =
            get_block_by_number_or_tag(provider.clone(), BlockNumberOrTag::Number(block_number))
                .await?;
        blocks.push(block);
    }
    return Ok(blocks);
}

async fn fetch_new_block(
    provider: RootProvider<Http<Client>>,
    last_block_number: u64,
) -> Option<alloy::rpc::types::Block> {
    let latest_block_number = provider
        .get_block_number()
        .await
        .expect("Failed to fetch block number");

    if latest_block_number > last_block_number {
        let block = get_block_by_number_or_tag(provider, BlockNumberOrTag::Latest).await;

        if block.is_ok() {
            return Some(block.unwrap());
        }
        return None;
    }

    return None;
}

async fn get_block_by_number_or_tag(
    provider: RootProvider<Http<Client>>,
    tag: BlockNumberOrTag,
) -> Result<alloy::rpc::types::Block> {
    let block = provider
        .get_block_by_number(tag, BlockTransactionsKind::Hashes)
        .await
        .expect("Failed to fetch block")
        .expect("Block not found");

    return Ok(block);
}
