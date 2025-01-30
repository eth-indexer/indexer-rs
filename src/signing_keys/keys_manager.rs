use alloy::rpc::types::Block;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use crate::blocks::blocks_observer::{Event, Publisher};

#[derive(Default)]
pub struct SigningKeysManager {
    op_stack: Arc<Mutex<Vec<Block>>>,
}

impl SigningKeysManager {
    pub fn new() -> Self {
        SigningKeysManager {
            op_stack: Arc::new(Mutex::new(vec![])),
        }
    }

    pub async fn listen(&mut self, event_publisher: Arc<Mutex<Publisher>>) {
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
    }
}
