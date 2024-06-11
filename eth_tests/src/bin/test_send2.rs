use std::fs;
use web3::types::{TransactionRequest, TransactionParameters, Transaction, BlockNumber};
use web3::transports::Http;
use tokio;
use eth_tests::address_manager::AddressManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let address_manager_path = "../converted_evmos_mainnet_transactions_01/address_manager.json";
    let address_manager = AddressManager::load_from_file(address_manager_path).unwrap();

    // let path = "test_tx_1.json";
    let path = "../generated_transactions_reth/transactions/num_4.json";

    let http = Http::new("http://127.0.0.1:8545")?;
    let web3 = web3::Web3::new(http);

    let content = fs::read_to_string(path)?;
    let transaction: TransactionRequest = serde_json::from_str(&content)?;
    let from_address = transaction.from; // Ensure from address is present

    // let priv_key = hex::decode("caef93e5b0693cddca9124fdc8f61832dd1927d661bbe572d8c02c415745db79")?;
    let priv_key = address_manager.get_private_key(&from_address).expect("Cannot retrieve private key");
    let priv_key = hex::decode(priv_key)?;
    let key = web3::signing::SecretKey::from_slice(&priv_key)?;

    let nonce = web3.eth().transaction_count(from_address.clone(), Some(BlockNumber::Pending)).await?;
    let tx_params = TransactionParameters {
        nonce: Some(nonce),
        gas_price: transaction.gas_price,
        gas: transaction.gas.unwrap_or_default(),
        to: transaction.to, 
        value: transaction.value.unwrap_or_default(),
        data: transaction.data.unwrap_or_default(),
        chain_id: Some(12345),
        ..Default::default()
    };

    println!("Sending transaction: {:?}", tx_params);

    let signed_tx = web3.accounts().sign_transaction(tx_params, &key).await?;
    println!("Signed transaction: {:?}", signed_tx.raw_transaction);
    let tx_hash = web3.eth().send_raw_transaction(signed_tx.raw_transaction).await?;

    println!("Transaction sent. Hash: {:?}", tx_hash);

    let tx: Option<Transaction> = web3.eth().transaction(web3::types::TransactionId::Hash(tx_hash)).await?;

    if let Some(tx) = tx {
        println!("{:?}", tx);
    } else {
        println!("Transaction not found");
    }
    
    // Wait for transaction receipt
    loop {
        match web3.eth().transaction_receipt(tx_hash).await? {
            Some(receipt) => {
                println!("Transaction receipt: {:?}", receipt);
                break;
            }
            None => {
                println!("Waiting for transaction to be mined...");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }


    Ok(())
}
