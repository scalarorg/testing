use ethers::prelude::*;
use ethers::signers::coins_bip39::English;
use ethers::core::k256::ecdsa::SigningKey;
use std::convert::TryFrom;
use ethers::types::transaction::eip2718::TypedTransaction;

pub async fn create_wallet(mnemonic: &str, index: u32, chain_id: u32) -> Result<Wallet<SigningKey>, Box<dyn std::error::Error>> {
    Ok(MnemonicBuilder::<English>::default()
        .phrase(mnemonic)
        .index(index)?
        .build()?
        .with_chain_id(chain_id))
}

pub async fn send_transaction(
    url: &str,
    transaction: TransactionRequest,
) -> Result<TxHash, Box<dyn std::error::Error>> {
    // Connect to an Ethereum node using HTTP
    let provider = Provider::<Http>::try_from(url)?;
    let chain_id = provider.get_chainid().await?;
    println!("Chain ID: {}", chain_id);
    println!("Block number: {}", provider.get_block_number().await?);

    // Retrieve the sender's address and fetch the nonce
    let from_address = transaction.from.expect("Sender address not specified");
    let nonce = provider.get_transaction_count(from_address, None).await?;
    let transaction = transaction.nonce(nonce);
    // transaction.chain_id(chain_id);
    println!("{:?}", transaction);

    // Sign the transaction
    let transaction: TypedTransaction = transaction.into();
    // let signature = wallet.sign_transaction(&transaction).await?;

    // Send the signed transaction
    // let raw_tx = transaction.rlp_signed(&signature);
    let result = provider.send_transaction(transaction, None).await?;
    // let result = provider.send_raw_transaction(raw_tx).await?;
    println!("Result {:?}", result);
    Ok(result.tx_hash())
}

pub async fn get_balance(url: &str, address: &H160, block: Option<BlockId>) -> Result<U256, Box<dyn std::error::Error>> {
    let provider = Provider::<Http>::try_from(url).expect("Cannot connect URL");
    let balance = provider.get_balance(*address, block).await?;
    Ok(balance)
}

pub async fn get_transaction_by_hash(url: &str, tx_hash: TxHash) -> Result<Option<Transaction>, Box<dyn std::error::Error>> {
    let provider = Provider::<Http>::try_from(url).expect("Cannot connect URL");
    let tx = provider.get_transaction(tx_hash).await?;
    Ok(tx)
}