use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, env};
use eth_tests::address_manager::{self, AddressManager};

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref TEMPLATE_GENESIS_FILE: String = env::var("TEMPLATE_GENESIS_FILE").expect("TEMPLATE_GENESIS_FILE must be set");
    static ref GENESIS_FILE: String = env::var("GENESIS_FILE").expect("GENESIS_FILE must be set");
    static ref MNEMONIC: String = env::var("MNEMONIC").expect("MNEMONIC must be set");
    static ref CHAIN_ID: u64 = env::var("CHAIN_ID").expect("CHAIN_ID must be set").parse().expect("CHAIN_ID must be integer");
}

#[derive(Debug, Serialize, Deserialize)]
struct GenesisConfig {
    ethash: Value,
    chainId: u64,
    homesteadBlock: u64,
    eip150Block: u64,
    eip155Block: u64,
    eip158Block: u64,
    byzantiumBlock: u64,
    constantinopleBlock: u64,
    petersburgBlock: u64,
    istanbulBlock: u64,
    // muirGlacierBlock: Option<u64>,
    berlinBlock: u64,
    londonBlock: u64,
    // arrowGlacierBlock: Option<u64>,
    // grayGlacierBlock: Option<u64>,
    // mergeNetsplitBlock: Option<u64>,
    terminalTotalDifficulty: u64,
    terminalTotalDifficultyPassed: bool,
    shanghaiTime: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Genesis {
    nonce: String,
    timestamp: String,
    extraData: String,
    gasLimit: String,
    difficulty: String,
    mixHash: String,
    coinbase: String,
    alloc: Value,  // Using serde_json::Value to allow arbitrary JSON
    number: String,
    gasUsed: String,
    parentHash: String,
    config: GenesisConfig,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load env variables
    let env_path = ".env.generate_genesis_reth";
    dotenv::from_path(env_path).ok();

    // Load the template genesis file
    let data = fs::read_to_string("template_reth_genesis.json")?;
    let mut genesis: Genesis = serde_json::from_str(&data)?;

    let address_manager_path = "../converted_evmos_mainnet_transactions_03/address_manager.json";
    let address_manager = AddressManager::load_from_file(address_manager_path).unwrap();
    let (_, addresses_size) = address_manager.info();

    // Create allocations for N accounts
    let mut new_allocations = serde_json::Map::new();
    for index in 0..addresses_size {
        let address = address_manager.get_converted_address_with_index(index);
        println!("Address: {}", address);
        let balance = "0x100000000000000000000000";  // Example balance
        new_allocations.insert(address, json!({ "balance": balance }));
    }

    // Merge new allocations with existing ones
    match genesis.alloc.as_object_mut() {
        Some(alloc) => {
            for (key, value) in new_allocations {
                alloc.insert(key, value);
            }
        }
        None => {
            genesis.alloc = Value::Object(new_allocations);
        }
    }

    // Serialize the updated genesis back to JSON
    let updated_data = serde_json::to_string_pretty(&genesis)?;

    // Write the updated JSON back to a file
    fs::write(&*GENESIS_FILE, updated_data)?;

    println!("Genesis file updated successfully.");

    Ok(())
}
