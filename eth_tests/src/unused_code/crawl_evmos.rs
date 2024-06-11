use reqwest;
use serde_json::json;
use dotenv::dotenv;
use log::{info, LevelFilter};
use fern::Dispatch;
use std::{env, fs::File, os::macos::raw};
use futures::future::join_all;
use ethers::prelude::*;
use std::{convert::{TryFrom, TryInto}, str::FromStr};
use ethers::providers::{Http, Middleware, Provider};
use ethers::types::{Transaction, H256, TransactionRequest, U256, NameOrAddress};
use ethers::utils::rlp::RlpStream;
use hex;


async fn recover_raw_transaction(node_url: &str, tx_hash: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Parse the tx_hash string into H256
    let tx_hash: H256 = tx_hash.parse()?;

    // Create the reqwest client
    let client = reqwest::Client::new();
    
    // JSON-RPC payload
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_getTransactionByHash",
        "params": [tx_hash],
        "id": 1,
    });

    // Fetch the transaction using JSON-RPC
    let mut tx = Transaction::default();
    for _ in 0..5 {
        let response = client.post(node_url)
            .json(&payload)
            .send()
            .await?;

        if let Ok(result) = response.json::<serde_json::Value>().await {
            if let Some(tx_response) = result["result"].as_object() {
                tx = serde_json::from_value(serde_json::Value::Object(tx_response.clone()))?;
                break;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    // info!("tx: {:?}", tx);

    // Construct a TransactionRequest from the Transaction
    let tx_request = TransactionRequest {
        from: Some(tx.from),
        to: tx.to.map(NameOrAddress::Address),
        gas: Some(tx.gas),
        gas_price: tx.gas_price,
        value: Some(tx.value),
        data: Some(tx.input.clone()),
        nonce: Some(tx.nonce),
        ..Default::default()
    };

    // Serialize the transaction
    let mut rlp_stream = RlpStream::new();
    rlp_stream.begin_list(9);
    rlp_stream.append(&tx_request.nonce.unwrap_or_default());
    rlp_stream.append(&tx_request.gas_price.unwrap_or_default());
    rlp_stream.append(&tx_request.gas.unwrap_or_default());
    if let Some(to) = tx_request.to {
        rlp_stream.append(&to);
    } else {
        rlp_stream.append_empty_data();
    }
    rlp_stream.append(&tx_request.value.unwrap_or_default());
    rlp_stream.append(&tx_request.data.unwrap_or_default().as_ref());
    rlp_stream.append(&tx.chain_id.unwrap_or_default()); // Chain ID
    rlp_stream.append(&tx.v); // v
    rlp_stream.append(&tx.r); // r
    rlp_stream.append(&tx.s); // s

    let raw_tx = rlp_stream.out().to_vec();
    Ok(raw_tx)
    // Ok(format!("0x{}", hex::encode(raw_tx)))
}

async fn get_block_transactions(url: &str, block_number: u64, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    let response = client.post(url)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "eth_getBlockByNumber",
            "params": [U256::from(block_number), true],
            "id": block_number
        }))
        .send()
        .await?;

    let parsed_json = response.json::<serde_json::Value>().await?;

    // Extract the "transactions" field if it exists
    if let Some(transactions) = parsed_json["result"]["transactions"].as_array() {
        info!("{} transactions found in block {}", transactions.len(), block_number);
        for (index, transaction) in transactions.iter().enumerate() {
            let pretty_json = serde_json::to_string_pretty(&transaction).expect("Failed to serialize JSON");
            let output_file = format!("{}/{}_{}.json", output_dir, block_number, index);
            tokio::fs::write(&output_file, &pretty_json).await?;
            info!("Transactions evmos mainnet of block {} saved to file {}", block_number, output_file);
        }
    } else {
        // Log a warning if the "transactions" field is not found
        log::warn!("No transactions found in block {}", block_number);
    }
    Ok(())
}

async fn get_block_raw_transactions(url: &str, block_number: u64, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let response = client.post(url)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "eth_getBlockByNumber",
            "params": [U256::from(block_number), false],
            "id": block_number
        }))
        .send()
        .await?;

    let parsed_json = response.json::<serde_json::Value>().await?;

    // Extract the "transactions" field if it exists
    if let Some(transactions) = parsed_json["result"]["transactions"].as_array() {
        info!("{} transactions found in block {}", transactions.len(), block_number);
        for (index, tx_hash) in transactions.iter().enumerate() {
            let tx_hash_str = tx_hash.as_str().ok_or("Invalid tx_hash format")?.trim_matches('"');
            // info!("tx_hash: {}", tx_hash_str);
            let raw_tx = recover_raw_transaction(url, tx_hash_str).await?;
            let output_file = format!("{}/{}_{}_bin.bin", output_dir, block_number, index);
            tokio::fs::write(&output_file, &raw_tx).await?;
            let raw_tx = format!("0x{}", hex::encode(raw_tx));
            // info!("Raw_tx {}", raw_tx);
            let output_file = format!("{}/{}_{}_raw.txt", output_dir, block_number, index);
            tokio::fs::write(&output_file, &raw_tx).await?;
            info!("Raw transactions of block {} saved to file {}", block_number, output_file);
            
            // match recover_raw_transaction(url, tx_hash_str).await {
            //     Ok(raw_tx) => {
            //         info!("Raw_tx {}", raw_tx);
            //         let output_file = format!("{}/{}_{}_raw.json", output_dir, block_number, index);
            //         tokio::fs::write(&output_file, &raw_tx).await?;
            //         info!("Raw transactions of block {} saved to file {}", block_number, output_file);
            //     },
            //     Err(e) => {
            //         log::error!("Error recovering raw transaction for hash {}: {}", tx_hash_str, e);
            //     }
            // }
        }
    } else {
        // Log a warning if the "transactions" field is not found
        log::warn!("No transactions found in block {}", block_number);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // Initialize logging
    let output_dir = env::var("EVMOS_TESTNET_TRANSACTIONS_OUTPUT_DIR")?;    
    if !std::fs::metadata(&output_dir).is_ok() {
        std::fs::create_dir(&output_dir)?;
    }

    let log_dir = env::var("EVMOS_TESTNET_LOG_DIR")?;
    let log_file = File::create(log_dir)?;
    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!("[{}] {}", record.level(), message))
        })
        .level(LevelFilter::Info)
        .chain(log_file)
        .chain(std::io::stdout())
        .apply()?;

    // Specify the block range to crawl
    let start_block: u64 = env::var("EVMOS_TESTNET_START_BLOCK")?.parse().unwrap(); // init start block
    let end_block: u64 = env::var("EVMOS_TESTNET_END_BLOCK")?.parse().unwrap(); // init end block

    let url = format!("https://evmos.drpc.org");

    // let client = reqwest::Client::new();
    // let response = client.post(url.clone())
    //     .json(&json!({
    //         "jsonrpc": "2.0",
    //         "method": "eth_getBlockTransactionCountByNumber",
    //         "params": ["20971861"],
    //         "id": 1
    //     }))
    //     .send()
    //     .await?;
    // let parsed_json = response.json::<serde_json::Value>().await?;
    // println!("{:?}", parsed_json);
    
    let mut tasks:Vec<tokio::task::JoinHandle<()>> = Vec::new();
    for block_number in start_block..=end_block{
        println!("Begin block {}", block_number);
        let url_clone = url.clone();
        let output_dir_clone = output_dir.clone();
        // get_block_transactions(&url_clone, block_number, &output_dir_clone).await;
        let task = tokio::spawn(async move {
            let _ = get_block_transactions(&url_clone, block_number, &output_dir_clone).await;
            // let _ = get_block_raw_transactions(&url_clone, block_number, &output_dir_clone).await;
        });
        tasks.push(task);
    }
    let result = join_all(tasks).await;
    Ok(())
}