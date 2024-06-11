use serde_json::json;
use log::{info, LevelFilter};
use fern::Dispatch;
use std::env;
use futures::future::join_all;
use ethers::{core::k256::elliptic_curve::rand_core::block, prelude::*};
use std::{convert::{TryFrom, TryInto}, str::FromStr};
use ethers::providers::{Http, Middleware, Provider};
use ethers::types::{Transaction, H256, TransactionRequest, U256, NameOrAddress};

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref OUTPUT_DIR: String = env::var("OUTPUT_DIR").expect("OUTPUT_DIR must be set");
    static ref LOG_DIR: String = env::var("LOG_DIR").expect("LOG_DIR must be set");
    static ref START_BLOCK: u64 = env::var("START_BLOCK").expect("START_BLOCK must be set").parse().expect("Invalid START_BLOCK");
    static ref END_BLOCK: u64 = env::var("END_BLOCK").expect("END_BLOCK must be set").parse().expect("Invalid END_BLOCK");
    static ref URL: String = env::var("URL").expect("URL must be set");
}

async fn get_block_transactions(block_number: u64) 
    -> Result<(), Box<dyn std::error::Error>> 
{
    let provider = Provider::try_from(&*URL).expect("Cannot connect URL");
    let response = provider.get_block_with_txs(block_number).await?;
    
    match response {
        Some(block) => {
            info!("{} transactions in block {}", block.transactions.len(), block_number);
            for (index, transaction) in block.transactions.iter().enumerate() {
                let pretty_json = serde_json::to_string_pretty(&transaction).expect("Failed to serialize JSON");
                let output_file = format!("{}/transactions/{}_{}.json", &*OUTPUT_DIR, block_number, index);
                tokio::fs::write(&output_file, &pretty_json).await?;
            }
        }
        None => {
            info!("No transaction in block {}", block_number);
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load env variables
    let env_path = ".env.crawl_evmos_mainnet";
    dotenv::from_path(env_path).ok();

    // Initialize output directory  

    
    // // Remove all data and recreate directory
    // remove_and_recreate_output_directory(&*OUTPUT_DIR).await?;

    // Create directory if not exist
    if !std::fs::metadata(&*OUTPUT_DIR).is_ok() {
        std::fs::create_dir(&*OUTPUT_DIR)?;
    }

    // Initialize logging
    let log_file = std::fs::File::create(&*LOG_DIR)?;
    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!("[{}] {}", record.level(), message))
        })
        .level(LevelFilter::Info)
        .chain(log_file)
        .chain(std::io::stdout())
        .apply()?;

    // Specify the block range to crawl
    let start_block: u64 = *START_BLOCK; // init start block
    let end_block: u64 = *END_BLOCK; // init end block
    
    // Define tasks vector for async handling

    for block_number in start_block..=end_block{
        let mut tasks:Vec<tokio::task::JoinHandle<()>> = Vec::new();

        println!("Begin block {}", block_number);
        let task = tokio::spawn(async move {
            let _ = get_block_transactions(block_number).await;
        });
        tasks.push(task);
        if (block_number - start_block + 1) % 1000 == 0 || block_number == end_block {
            let result = join_all(tasks).await;
            println!("{:?}", result);
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }

    // Ensure all tasks finished
    Ok(())
}

async fn remove_and_recreate_output_directory(dir_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Remove the directory if it exists
    if std::fs::metadata(dir_path).is_ok() {
        std::fs::remove_dir_all(dir_path)?;
    }

    // Recreate the directory
    std::fs::create_dir(dir_path)?;

    // Create the 'transactions' directory inside 'outputdir'
    let transactions_dir_path = format!("{}/transactions", dir_path);
    std::fs::create_dir(&transactions_dir_path)?;
    Ok(())
}