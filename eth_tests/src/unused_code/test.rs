const DEFAULT_RPC_URL: &str = "http://localhost:8145";
const TEST_ACCOUNT: &str = "0xf73FfEC0CAEDDCC755C0CE2411569208444F5ca1";
const TEST_ACCOUNT_2: &str = "0x1A585566a3C0C6Dfb607F93DB7Bd3813997d7C06";
const LOCAL_CHAIN_ID: u64 = 9000;
const DEFAULT_MNEMONIC: &str = "orphan assume tent east october arena empower light notice border scatter off thought hawk link lion coconut void huge elegant crucial decline adjust pride";

use ethers::prelude::*;
use ethers::signers::coins_bip39::English;
use ethers::core::k256::ecdsa::SigningKey;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::{convert::TryFrom, str::FromStr};
use ethers::providers::{Http, Middleware, Provider};
use ethers::types::{Transaction, H256, TransactionRequest, U256, NameOrAddress, Bytes, Signature};

pub async fn test_send_raw_transaction() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to an Ethereum node using HTTP
    let provider = Provider::<Http>::try_from(DEFAULT_RPC_URL).expect("Cannot connect URL");
    println!("Chain ID: {}", provider.get_chainid().await?);
    println!("Block number: {}", provider.get_block_number().await?);

    let tx_hash = "0x831ea887beb3654b806ca03469273c570c215a0e5f042c8c1907311cf5d65da7";
    let crawled_transaction = crawl_transaction(tx_hash).await?.expect("JSON RPC cannot call");
    
    let new_transaction = generate_new_transaction_request_hard_code(&crawled_transaction).await?;

    let from_address: H160 = "0x7cB61D4117AE31a12E393a1Cfa3BaC666481D02E".parse()?;
    let new_transaction = new_transaction.from(from_address);
    let nonce = provider.get_transaction_count(from_address, None).await?;
    let new_transaction = new_transaction.nonce(nonce);
    let new_transaction: TypedTransaction = new_transaction.into();
    
    // let signature = sign_transaction(&new_transaction).await?;
    
    // println!("Crawled tx {:?}", crawled_transaction);
    // println!("New tx {:?}", new_transaction);
    // println!("Signature {:?}", signature);
    
    // let raw_tx = new_transaction.rlp_signed(&signature);
    // let result = provider.send_raw_transaction(raw_tx).await?;
    let result = provider.send_transaction(new_transaction, None).await?;
    println!("{:?}", result);
    Ok(())
}

pub async fn get_balance() -> Result<(), Box<dyn std::error::Error>> {
    let provider = Provider::<Http>::try_from(DEFAULT_RPC_URL).expect("Cannot connect URL");
    // Valid address
    let balance = provider.get_balance(TEST_ACCOUNT, None).await?;

    // assert!(
    //     balance > U256::zero(),
    //     "balance should be greater than zero"
    // );

    // // Random address
    // let balance = provider
    //     .get_balance("0x1A585566a3C0C6Dfb607F93DB7Bd3813997d7C06", None)
    //     // .get_balance("0xf73FfEC0CAEDDCC755C0CE2411569208444F5ca1", None)
    //     .await?;
    println!("{}", balance);
    // assert!(balance == U256::zero(), "balance should be zero");
    Ok(())
}

pub async fn crawl_transaction(tx_hash: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error>> {
    // Create provider
    let test_provider = Provider::<Http>::try_from("https://evmos.drpc.org").expect("Cannot connect network");
    
    // Parse tx_hash
    let tx_hash: H256 = tx_hash.parse()?;

    // Loop until receive response
    loop {
        let result = test_provider.get_transaction(tx_hash).await;
        if let Ok(Some(crawled_transaction)) = result {
            return Ok(Some(crawled_transaction))
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    Ok(None)
}

pub async fn generate_new_transaction_request(
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
        // chain_id: Some(U64([chain_id])),
        ..Default::default()
    };
    Ok(new_transaction_request)
}

pub async fn generate_new_transaction_request_hard_code(
    crawled_transaction: &Transaction,
) -> Result<TransactionRequest, Box<dyn std::error::Error>> {
    // Your logic to generate the new transaction request here
    // This could involve extracting relevant fields from the crawled_transaction object
    // and constructing a TransactionRequest struct
    // For demonstration, let's assume a simple implementation

    // let new_transaction_request = TransactionRequest {
    //     from: Some(crawled_transaction.from),
    //     to: crawled_transaction.to.map(NameOrAddress::Address),
    //     gas: Some(crawled_transaction.gas),
    //     gas_price: crawled_transaction.gas_price,
    //     value: Some(crawled_transaction.value),
    //     data: Some(crawled_transaction.input.clone()),
    //     ..Default::default()
    // };


    // needing modify parameters: from, to, nonce, chain_id

    // nonce cannot hard code

    let new_transaction_request = TransactionRequest {
        from: Some(TEST_ACCOUNT).map(|x| x.parse().unwrap()),
        to: Some(TEST_ACCOUNT_2).map(|x| x.parse().unwrap()),
        gas: Some(crawled_transaction.gas),
        gas_price: crawled_transaction.gas_price,
        value: Some(crawled_transaction.value),
        data: Some(crawled_transaction.input.clone()),
        nonce: get_transaction_count(TEST_ACCOUNT.parse()?).await?,
        chain_id: Some(U64([LOCAL_CHAIN_ID])),
        ..Default::default()
    };

    Ok(new_transaction_request)
}
// async fn sign_transaction(
//     crawled_transaction: &Transaction,
// ) -> Result<Signature, Box<dyn std::error::Error>> {
//     // Your logic to sign the transaction here
//     // This could involve using a signer to sign the transaction
//     // and obtaining the signature
//     // For demonstration, let's return a dummy signature
//     let r = crawled_transaction.r;
//     let s = crawled_transaction.s;
//     let v = crawled_transaction.v.0[0];
//     let signature = Signature {r, s, v};
//     Ok(signature)
// }

// Define your sign_transaction function here
async fn sign_transaction(
    transaction: &TypedTransaction,
) -> Result<Signature, Box<dyn std::error::Error>> {
    // Create a LocalWallet from the private key
    let wallet = create_test_wallet().await?;

    // Sign the transaction using the wallet
    let signature = wallet.sign_transaction(&transaction).await?;
    Ok(signature)
}

async fn get_transaction_count(
    address: Address,
) -> Result<Option<U256>, Box<dyn std::error::Error>> {
    // Connect url
    let provider = Provider::<Http>::try_from(DEFAULT_RPC_URL).expect("Cannot connect url");

    // Get the transaction count for the address
    let tx_count = provider.get_transaction_count(address, None).await?;

    // println!("Transaction count: {}", tx_count);
    Ok(Some(tx_count))
}    

async fn create_test_wallet(
    // chain_id: u64,
) -> Result<Wallet<SigningKey>, Box<dyn std::error::Error>> {
    /// Includes 20 prefunded accounts with 10_000 ETH each derived from mnemonic "test test test test
    /// test test test test test test test junk".
    /// See crates-reth/primitives/src/chain/spec.rs [DEV] for more details
    const PHRASE: &str = DEFAULT_MNEMONIC;
    // Use first account
    const INDEX: u32 = 0u32;

    // instantiate the wallet with the phrase and the index of the account we want to use
    Ok(MnemonicBuilder::<English>::default()
        .phrase(PHRASE)
        .index(INDEX)?
        .build()?)
        // .with_chain_id(chain_id))
}