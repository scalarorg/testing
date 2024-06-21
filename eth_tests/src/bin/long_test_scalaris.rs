use std::error::Error;
use std::env;
use eth_tests::address_manager::AddressManager;
use eth_tests::tester::Tester;
use serde_json;
use web3::types::{Bytes, TransactionRequest, H256, U256};
use web3::transports::Http;
use web3::Web3;
use rand::Rng;

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref LOCAL_URL: String = env::var("LOCAL_URL").expect("LOCAL_URL must be set");
    static ref GENERATED_TRANSACTIONS_OUTPUT_DIR: String =
        env::var("GENERATED_TRANSACTIONS_OUTPUT_DIR").expect("GENERATED_TRANSACTIONS_OUTPUT_DIR must be set");
    static ref CHAIN_ID: u64 = env::var("CHAIN_ID").expect("CHAIN_ID must be set")
        .parse().expect("CHAIN_ID must be integer");
}

async fn make_transaction(transaction: TransactionRequest, tester: &Tester, address_manager: &AddressManager) -> Result<Bytes, Box<dyn Error>> {
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
            Ok(tx_hash) => {
                println!("Sent transaction hash: {:?}", tx_hash);
                return Ok(tx_hash)
            }
            Err(_) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
        }
    }
}

async fn wait_transaction_receipt(web3: Web3<Http>, tx_hash: H256) -> Result<H256, Box<dyn Error>> {
    loop {
        match web3.eth().transaction_receipt(tx_hash).await {
            Ok(Some(_)) => {
                println!("Received transaction receipt: {:?}", tx_hash);
                return Ok(tx_hash)
            }
            Ok(None) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
            Err(_) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
        }
    }
}

use serde::Serialize;
#[derive(Serialize)]
struct MyTransactionRequest {
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

async fn create_transaction(address_manager: &AddressManager, i: usize) -> Result<MyTransactionRequest, Box<dyn std::error::Error>> {
    let (_, addresses_size) = address_manager.info();
    let mut rng = rand::thread_rng();
    let to = address_manager.get_converted_address_with_index(i%addresses_size);
    let from = address_manager.get_converted_address_with_index(i%addresses_size);
    let value = U256::from(random_usize(&mut rng, 1, 100));
    let gas = U256::from(random_usize(&mut rng, 5000000, 5100000));
    let gasPrice = U256::from(random_usize(&mut rng, 0, 10000)) + 6675204846300u64;

    let transaction_request = MyTransactionRequest {
        to,
        from,
        value,
        gas,
        gasPrice,
        // data,
    };
    Ok(transaction_request)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load env variables
    let env_path = ".env.long_test_scalaris";
    dotenv::from_path(env_path).ok();

    let transactions_folder = &*GENERATED_TRANSACTIONS_OUTPUT_DIR;
    remove_and_recreate_output_directory(&transactions_folder).await?;
    let url = (*LOCAL_URL).as_str();
    let tester = Tester::new(url).await?;

    let address_manager_path = "../evmos_build_env/address_manager_743.json";
    let address_manager = AddressManager::load_from_file(address_manager_path).unwrap();

    for index in 0..10 {
        let transaction = create_transaction(&address_manager, index).await?;
        let transaction = serde_json::to_string_pretty(&transaction).unwrap();
        let file_name = format!("{}/transactions/num_{}.json", transactions_folder, index);
        tokio::fs::write(&file_name, &transaction).await?;
        let transaction: TransactionRequest = serde_json::from_str(&transaction).unwrap();
        let signed_raw_transaction = make_transaction(transaction, &tester, &address_manager).await?;
        let tx_hash = send_signed_raw_transaction(tester.web3.clone(), signed_raw_transaction).await?;
        let tx_hash = wait_transaction_receipt(tester.web3.clone(), tx_hash).await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
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
