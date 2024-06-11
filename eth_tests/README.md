# Guide

## 1. Crawl transactions
Crawl transactions from API https://evmos.drpc.org (new API: https://evmos.lava.build)
[Code](src/bin/crawl_evmos_mainnet.rs)
```bash
cargo run --bin crawl_evmos_mainnet
```

## 2. Convert transactions
Convert transactions to executable transactions on EVMOS
[Code](src/bin/convert_addresses.rs)
```bash
cargo run --bin convert_addresses
```

## 3. Send transactions
Send all converted transactions on EVMOS environment
[Code](src/bin/send_transactions.rs)
```bash
cargo run --bin send_transactions
```
# Status

## Folder: ../evmos_mainnet_transactions_03
- Number of transactions: 2433483
- Start block: 19000000
- End block: > ~193000000 (not exactly because of async call and break)

## Folder ../converted_evmos_mainnet_transactions_03
- Mnemonic: "orphan assume tent east october arena empower light notice border scatter off thought hawk link lion coconut void huge elegant crucial decline adjust pride"
- Number of transactions: 2433483
- Number of allocated addresses: 30138

## Project
- [x] Prepare the environment: clone and install evmos, install evmosd

- [x] Crawl transactions from the evmos mainnet
- Goal: 1 million transactions
- Now: 10000 transactions
- Problems: limit of API call
- Update now: >2.4 million transactions by using sleep (from block 19000000 to block  ~19300000)

- [x] Convert transactions to executable transactions on test environment
- Generate 1 million tuples (private key, public key, address) from mnemonic
- Generate hashmap from crawled transaction addresses to generated addresses
> Note: EvmOS use tendermint/PrivKeyEd25519 criteria, ethers use Secp256k1, cannot convert between them → must use mnemonic 

- Map from and to addresses
- Save hashmap to file
- Modify chain_id: test chain_id 
- Recalculate nonce: calculate when executing

- [x] Setup the test evmos environment
- Create a new genesis file to initialize genesis accounts, balance, and genesis transactions 
- Setup 4 evmos nodes on docker
> Note: Account balances are initialized equal to a very big fixed number (may calculate to defined exactly)

- [x] Execute transactions on the test environment
- Load a transaction in a file path to execute
- Load all transactions in a directory to execute

- [x] Verify correctness of execution
- Check balance after 1 transaction
- Check balance after some transactions