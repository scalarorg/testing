use tokio::fs;
use serde_json;
use web3::types::{BlockNumber, Bytes, Transaction, TransactionParameters, TransactionRequest, H256};
use web3::transports::Http;
use web3::Web3;
use std::{env, path::PathBuf};
use eth_tests::{address_manager::{self, AddressManager}, tester::Tester};
use tokio::time::Instant;
use futures::stream::{self, StreamExt};

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref LOCAL_URL: String = env::var("LOCAL_URL").expect("LOCAL_URL must be set");
    static ref CONVERTED_TRANSACTIONS_OUTPUT_DIR: String = env::var("CONVERTED_TRANSACTIONS_OUTPUT_DIR").expect("CONVERTED_TRANSACTIONS_OUTPUT_DIR must be set");
    static ref MNEMONIC: String = env::var("MNEMONIC").expect("MNEMONIC must be set");
    static ref CHAIN_ID: u64 = env::var("CHAIN_ID").expect("CHAIN_ID must be set").parse().expect("CHAIN_ID must be integer");
}

async fn send_transaction_from_path_by_web3(path: &PathBuf, tester: &Tester, address_manager: &AddressManager) -> Result<H256, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path).await?;
    let transaction: TransactionRequest = serde_json::from_str(&content)?;
    // println!("{:?}", transaction);
    let from_address = transaction.from;
    let priv_key = address_manager.get_private_key(&from_address).expect("Cannot retrieve private key");
    let priv_key = hex::decode(priv_key)?;
    let key = web3::signing::SecretKey::from_slice(&priv_key)?;

    // let tx_hash = tester.send_transaction_with_private_key(transaction, key, *CHAIN_ID).await?;

    let signed_raw_transaction = tester.make_signed_raw_transaction(transaction, key, *CHAIN_ID).await?;
    let web3 = tester.web3.clone();
    let tx_hash = tokio::task::spawn(async move {
        let tx_hash = web3.eth().send_raw_transaction(signed_raw_transaction).await.unwrap();
        tx_hash
    });

    // Wait for transaction receipt
    // loop {
    //     match tester.get_transaction_receipt(tx_hash).await? {
    //         Some(receipt) => {
    //             println!("Transaction receipt: {:?}", receipt);
    //             break;
    //         }
    //         None => {
    //             println!("Waiting for transaction to be mined...");
    //             tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    //         }
    //     }
    // }

    // println!("Transaction sent. Hash: {:?}", tx_hash);
    Ok(tx_hash.await?)
    // Ok(tx_hash)
}

async fn make_transaction_from_path_by_web3(path: &PathBuf, tester: &Tester, address_manager: &AddressManager) -> Result<Bytes, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path).await?;
    let transaction: TransactionRequest = serde_json::from_str(&content)?;
    // println!("{:?}", transaction);
    let from_address = transaction.from;
    let priv_key = address_manager.get_private_key(&from_address).expect("Cannot retrieve private key");
    let priv_key = hex::decode(priv_key)?;
    let key = web3::signing::SecretKey::from_slice(&priv_key)?;

    let signed_raw_transaction = tester.make_signed_raw_transaction(transaction, key, *CHAIN_ID).await?;
    Ok(signed_raw_transaction)
}

// Send signed raw transaction with Web3<Http>
async fn send_signed_raw_transaction(web3: Web3<Http>, signed_raw_transaction: Bytes) -> Result<H256, Box<dyn std::error::Error>> {
    let tx_hash = web3.eth().send_raw_transaction(signed_raw_transaction).await.unwrap();
    Ok(tx_hash)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Load env variables
    let env_path = ".env.send_transactions";
    dotenv::from_path(env_path).ok();
    
    let transactions_folder = format!("{}/transactions", *CONVERTED_TRANSACTIONS_OUTPUT_DIR);
    let url = (*LOCAL_URL).as_str();

    let tester = Tester::new(url).await?;
    let address_manager_path = format!("{}/address_manager.json", *CONVERTED_TRANSACTIONS_OUTPUT_DIR);
    let address_manager = AddressManager::load_from_file(&address_manager_path)?;

    // let path = PathBuf::from("../generated_transactions_reth/transactions/num_2.json");
    // send_transaction_from_path_with_private_key(&path, &tester, &address_manager).await?;
    // return Ok(());

    // Get list of all JSON files in the directory
    let mut entries = fs::read_dir(&transactions_folder).await?;

    let mut paths: Vec<PathBuf> = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            paths.push(path);
        }
    }

    let mut count = 0;
    let mut start_all = Instant::now();
    let mut start = Instant::now();

    let mut tx_hashes = Vec::new();

    for path in paths {
        count += 1;
        // println!("Sending transaction {}: {}", count, path.to_string_lossy());
        let tx_hash = send_transaction_from_path_by_web3(&path, &tester, &address_manager).await?;

        tx_hashes.push(tx_hash);

        // send_transaction_from_path_without_private_key(&path, &tester).await?;

        if count % 100 == 0 {
            let duration = start.elapsed();
            let tps = 100.0 / duration.as_secs_f64();
            println!("Sent {} transactions in {:?}. TPS: {:.2}", count, duration, tps);
            start = Instant::now();
        }
    }

    let duration = start_all.elapsed();
    let tps = (count) as f64 / duration.as_secs_f64();
    println!("Sent {} transactions in {:?}. TPS: {:.2}", count, duration, tps);

    let mut tasks = Vec::new();

    // Comfirm all receipt from tx_hashes not None
    for tx_hash in tx_hashes {
        let web3 = tester.web3.clone();
        let task = tokio::spawn(async move {
            loop {
                match web3.eth().transaction_receipt(tx_hash).await {
                    Ok(Some(receipt)) => {
                        // println!("Transaction receipt: {:?}", receipt);
                        // println!("Received transaction: {:?}", tx_hash);
                        break;
                    }
                    _ => {
                        // println!("Waiting for transaction {} to be mined...", tx_hash);
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    }
                }
            }
        });
        tasks.push(task);
    }
    futures::future::join_all(tasks).await;

    let duration = start_all.elapsed();
    let tps = count as f64 / duration.as_secs_f64();
    println!("Get {} transactions receipt in {:?}. TPS: {:.2}", count, duration, tps);

    Ok(())
}
