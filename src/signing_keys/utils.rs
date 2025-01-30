use alloy::dyn_abi::DynSolValue;
use alloy::eips::BlockId;
use alloy::primitives::U256;
use tracing::{error, info};

use crate::contracts::no_registry_contract::NO_REGISTRY_CONTRACT;

const MAX_BATCH_SIZE: u64 = 500;

pub async fn get_signing_keys(block_id: BlockId) {
    // TODO: fetch operator ids and then fetch signing keys for each operator
    // let operator_id = 0;
    // get_operator_keys(operator_id, block_id).await;
}

async fn get_operator_keys(operator_id: u64, block_id: BlockId) {
    let total_signing_keys_count = get_signing_keys_count(
        operator_id,
        block_id, // BlockId::Number(last_finalized_block_number.into()),
    )
    .await;

    let total_signing_keys_count = match total_signing_keys_count {
        Some(count) => count,
        None => return,
    };

    info!(
        "Total signing keys count for operator {}: {}",
        operator_id, total_signing_keys_count
    );

    // let batches_count = (total_signing_keys_count / MAX_BATCH_SIZE).ceil();

    // TODO: Fetch signing keys
}

async fn get_signing_keys_count(operator_id: u64, block_id: BlockId) -> Option<U256> {
    let contract = &*NO_REGISTRY_CONTRACT;

    let total_signing_keys_count = contract
        .function(
            "getTotalSigningKeyCount",
            &[DynSolValue::from(U256::from(operator_id))],
        )
        .expect("No getToalSigningKeyCount function")
        .block(block_id)
        .call()
        .await;

    let total_signing_keys_count = match total_signing_keys_count {
        Ok(count) => count,
        Err(e) => {
            error!("Failed to fetch total signing keys count: {:?}", e);
            return None;
        }
    };

    return Some(
        total_signing_keys_count
            .first()
            .unwrap()
            .as_uint()
            .unwrap()
            .0,
    );
}
