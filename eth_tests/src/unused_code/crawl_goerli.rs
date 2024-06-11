use ethers::core::k256::elliptic_curve::rand_core::block;
use serde::Deserialize;
use std::{env, fs::File};
use tokio;
use dotenv::dotenv;
use reqwest::Client;
use log::{info, LevelFilter};
use fern::Dispatch;

const LIMIT_RECORDS :u32 = 10000;

#[derive(Deserialize, Debug)]
struct EtherscanResponse {
    status: String,
    message: String,
    result: Vec<Transaction>,
}

#[derive(Deserialize, Debug)]
struct Transaction {
    blockNumber: String,
    timeStamp: String,
    hash: String,
    nonce: Option<String>,
    blockHash: Option<String>,
    transactionIndex: Option<String>,
    from: String,
    to: String,
    value: String,
    gas: String,
    gasPrice: Option<String>,
    isError: String,
    txreceipt_status: Option<String>,
    input: String,
    contractAddress: String,
    cumulativeGasUsed: Option<String>,
    gasUsed: String,
    confirmations: Option<String>,
}

async fn crawl_transactions(start_block: u64, end_block: u64, verbal: bool) -> Result<(), Box<dyn std::error::Error>> {
    let etherscan_api_key = env::var("ETHERSCAN_API_KEY")?;
    let output_dir = env::var("GOERLI_TRANSACTIONS_OUTPUT_DIR")?;    
    let client = Client::new();
    let step = env::var("CRAWLED_STEP").unwrap_or_default();
    let step: usize = step.parse().unwrap();
    let etherscan_offset = LIMIT_RECORDS; // maximum offset
    let mut total_txs: u64 = 0;

    for block_number in (start_block..=end_block).step_by(step) {
        // Make a request to the Etherscan API
        let mut endstep_block_number = block_number + step as u64 - 1;
        if endstep_block_number > end_block {
            endstep_block_number = end_block;
        }
        let url = format!(
            "https://api.etherscan.io/api?module=account&action=txlistinternal&startblock={}&endblock={}&page=1&offset={}&sort=asc&apikey={}",
            block_number, endstep_block_number, etherscan_offset, etherscan_api_key
        );

        // Send the request and parse the response
        let response = client.get(&url).send().await?;
        let string_response = response.text().await?;

        let etherscan_response: EtherscanResponse = serde_json::from_str(&string_response)?;
        assert!(etherscan_response.result.len() < LIMIT_RECORDS as usize);

        let parsed_json: serde_json::Value = serde_json::from_str(&string_response).expect("Invalid JSON");

        // Serialize the JSON with pretty formatting
        let pretty_json = serde_json::to_string_pretty(&parsed_json).expect("Failed to serialize JSON");

        total_txs += etherscan_response.result.len() as u64;
        if verbal {
            // Check the response status
            if etherscan_response.status == "1" {
                info!("{} transactions from block {} to block {}", etherscan_response.result.len(), block_number, endstep_block_number);
            } else {
                info!("No transactions found from block {} to block {}", block_number, endstep_block_number);
            }
            info!("Total transactions = {}", total_txs);
        }
        let output_file = format!("{}/txs_{}_{}.json", output_dir, block_number, endstep_block_number);

        tokio::fs::write(&output_file, &pretty_json).await?;
        info!("Transactions from block {} to block {} saved to file {}", block_number, endstep_block_number, output_file);
    }
    Ok(())
}

async fn bin_crawl_transactions(start_block: u64, end_block: u64, verbal: bool) -> Result<(), Box<dyn std::error::Error>> {
    let etherscan_api_key = env::var("ETHERSCAN_API_KEY")?;
    let output_dir = env::var("GOERLI_TRANSACTIONS_OUTPUT_DIR")?;    
    let client = Client::new();
    let etherscan_offset = LIMIT_RECORDS; // maximum offset
    let step = end_block + 1;

    for block_number in (start_block..=end_block).step_by(step as usize) {
        // Make a request to the Etherscan API
        let mut endstep_block_number = block_number + step - 1;
        if endstep_block_number > end_block {
            endstep_block_number = end_block;
        }
        let url = format!(
            "https://api.etherscan.io/api?module=account&action=txlistinternal&startblock={}&endblock={}&page=1&offset={}&sort=asc&apikey={}",
            block_number, endstep_block_number, etherscan_offset, etherscan_api_key
        );

        // Send the request and parse the response
        let response = client.get(&url).send().await?;
        let string_response = response.text().await?;
        let etherscan_response: EtherscanResponse = serde_json::from_str(&string_response)?;
        // Check the response status

        let parsed_json: serde_json::Value = serde_json::from_str(&string_response).expect("Invalid JSON");

        // Serialize the JSON with pretty formatting
        let pretty_json = serde_json::to_string_pretty(&parsed_json).expect("Failed to serialize JSON");

        let output_file = format!("{}/txs_{}_{}.json", output_dir, block_number, endstep_block_number);
        tokio::fs::write(&output_file, &pretty_json).await?;
        info!("Transactions from block {} to block {} saved to file {}", block_number, endstep_block_number, output_file);

        if verbal {
            
            if etherscan_response.status == "1" {
                info!("{} transactions from block {} to block {}", etherscan_response.result.len(), block_number, endstep_block_number);
            } else {
                info!("No transactions found from block {} to block {}", block_number, endstep_block_number);
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // Initialize logging
    let output_dir = env::var("GOERLI_TRANSACTIONS_OUTPUT_DIR")?;    
    if !std::fs::metadata(&output_dir).is_ok() {
        std::fs::create_dir(&output_dir)?;
    }

    let log_dir = env::var("LOG_DIR")?;
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
    let start_block: u64 = env::var("CRAWLED_START_BLOCK")?.parse().unwrap(); // init start block
    let end_block: u64 = env::var("CRAWLED_END_BLOCK")?.parse().unwrap(); // init end block
    crawl_transactions(start_block, end_block, true).await?;
    // bin_crawl_transactions(start_block, end_block, true).await?;
    Ok(())
}