# Reth Execution Task

- [x]  Install Reth and setup Docker
- Clone repository from GitHub

```
git clone https://github.com/paradigmxyz/reth
cd reth
```

```rust
repo reth scalar
https://github.com/scalarorg/reth
```

- Build a docker image from source

```jsx
docker build -t reth:local
```

- Run docker container devnet (run from pull image) — not yet send transaction

```jsx
docker run \
    -v rethdata:/root/.local/share/reth/devnet \
    -d \
    -p 9001:9001 \
    -p 8545:8545 \
    -p 30303:30303 \
    -p 30303:30303/udp \
    --name reth \
    reth:local \
    node \
    --dev \
    --metrics 0.0.0.0:9001 \
    --http --http.addr 0.0.0.0 --http.port 8545
```

- Default CHAIN_ID: 1337
- Run a reth node custom code (bash script)

```jsx
#!/bin/bash

# Define variables
DATA_DIR="../reth-json/tmp/screth"
GENESIS_FILE="../reth-json/chain.json"

# Remove existing data directory
rm -rf $DATA_DIR

# Set the RUST_LOG environment variable for logging
export RUST_LOG="info"

# Run the Reth node with the specified parameters
cargo run --release -- node -d --chain $GENESIS_FILE \
	--datadir $DATA_DIR --auto-mine --http

```

- [x]  Test send transactions to Reth
- Get the balance of a Genesis account

```jsx
curl --location '127.0.0.1:8545' \
--header 'Content-Type: application/json' \
--data '{
    "jsonrpc":"2.0",
    "method":"eth_getBalance",
    "params":["0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65", 
    "latest"],
    "id":1
}'
```

```jsx
curl --location '127.0.0.1:8545' \
--header 'Content-Type: application/json' \
--data '{
    "jsonrpc":"2.0",
    "method":"eth_getTransactionCount",
    "params":["0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC","latest"],
    "id":1
}'
```

```jsx
curl --location '127.0.0.1:8545' \
--header 'Content-Type: application/json' \
--data '{
    "jsonrpc":"2.0",
    "method":"eth_blockNumber",
    "params":[],
    "id":1
}'
```

Note:

- Unlike when testing send_transactions with EVMOS, ethers method provider.send_transaction() method cannot run directly because of the invalid signature → Rewrite Tester struct to sign the transaction before sending
- ethers provider.getTransactionCount() method always return 0x0 because the Reth doesn't have Consensus Layer → cannot commit block → always in block 1
- Reth is just an executor and does not have consensus. By default with —auto-mine flag in dev, Reth use AutoSealConsensus

- [ ]  Benchmark each component: EVM, DB

```jsx
cast send --from 0x1a585566a3c0c6dfb607f93db7bd3813997d7c06 --value 1 \
	--legacy --private-key caef93e5b0693cddca9124fdc8f61832dd1927d661bbe572d8c02c415745db79 \
	0x84e21a432cbf66913a907176d560386e8a37a89d
```

```bash
cast balance 0x84e21a432cbf66913a907176d560386e8a37a89d
```

Problem when send transaction to Reth:

- JavaScript or cast send: block_number moving → can resend transaction
- Rust: block_number does not moving ??? (try use ethers and web3)

→ Wrong gas or gas limit on the transaction 

- Reth auto calculate nonce (different from EVMOS) → preset nonce make WARN in executor → transaction need re-execute → do not preset nonce when send a transaction

## Solution

**Try with 1 succeed transaction without preset nonce → replicate N transactions with few changes → successfully send to executor**

Next problem:

- Reth executor try to execute 1 transaction multiple times → Change mining mode to interval (default dev net is instant) in fn dev_mining_mode()
    - Sol 1: hard_code dev_mining mode return MiningMode interval 3s
    - Sol 2: use —dev.block-time
- Exceed block gas limit → Fix hard code ETHEREUM_BLOCK_GAS_LIMIT to large value

Tasks:

- Log info of mem pool and MiningTask

# References

- [Inside Reth: The journey of a transaction through Ethereum (superchain.network)](https://blog.superchain.network/inside-reth-the-journey-of-a-tranaction-through-ethereum/)