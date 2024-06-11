use std::{env, fs};

use serde_json;
use tokio::time::sleep;
use tokio::fs::File;
use web3::types::{TransactionId, TransactionRequest};
use web3::transports::Http;
use web3::Web3;

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref LOCAL_URL: String = env::var("LOCAL_URL").expect("LOCAL_URL must be set");
    static ref CONVERTED_TRANSACTIONS_OUTPUT_DIR: String =
        env::var("CONVERTED_TRANSACTIONS_OUTPUT_DIR").expect("CONVERTED_TRANSACTIONS_OUTPUT_DIR must be set");
    static ref MNEMONIC: String = env::var("MNEMONIC").expect("MNEMONIC must be set");
    static ref CHAIN_ID: u64 = env::var("CHAIN_ID").expect("CHAIN_ID must be set")
        .parse().expect("CHAIN_ID must be integer");
}

async fn verify_send_transaction_from_path(
    path: &str,
    web3: &Web3<Http>,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let transaction: TransactionRequest = serde_json::from_str(&content)?;
    println!("{:?}", transaction);

    let block_number = web3.eth().block_number().await?;
    println!("Current block number: {}", block_number);

    let from_address = transaction.from;
    let balance0 = web3.eth().balance(from_address, None).await?;
    println!("From address: {}", from_address);
    println!("Balance before transaction: {}", balance0);
    let tx_hash = web3.eth().send_transaction(transaction).await?;
    println!("Transaction sent. Hash: {:?}", tx_hash);
    let tx = web3.eth().transaction(TransactionId::Hash(tx_hash)).await?;
    println!("{:?}", tx);
    loop {
        match web3.eth().transaction_receipt(tx_hash).await? {
            Some(receipt) => {
                println!("Transaction receipt: {:?}", receipt);
                break;
            }
            None => {
                println!("Transaction not found");
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }
    let balance = web3.eth().balance(from_address, None).await?;
    println!("Balance after transaction: {}", balance);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load env variables
    let env_path = ".env.verify_transaction_reth";
    dotenv::from_path(env_path).ok();

    let transactions_folder = format!("{}/transactions", *CONVERTED_TRANSACTIONS_OUTPUT_DIR);
    let url = (*LOCAL_URL).as_str();
    let http = Http::new(url)?;
    let web3 = Web3::new(http);

    // let path = format!("{}/20000536_3.json", transactions_folder);
    let path = "test_tx.json";
    let _ = verify_send_transaction_from_path(&path, &web3).await?;
    Ok(())
}
