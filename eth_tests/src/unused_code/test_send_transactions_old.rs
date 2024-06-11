use tokio::fs;
use serde_json;
use ethers::types::TransactionRequest;
use std::env;
use std::path::Path;
use eth_tests::address_manager::{self, AddressManager};

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref LOCAL_URL: String = env::var("LOCAL_URL").expect("LOCAL_URL must be set");
    static ref CONVERTED_TRANSACTIONS_OUTPUT_DIR: String = env::var("CONVERTED_TRANSACTIONS_OUTPUT_DIR").expect("CONVERTED_TRANSACTIONS_OUTPUT_DIR must be set");
    static ref MNEMONIC: String = env::var("MNEMONIC").expect("MNEMONIC must be set");
    static ref CHAIN_ID: u64 = env::var("CHAIN_ID").expect("CHAIN_ID must be set").parse().expect("CHAIN_ID must be integer");
}

mod tester {
    use ethers::prelude::*;
    use ethers::signers::coins_bip39::English;
    use ethers::core::k256::ecdsa::SigningKey;
    use std::convert::TryFrom;
    use ethers::types::transaction::eip2718::TypedTransaction;

    pub async fn create_wallet(mnemonic: &str, index: u32) -> Result<Wallet<SigningKey>, Box<dyn std::error::Error>> {
        use crate::CHAIN_ID;
        Ok(MnemonicBuilder::<English>::default()
            .phrase(mnemonic)
            .index(index)?
            .build()?
            .with_chain_id(*CHAIN_ID))
    }

    pub async fn send_transaction(
        url: &str,
        wallet: &Wallet<SigningKey>,
        mut transaction: TransactionRequest,
    ) -> Result<TxHash, Box<dyn std::error::Error>> {
        // Connect to an Ethereum node using HTTP
        let provider = Provider::<Http>::try_from(url)?;
        let chain_id = provider.get_chainid().await?;
        println!("Chain ID: {}", chain_id);
        println!("Block number: {}", provider.get_block_number().await?);
    
        // Retrieve the sender's address and fetch the nonce
        let from_address = transaction.from.expect("Sender address not specified");
        let nonce = provider.get_transaction_count(from_address, None).await?;
        transaction = transaction.nonce(nonce);
        // transaction.chain_id(chain_id);
        println!("{:?}", transaction);

        // Sign the transaction
        let transaction: TypedTransaction = transaction.into();
        // let signature = wallet.sign_transaction(&transaction).await?;

        // Send the signed transaction
        // let raw_tx = transaction.rlp_signed(&signature);
        let result = provider.send_transaction(transaction, None).await?;
        // let result = provider.send_raw_transaction(raw_tx).await?;
        println!("Result {:?}", result);
        Ok(result.tx_hash())
    }
    

    pub async fn get_balance(url: &str, address: &H160, block: Option<BlockId>) -> Result<U256, Box<dyn std::error::Error>> {
        let provider = Provider::<Http>::try_from(url).expect("Cannot connect URL");
        let balance = provider.get_balance(*address, block).await?;
        Ok(balance)
    }

    pub async fn get_transaction_by_hash(url: &str, tx_hash: TxHash) -> Result<Option<Transaction>, Box<dyn std::error::Error>> {
        let provider = Provider::<Http>::try_from(url).expect("Cannot connect URL");
        let tx = provider.get_transaction(tx_hash).await?;
        Ok(tx)
    }
}

async fn send_1_transaction_from_path(path: &str, address_manager: &AddressManager) -> Result<(), Box<dyn std::error::Error>> {
    let mnemonic = (*MNEMONIC).as_str();
    let url = (*LOCAL_URL).as_str();
    let content = fs::read_to_string(path).await?;
    let transaction: TransactionRequest = serde_json::from_str(&content)?;
    println!("{:?}", transaction);

    let from_address = transaction.from.expect("Sender address not specified");
    let index = address_manager.get_index_with_converted_address(&from_address).await
        .expect("Cannot retrieve mnemonic index of transaction sender address");
    let wallet = tester::create_wallet(mnemonic, index as u32).await?;
    let tx_hash = tester::send_transaction(&url, &wallet, transaction).await?;

    use tokio::time::{sleep, Duration};
    sleep(Duration::from_millis(1000)).await;

    let balance = tester::get_balance(url, &from_address, None).await?;
    let tx = tester::get_transaction_by_hash(url, tx_hash).await?.unwrap();
    println!("Balance of {}: {}", hex::encode(from_address), balance);
    println!("{:?}", tx);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load env variables
    let env_path = ".env.evmos_mainnet";
    dotenv::from_path(env_path).ok();
    
    let transactions_folder = format!("{}/transactions", *CONVERTED_TRANSACTIONS_OUTPUT_DIR);
    let url = (*LOCAL_URL).as_str();
    let mnemonic = (*MNEMONIC).as_str();

    let path = format!("{}/address_manager.json", *CONVERTED_TRANSACTIONS_OUTPUT_DIR);
    let address_manager = AddressManager::load_from_file(&path)?;

    // Get list of all JSON files in the directory
    let mut entries = fs::read_dir(&transactions_folder).await?;

    // let mut tasks = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            println!("Open file {}", path.to_string_lossy());
            // Load and parse JSON file
            let content = fs::read_to_string(&path).await?;
            let transaction: TransactionRequest = serde_json::from_str(&content)?;
            // println!("{:?}", transaction);

            let from_address = transaction.from.expect("Sender address not specified");
            let index = address_manager.get_index_with_converted_address(&from_address).await
                .expect("Cannot retrieve mnemonic index of transaction sender address");
            let wallet = tester::create_wallet(mnemonic, index as u32).await?;
            let tx_hash = tester::send_transaction(&url, &wallet, transaction).await?;

            use tokio::time::{sleep, Duration};
            sleep(Duration::from_millis(100)).await;

            let balance = tester::get_balance(url, &from_address, None).await?;
            let tx = tester::get_transaction_by_hash(url, tx_hash).await?.unwrap();
            println!("Balance of {}: {}", hex::encode(from_address), balance);
            println!("{:?}", tx);

            // let task = tokio::spawn(async move {
            //     match load_and_parse_json(&path).await {
            //         Ok(transaction) => {
            //             println!("{:?}", transaction);
            //             let wallet = tester::create_wallet(mnemonic, index).await?;
            //             let _ = tester::send_transaction(&URL, &wallet, &transaction).await;
            //         }
            //         Err(e) => {
            //             eprintln!("Failed to load and parse {}: {}", path.display(), e);
            //         }
            //     }
            // });
            // tasks.push(task);
        }
    }

    // Wait for all tasks to complete
    // let _ = futures::future::join_all(tasks).await;

    Ok(())
}
