use alloy::contract::{ContractInstance, Interface};
use alloy::primitives::address;
use alloy::providers::RootProvider;
use alloy::transports::http::{Client, Http};
use once_cell::sync::Lazy;
use std::sync::Arc;

use crate::blocks::utils::PROVIDER;

pub static NO_REGISTRY_CONTRACT: Lazy<
    Arc<ContractInstance<Http<Client>, Arc<RootProvider<Http<Client>>>>>,
> = Lazy::new(|| {
    let provider = &*PROVIDER;
    let path = std::env::current_dir()
        .expect("Can't find NoRegistryContract ABI")
        .join("src/abi/no_registry_abi.json");

    let artifact = std::fs::read(path).expect("Failed to read artifact");
    let json: serde_json::Value = serde_json::from_slice(&artifact).expect("Can't parse abi json");
    let abi = serde_json::from_str(&json.to_string()).expect("Failed to parse ABI");

    let contract: ContractInstance<Http<Client>, _> = ContractInstance::new(
        address!("0x595F64Ddc3856a3b5Ff4f4CC1d1fb4B46cFd2bAC"),
        provider.clone(),
        Interface::new(abi),
    );

    Arc::new(contract)
});
