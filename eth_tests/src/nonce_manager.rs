use std::collections::HashMap;
use tokio::sync::Mutex;
use web3::transports::Http;
use web3::types::{H160, U64};
use web3::{Transport, Web3};
use std::sync::Arc;

pub struct NonceManager{
    nonces: Mutex<HashMap<H160, u64>>,
}

impl NonceManager {
    pub fn new() -> Self {
        NonceManager {
            nonces: Mutex::new(HashMap::new())
        }
    }

    pub async fn info(&self) {
        let nonces = self.nonces.lock().await;
        let len = nonces.len();
        println!("Size of NonceManager: {}", len);
    }

    // Get the next nonce for a given address and increment it
    pub async fn get_and_increment_nonce(&self, web3: &Web3<Http>, address: H160) -> Result<u64, web3::Error> {
        let mut nonces = self.nonces.lock().await;
        let nonce = nonces.entry(address).or_insert_with(|| {
            let current_nonce = futures::executor::block_on(web3.eth().transaction_count(address, None));
            // current_nonce.unwrap_or_else(|_| 0.into()).as_u64()
            current_nonce.unwrap().as_u64()
        });

        let current_nonce = *nonce;
        *nonce += 1;

        Ok(current_nonce)
    }
}
