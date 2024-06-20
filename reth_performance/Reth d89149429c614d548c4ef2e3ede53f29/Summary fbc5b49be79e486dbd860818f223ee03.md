# Summary

## Reth Auto Seal Consensus diagram

1. Link diagram: https://drive.google.com/file/d/1cocqKJ-9WSZJWFOXt5700fShWogO5ioR/view?usp=sharing

![Untitled](Summary%20fbc5b49be79e486dbd860818f223ee03/Untitled.png)

1. Components
- EthApi and EthEngineApi: handle eth_ rpc call (eth_sendRawTransaction)
- TxPool:  maintains the state of all transactions and stores them accordingly. Including 4 sub-pools:
    - The `Queued` pool contains transactions with gaps in its dependency tree: It requires additional transactions that are note yet present in the pool. And transactions that the sender can not afford with the current balance.
    - The `Pending` pool contains all transactions that have no nonce gaps, and can be afforded by the sender. It only contains transactions that are ready to be included in the pending block. The pending pool contains all transactions that could be listed currently, but not necessarily independently. However, this pool never contains transactions with nonce gaps. A transaction is considered `ready` when it has the lowest nonce of all transactions from the same sender. Which is equals to the chain nonce of the sender in the pending pool.
    - The `BaseFee` pool contains transactions that currently can't satisfy the dynamic fee requirement. With EIP-1559, transactions can become executable or not without any changes to the sender's balance or nonce and instead their `feeCap` determines whether the transaction is _currently_ (on the current state) ready or needs to be parked until the `feeCap` satisfies the block's `baseFee`.
    - The `Blob` pool contains _blob_ transactions that currently can't satisfy the dynamic fee requirement, or blob fee requirement. Transactions become executable only if the transaction `feeCap` is greater than the block's `baseFee` and the `maxBlobFee` is greater than the block's `blobFee`.
- BlockchainTree:
    - The flowchart represents all the states a block can have inside the tree.
        - Green blocks belong to the canonical chain and are saved inside the database.
        - Pending blocks and sidechains are found in-memory inside [`BlockchainTree`].
    - Both pending chains and sidechains have the same mechanisms, the only difference is when they get committed to the database.
    - For pending, it is an append operation, but for sidechains they need to move the current canonical blocks to the tree (by removing them from the database), and commit the sidechain blocks to the database to become the canonical chain (reorg).
    - Main functions:
        - [BlockchainTree::insert_block]: Connect a block to a chain, execute it, and if valid, insert the block into the tree.
        - [BlockchainTree::finalize_block]: Remove chains that branch off of the now finalized block.
        - [BlockchainTree::make_canonical]: Check if we have the hash of a block that is the current canonical head and commit it to db.
- AutoSealConsensus and AutoSealClient: implement for local testing without consensus
- MiningTask: runs with AutoSealBuilder, listens for new ready transactions and puts new blocks into storage
- EthBeaconConsensus: ethereum beacon consensus
- BeaconConsensusEngine:

```markdown
 The beacon consensus engine is the driver that switches between historical and live sync.

 The beacon consensus engine is itself driven by messages from the Consensus Layer, which are
 received by Engine API (JSON-RPC).

 The consensus engine is idle until it receives the first
 [BeaconEngineMessage::ForkchoiceUpdated] message from the CL which would initiate the sync. At
 first, the consensus engine would run the [Pipeline] until the latest known block hash.
 Afterward, it would attempt to create/restore the [`BlockchainTreeEngine`] from the blocks
 that are currently available. In case the restoration is successful, the consensus engine would
 run in a live sync mode, populating the [`BlockchainTreeEngine`] with new blocks as they arrive
 via engine API and downloading any missing blocks from the network to fill potential gaps.

 The consensus engine has two data input sources:

 ## New Payload (`engine_newPayloadV{}`)

 The engine receives new payloads from the CL. If the payload is connected to the canonical
 chain, it will be fully validated added to a chain in the [BlockchainTreeEngine]: `VALID`

 If the payload's chain is disconnected (at least 1 block is missing) then it will be buffered:
 `SYNCING` ([BlockStatus::Disconnected]).

 ## Forkchoice Update (FCU) (`engine_forkchoiceUpdatedV{}`)

 This contains the latest forkchoice state and the payload attributes. The engine will attempt to
 make a new canonical chain based on the `head_hash` of the update and trigger payload building
 if the `payload_attrs` are present and the FCU is `VALID`.

 The `head_hash` forms a chain by walking backwards from the `head_hash` towards the canonical
 blocks of the chain.

 Making a new canonical chain can result in the following relevant outcomes:

 ### The chain is connected

 All blocks of the `head_hash`'s chain are present in the [BlockchainTreeEngine] and are
 committed to the canonical chain. This also includes reorgs.

 ### The chain is disconnected

 In this case the [BlockchainTreeEngine] doesn't know how the new chain connects to the existing
 canonical chain. It could be a simple commit (new blocks extend the current head) or a re-org
 that requires unwinding the canonical chain.

 This further distinguishes between two variants:

 #### `head_hash`'s block exists

 The `head_hash`'s block was already received/downloaded, but at least one block is missing to
 form a _connected_ chain. The engine will attempt to download the missing blocks from the
 network by walking backwards (`parent_hash`), and then try to make the block canonical as soon
 as the chain becomes connected.

 However, it still can be the case that the chain and the FCU is `INVALID`.

 #### `head_hash` block is missing

 This is similar to the previous case, but the `head_hash`'s block is missing. At which point the
 engine doesn't know where the new head will point to: new chain could be a re-org or a simple
 commit. The engine will download the missing head first and then proceed as in the previous
 case.

 # Panics

 If the future is polled more than once. Leads to undefined state.
```

- EthBlockExecutor, EthEvmExecutor and revm: help execute transactions in a block

## Reth Performance - Auto Seal

![Untitled](Summary%20fbc5b49be79e486dbd860818f223ee03/Untitled%201.png)

Expensive functions and components: 

### Build block header template

- build_and_execute → build_header_template:  fills in pre-execution header fields based on the current best block and given transactions → need calculate some field of block → quite slow

### Execute

- storage.build_and_execute → call execute: execute takes only 1/4 time of build_and_execute, calculate root and load db provider takes significant time
- When on ForceChoiceUpdated: send sealed block  → call execute again, build canonical block → very slow
- leader node execute block 2 times - when propose block and add block to chain

### Recover sender

- SLOWEST: build_and_execute → Block with_recovered_senders
- with_recovered_senders(): recover sender address from signature (ensure consistency of data)
- in Reth: already PARALLEL recover sender → but still slowest (3.5 times comparing with execute transaction on EVM, although parallel vs sequentially)
- use try_with_senders_unchecked(senders) to pass recover process → faster but unsecured (already ask in telegram but no anyone response)
- when send_raw_transaction through rpc call→ decode to PooledTransactionsElementEcRecovered (signer: Address, transaction) → already call recover_signer() to get sender address → ? why need to recalculate in execute block session

### Calculate root hash

- Run after each block execution to calculate root hash
- Walks the intermediate nodes of existing state trie (if any) and hashed entries. Feeds the nodes into the hash builder → returns the state root hash.