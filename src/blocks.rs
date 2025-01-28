use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::rpc::types::{Block, BlockNumberOrTag, BlockTransactionsKind};
use alloy::transports::http::{Client, Http};
use dotenv::dotenv;
use eyre::Result;
use futures::future::join_all;
use once_cell::sync::Lazy;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

// TODO: Replace expects in requests with something no causing panic

static PROVIDER: Lazy<Arc<RootProvider<Http<Client>>>> = Lazy::new(|| {
    dotenv().ok();
    let rpc_url = env::var("NODE_URL")
        .expect("Cannot find NODE_URL")
        .parse()
        .expect("Failed to parse NODE_URL");
    let provider = ProviderBuilder::new().on_http(rpc_url);
    Arc::new(provider)
});

pub async fn cold_start(
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

    let fetch_blocks_futures = (start_block_number..=end_block_number)
        .map(|block_number| get_block_by_number_or_tag(BlockNumberOrTag::Number(block_number)));

    let fetched_blocks: Vec<Block> = join_all(fetch_blocks_futures)
        .await
        .into_iter()
        .filter_map(Result::ok)
        .collect();

    blocks.lock().await.splice(0..0, fetched_blocks);
    return Ok(());
}

pub async fn fetch_new_block(last_block_number: u64) -> Option<alloy::rpc::types::Block> {
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

pub async fn get_block_by_number_or_tag(tag: BlockNumberOrTag) -> Result<alloy::rpc::types::Block> {
    let provider = &*PROVIDER;

    let block = provider
        .get_block_by_number(tag, BlockTransactionsKind::Hashes)
        .await
        .expect("Failed to fetch block")
        .expect("Block not found");

    return Ok(block);
}

pub async fn trim_extra_finalized_blocks(
    blocks: Arc<Mutex<Vec<Block>>>,
) -> Result<(), Box<dyn std::error::Error + Send>> {
    let finalized_block = get_block_by_number_or_tag(BlockNumberOrTag::Finalized)
        .await
        .expect("Can't fetch finalized block");

    blocks
        .lock()
        .await
        .retain(|block| block.header.number >= finalized_block.header.number - 11);

    return Ok(());
}

pub async fn check_reorg(blocks: Arc<Mutex<Vec<Block>>>) -> bool {
    let blocks_guard = blocks.lock().await;

    let last_block = match blocks_guard.last() {
        Some(block) => block,
        None => return false,
    };

    let parent_block = blocks_guard
        .iter()
        .rev()
        .skip(1)
        .find(|block| block.header.number == last_block.header.number - 1);

    let parent_block = match parent_block {
        Some(block) => block,
        None => return false,
    };

    last_block.header.parent_hash != parent_block.header.hash
}

pub async fn reorganize_blocks(
    blocks: Arc<Mutex<Vec<Block>>>,
) -> Result<(), Box<dyn std::error::Error + Send>> {
    // Lock the blocks vector
    let mut blocks_guard = blocks.lock().await;

    for block in blocks_guard.iter_mut().rev() {
        let expected_block =
            get_block_by_number_or_tag(BlockNumberOrTag::Number(block.header.number))
                .await
                .expect("Failed to fetch block");

        if block.header.hash != expected_block.header.hash {
            info!(
                "Reorg detected at block number {}. Refetching...",
                block.header.number
            );
            info!("Old block hash: {}", block.header.hash);

            // Replace the block with the updated block
            *block = expected_block;
            info!("Updated! New block hash: {}", block.header.hash);
        }
    }

    info!("Reorganization complete.");
    Ok(())
}
