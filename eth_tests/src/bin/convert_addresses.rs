use std::{
    fs::File, io::Read,
};
use ethers::{core::types::Transaction, prelude::*};
use std::env;

use eth_tests::address_manager::AddressManager;

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref OUTPUT_DIR: String = env::var("OUTPUT_DIR").expect("OUTPUT_DIR must be set");
    static ref CONVERTED_TRANSACTIONS_OUTPUT_DIR: String = env::var("CONVERTED_TRANSACTIONS_OUTPUT_DIR").expect("CONVERTED_TRANSACTIONS_OUTPUT_DIR must be set");
    static ref MNEMONIC: String = env::var("MNEMONIC").expect("MNEMONIC must be set");
    static ref CHAIN_ID: u64 = env::var("CHAIN_ID").expect("CHAIN_ID must be set").parse().expect("CHAIN_ID must be integer");
}

async fn generate_new_transaction_request(
    crawled_transaction: &Transaction,
    from: H160,
    to: Option<H160>,
    // chain_id: u64,
) -> Result<TransactionRequest, Box<dyn std::error::Error>> {
    // Your logic to generate the new transaction request here
    // This could involve extracting relevant fields from the crawled_transaction object
    // and constructing a TransactionRequest struct
    // For demonstration, let's assume a simple implementation

    // needing modify parameters: from, to, nonce, chain_id
    
    // nonce and chain_id manually config at calling time

    let new_transaction_request = TransactionRequest {
        from: Some(from),
        to: to.map(|x| NameOrAddress::Address(x)),
        gas: Some(crawled_transaction.gas),
        gas_price: crawled_transaction.gas_price,
        value: Some(crawled_transaction.value),
        data: Some(crawled_transaction.input.clone()),
        // nonce: get_transaction_count(from.parse()?).await?,
        // chain_id: Some(U64([*CHAIN_ID])),
        ..Default::default()
    };
    Ok(new_transaction_request)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let env_path = ".env.convert_addresses";
    dotenv::from_path(env_path).ok();

    remove_and_recreate_output_directory(&*&CONVERTED_TRANSACTIONS_OUTPUT_DIR).await?;

    let mut address_manager = AddressManager::new(&MNEMONIC);

    let folder_path = format!("{}/transactions", &*OUTPUT_DIR);
    let dir = std::fs::read_dir(folder_path)?;

    // Collect all entries in the directory
    let entries: Vec<_> = dir.collect::<Result<_, _>>()?;

    // Filter out directories and count only files
    let file_count = entries.iter().filter(|entry| entry.path().is_file()).count();
    println!("Number of files: {}", file_count);

    for file_entry in entries {
        let file_entry = file_entry;
        let file_path = file_entry.path();

        if let Some(file_name) = file_path.file_name() {
            let file_name = file_name.to_string_lossy();
            println!("File name: {:?}", file_name);
            if file_name.ends_with(".json") {
                // Read the JSON file
                let mut file = File::open(&file_path)?;
                let mut content = String::new();
                file.read_to_string(&mut content)?;

                // Parse JSON content into transaction structure
                let transaction: Transaction = serde_json::from_str(&content)?;

                // Modify the "from" and "to" fields using the new addresses
                let from_address = address_manager.get_map_address(&transaction.from).expect("None from address");
                let to_address = match &transaction.to {
                    Some(to) => address_manager.get_map_address(to),
                    None => None,
                };

                let new_transaction = generate_new_transaction_request(&transaction, from_address, to_address).await?;

                // Write the modified transaction structure back into the JSON file
                let pretty_json = serde_json::to_string_pretty(&new_transaction).expect("Failed to serialize JSON");
                println!("New tx: {}", pretty_json);
                let output_file = format!("{}/transactions/{}", &*CONVERTED_TRANSACTIONS_OUTPUT_DIR, file_name);
                tokio::fs::write(&output_file, &pretty_json).await?;
            }
        }
    }

    let path = format!("{}/address_manager.json", *CONVERTED_TRANSACTIONS_OUTPUT_DIR);
    address_manager.save_to_file(&path)?;
    address_manager.info();
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
