use web3::signing::SecretKey;
use web3::transports::Http;
use web3::types::{BlockNumber, Bytes, Transaction, TransactionId, TransactionParameters, TransactionReceipt, TransactionRequest, H160, H256, U256};
use web3::Web3;
use crate::nonce_manager::NonceManager;

pub struct Tester {
    pub web3: Web3<Http>,
    nonce_manager: NonceManager,
}

impl Tester {
    pub async fn new(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let transport = web3::transports::Http::new(url)?;
        let web3 = web3::Web3::new(transport);
        let nonce_manager = NonceManager::new();
        Ok(Self { web3, nonce_manager })
    }

    pub async fn send_transaction_with_private_key(
        &self,
        transaction: TransactionRequest,
        key: SecretKey,
        chain_id: u64,
    ) -> Result<H256, Box<dyn std::error::Error>> {
        // Retrieve the sender's address and fetch the nonce
        let from_address = transaction.from;
        let nonce = self.nonce_manager.get_and_increment_nonce(&self.web3, from_address).await?;

        // Sign the transaction
        let tx_params = TransactionParameters {
            nonce: Some(U256::from(nonce)),
            gas_price: transaction.gas_price,
            gas: transaction.gas.unwrap_or_default(),
            to: transaction.to, 
            value: transaction.value.unwrap_or_default(),
            data: transaction.data.unwrap_or_default(),
            chain_id: Some(chain_id),
            ..Default::default()
        };
        // println!("tx_params: {:?}", tx_params);

        let signed_tx = self.web3.accounts().sign_transaction(tx_params, &key).await?;
        // println!("Signed transaction: {:?}", signed_tx.raw_transaction);

        // Send the transaction
        
        let tx_hash = self.web3.eth().send_raw_transaction(signed_tx.raw_transaction).await?;
        Ok(tx_hash)
    }

    pub async fn make_signed_raw_transaction(
        &self,
        transaction: TransactionRequest,
        key: SecretKey,
        chain_id: u64,
    ) -> Result<Bytes, Box<dyn std::error::Error>> {
        // Retrieve the sender's address and fetch the nonce
        let from_address = transaction.from;
        let nonce = self.nonce_manager.get_and_increment_nonce(&self.web3, from_address).await?;

        // Sign the transaction
        let tx_params = TransactionParameters {
            nonce: Some(U256::from(nonce)),
            gas_price: transaction.gas_price,
            gas: transaction.gas.unwrap_or_default(),
            to: transaction.to, 
            value: transaction.value.unwrap_or_default(),
            data: transaction.data.unwrap_or_default(),
            chain_id: Some(chain_id),
            ..Default::default()
        };
        // println!("tx_params: {:?}", tx_params);

        let signed_tx = self.web3.accounts().sign_transaction(tx_params, &key).await?;
        // println!("Signed transaction: {:?}", signed_tx.raw_transaction);

        Ok(signed_tx.raw_transaction)
    }

    pub async fn get_transaction_receipt(&self, tx_hash: H256) -> Result<Option<TransactionReceipt>, Box<dyn std::error::Error>> {
        let receipt = self.web3.eth().transaction_receipt(tx_hash).await?;
        Ok(receipt)
    }

    pub async fn get_balance(&self, address: H160, block: Option<BlockNumber>) -> Result<U256, Box<dyn std::error::Error>> {
        let balance = self.web3.eth().balance(address, block).await?;
        Ok(balance)
    }

    pub async fn get_transaction_by_hash(&self, tx_hash: H256) -> Result<Option<Transaction>, Box<dyn std::error::Error>> {
        let tx = self.web3.eth().transaction(TransactionId::Hash(tx_hash)).await?;
        Ok(tx)
    }
}
