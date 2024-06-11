use tokio::fs;
use serde_json;
use web3::types::{BlockNumber, Bytes, TransactionRequest, H256};
use web3::transports::Http;
use web3::Web3;
use core::num;
use std::error::Error;
use std::{env, path::PathBuf};
use eth_tests::{address_manager::{self, AddressManager}, tester::Tester};
use tokio::time::{Instant, Duration};
use futures::stream::{self, StreamExt};

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref LOCAL_URL: String = env::var("LOCAL_URL").expect("LOCAL_URL must be set");
    static ref CONVERTED_TRANSACTIONS_OUTPUT_DIR: String = env::var("CONVERTED_TRANSACTIONS_OUTPUT_DIR").expect("CONVERTED_TRANSACTIONS_OUTPUT_DIR must be set");
    static ref MNEMONIC: String = env::var("MNEMONIC").expect("MNEMONIC must be set");
    static ref CHAIN_ID: u64 = env::var("CHAIN_ID").expect("CHAIN_ID must be set").parse().expect("CHAIN_ID must be integer");
}

async fn send_transaction_from_path_by_web3(path: &PathBuf, tester: &Tester, address_manager: &AddressManager) -> Result<H256, Box<dyn Error>> {
    let content = fs::read_to_string(path).await?;
    let transaction: TransactionRequest = serde_json::from_str(&content)?;
    let from_address = transaction.from;
    let priv_key = address_manager.get_private_key(&from_address).expect("Cannot retrieve private key");
    let priv_key = hex::decode(priv_key)?;
    let key = web3::signing::SecretKey::from_slice(&priv_key)?;

    let tx_hash = tester.send_transaction_with_private_key(transaction, key, *CHAIN_ID).await?;
    Ok(tx_hash)
}

async fn make_transaction_from_path_by_web3(path: &PathBuf, tester: &Tester, address_manager: &AddressManager) -> Result<Bytes, Box<dyn Error>> {
    let content = fs::read_to_string(path).await?;
    let transaction: TransactionRequest = serde_json::from_str(&content)?;
    let from_address = transaction.from;
    let priv_key = address_manager.get_private_key(&from_address).expect("Cannot retrieve private key");
    let priv_key = hex::decode(priv_key)?;
    let key = web3::signing::SecretKey::from_slice(&priv_key)?;

    let signed_raw_transaction = tester.make_signed_raw_transaction(transaction, key, *CHAIN_ID).await?;
    Ok(signed_raw_transaction)
}

async fn send_signed_raw_transaction(web3: Web3<Http>, signed_raw_transaction: Bytes) -> Result<H256, Box<dyn Error>> {
    loop {
        match web3.eth().send_raw_transaction(signed_raw_transaction.clone()).await {
            Ok(tx_hash) => return Ok(tx_hash),
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let env_path = ".env.send_transactions";
    dotenv::from_path(env_path).ok();
    
    let transactions_folder = format!("{}/transactions", *CONVERTED_TRANSACTIONS_OUTPUT_DIR);
    let url = (*LOCAL_URL).as_str();

    let tester = Tester::new(url).await?;
    let address_manager_path = format!("{}/address_manager.json", *CONVERTED_TRANSACTIONS_OUTPUT_DIR);
    let address_manager = AddressManager::load_from_file(&address_manager_path)?;

    let mut entries = fs::read_dir(&transactions_folder).await?;

    let mut paths: Vec<PathBuf> = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            paths.push(path);
        }
    }

    let mut start = Instant::now();
    let mut signed_raw_transaction_vec = Vec::new();

    let mut count = 0;
    for path in paths {
        let signed_raw_transaction = make_transaction_from_path_by_web3(&path, &tester, &address_manager).await?;
        signed_raw_transaction_vec.push(signed_raw_transaction);
        count += 1;
        if count % 1000 == 0 {
            println!("Made {} transactions", count);
        }
    }

    let num_of_transactions = signed_raw_transaction_vec.len();
    let duration = start.elapsed();
    let tps = num_of_transactions as f64 / duration.as_secs_f64();
    println!("Made {} signed raw transactions in {:?}, TPS={:.2}", num_of_transactions, duration, tps);

    let vec_file_name = "vec.json";
    save_vec_to_file(&signed_raw_transaction_vec, vec_file_name).await?;

    // start = Instant::now();
    // let mut tx_hashes_tasks = Vec::new();
    // let mut tx_hashes_vec = Vec::new();

    // let mut start_100 = Instant::now();
    // for (i, signed_raw_transaction) in signed_raw_transaction_vec.iter().enumerate() {
    //     let web3 = tester.web3.clone();
    //     let signed_raw_transaction = signed_raw_transaction.clone();
    //     let tx_hash = tokio::task::spawn(async move {
    //         send_signed_raw_transaction(web3, signed_raw_transaction).await.unwrap()
    //     });
    //     tx_hashes_tasks.push(tx_hash);

    //     if (i + 1) % 100 == 0 || i >= (num_of_transactions - 1) {
    //         let tx_hashes = futures::future::join_all(tx_hashes_tasks).await;
    //         tx_hashes_vec.extend(tx_hashes);
    //         let duration = start_100.elapsed();
    //         println!("Sent {} transactions in {:?}", i + 1, duration);
    //         start_100 = Instant::now();
    //         tx_hashes_tasks = Vec::new();
    //     }
    // }

    // let duration = start.elapsed();
    // let tps = num_of_transactions as f64 / duration.as_secs_f64();
    // println!("Sent {} transactions in {:?}, TPS={:.2}", num_of_transactions, duration, tps);

    // // start = Instant::now();

    // let mut tasks = Vec::new();
    // for tx_hash in tx_hashes_vec {
    //     let tx_hash = tx_hash?;
    //     let web3 = tester.web3.clone();

    //     let task = tokio::spawn(async move {
    //         loop {
    //             match web3.eth().transaction_receipt(tx_hash).await {
    //                 Ok(Some(_receipt)) => break,
    //                 _ => {
    //                     tokio::time::sleep(Duration::from_millis(10)).await;
    //                 }
    //             }
    //         }
    //     });
    //     tasks.push(task);
    // }

    // futures::future::join_all(tasks).await;

    // let duration = start.elapsed();
    // let tps = num_of_transactions as f64 / duration.as_secs_f64();
    // println!("Get {} transactions receipt in {:?}. TPS: {:.2}", num_of_transactions, duration, tps);

    Ok(())
}

// save vec of Bytes to file
async fn save_vec_to_file(vec: &Vec<Bytes>, file_name: &str) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string_pretty(&vec)?;
    tokio::fs::write(file_name, json).await?;
    Ok(())
}

// load vec of Bytes
async fn load_vec_from_file(file_name: &str) -> Result<Vec<Bytes>, Box<dyn Error>> {
    let json = tokio::fs::read_to_string(file_name).await?;
    let vec: Vec<Bytes> = serde_json::from_str(&json)?;
    Ok(vec)
}