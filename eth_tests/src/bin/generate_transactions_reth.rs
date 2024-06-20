use eth_tests::address_manager::{self, AddressManager};
use ethers::etherscan::gas;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::Serialize;
use std::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use ethers::types::U256;

#[derive(Serialize)]
struct TransactionRequest {
    to: String,
    from: String,
    value: U256,
    gas: U256,
    gasPrice: U256,
    // data: String,
}

// Random an usize in range [L, R]
fn random_usize(rng: &mut rand::rngs::ThreadRng, L: usize, R: usize) -> usize {
    L + rng.gen::<usize>() % (R - L + 1)
}

#[tokio::main]
async fn main() {
    let num_of_transactions = 100;
    let output_dir = "../../generated_transactions_reth";

    remove_and_recreate_output_directory(output_dir).await.unwrap();

    let address_manager_path = "../../converted_evmos_mainnet_transactions_03/address_manager.json";
    let address_manager = AddressManager::load_from_file(address_manager_path).unwrap();
    let (_, addresses_size) = address_manager.info();

    let mut rng = rand::thread_rng();

    for i in 0..num_of_transactions {
        // let to = address_manager.get_converted_address_with_index(random_usize(&mut rng, 0usize, addresses_size-1));
        // let from = address_manager.get_converted_address_with_index(random_usize(&mut rng, 0usize, addresses_size-1));
        let to = address_manager.get_converted_address_with_index(i%addresses_size);
        let from = address_manager.get_converted_address_with_index(i%addresses_size);
        let value = U256::from(random_usize(&mut rng, 1, 100));
        let gas = U256::from(random_usize(&mut rng, 5000000, 5100000));
        let gasPrice = U256::from(random_usize(&mut rng, 0, 10000)) + 6675204846300u64;
        // let value = U256::from(5);
        // let gas = U256::from(21000);
        // let gasPrice = U256::from(1000000000);
        // let data = format!("0x{}", hex::encode(&rng.gen::<[u8; 32]>()));

        let transaction_request = TransactionRequest {
            to,
            from,
            value,
            gas,
            gasPrice,
            // data,
        };

        let json = serde_json::to_string_pretty(&transaction_request).unwrap();
        let file_name = format!("{}/transactions/num_{}.json", output_dir, i);
        tokio::fs::write(&file_name, &json).await.unwrap();
    }

    address_manager.save_to_file(&format!("{}/address_manager.json", output_dir)).unwrap();
    println!("Generated {} transactions to {}", num_of_transactions, output_dir);

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