# Reth Testing Performance

log

![Untitled](Reth%20Testing%20Performance%20aeb46350353f4381b9147bb135a95016/Untitled.png)

![Untitled](Reth%20Testing%20Performance%20aeb46350353f4381b9147bb135a95016/Untitled%201.png)

![Untitled](Reth%20Testing%20Performance%20aeb46350353f4381b9147bb135a95016/Untitled%202.png)

![Untitled](Reth%20Testing%20Performance%20aeb46350353f4381b9147bb135a95016/Untitled%203.png)

# Slow function

Example: 10000 transactions

- MiningTask: storage.build_and_execute()
    - loading db provider
    - execute block
- ForkChoiceUpdate
    - too slow
    
    ```
     // send the new update to the engine, this will trigger the engine
                                    // to download and execute the block we just inserted
    ```
    

# Flow of MiningTask

![Untitled](Reth%20Testing%20Performance%20aeb46350353f4381b9147bb135a95016/Untitled.png)

- build_and_execute → build_header_template:  fills in pre-execution header fields based on the current best block and given transactions → need calculate some field of block → quite slow
- storage.build_and_execute → call execute: execute takes only 1/4 time of build_and_execute, calculate root and load db provider takes significant time
- When on ForceChoiceUpdated: send sealed block  → call execute again, build canonical block → very slow
- max_connection (json-rpc) = 500
- leader node execute block 2 times - when propose block and add block to chain
- SLOWEST: build_and_execute → Block with_recovered_senders
- with_recovered_senders(): recover sender address from signature (ensure consistency of data)
- in Reth: already PARALLEL recover sender → but still slowest (3.5 times comparing with execute transaction on EVM, although parallel vs sequentially)
- use try_with_senders_unchecked(senders) to pass recover process → faster but unsecured (already ask in telegram but no anyone response)
- when send_raw_transaction through rpc call→ decode to PooledTransactionsElementEcRecovered (signer: Address, transaction) → already call recover_signer() to get sender address → ? why need to recalculate in execute block session

```rust
    async fn send_raw_transaction(&self, tx: Bytes) -> EthResult<B256> {
        // On optimism, transactions are forwarded directly to the sequencer to be included in
        // blocks that it builds.
        if let Some(client) = self.inner.raw_transaction_forwarder.as_ref() {
            tracing::debug!( target: "rpc::eth",  "forwarding raw transaction to");
            client.forward_raw_transaction(&tx).await?;
        }

        let recovered = recover_raw_transaction(tx)?;
        let pool_transaction = <Pool::Transaction>::from_recovered_pooled_transaction(recovered);

        // submit the transaction to the pool with a `Local` origin
        let hash = self.pool().add_transaction(TransactionOrigin::Local, pool_transaction).await?;

        Ok(hash)
    }
```

```rust
/// Recovers a [PooledTransactionsElementEcRecovered] from an enveloped encoded byte stream.
///
/// See [PooledTransactionsElement::decode_enveloped]
pub(crate) fn recover_raw_transaction(
    data: Bytes,
) -> EthResult<PooledTransactionsElementEcRecovered> {
    if data.is_empty() {
        return Err(EthApiError::EmptyRawTransactionData)
    }

    let transaction = PooledTransactionsElement::decode_enveloped(&mut data.as_ref())
        .map_err(|_| EthApiError::FailedToDecodeSignedTransaction)?;

    transaction.try_into_ecrecovered().or(Err(EthApiError::InvalidTransactionSignature))
}
```

```rust
fn calculate(self, retain_updates: bool) -> Result<StateRootProgress, StateRootError> {
        // tracing info run time for state root
        tracing::info!(target: "trie::state_root", "Huy: state root started");
        let start = std::time::Instant::now();

        trace!(target: "trie::state_root", "calculating state root");
        let mut tracker = TrieTracker::default();
        let mut trie_updates = TrieUpdates::default();

        let trie_cursor = self.trie_cursor_factory.account_trie_cursor()?;

        let hashed_account_cursor = self.hashed_cursor_factory.hashed_account_cursor()?;
        let (mut hash_builder, mut account_node_iter) = match self.previous_state {
            Some(state) => {
                let hash_builder = state.hash_builder.with_updates(retain_updates);
                let walker = TrieWalker::from_stack(
                    trie_cursor,
                    state.walker_stack,
                    self.prefix_sets.account_prefix_set,
                )
                .with_updates(retain_updates);
                let node_iter = TrieNodeIter::new(walker, hashed_account_cursor)
                    .with_last_hashed_key(state.last_account_key);
                (hash_builder, node_iter)
            }
            None => {
                let hash_builder = HashBuilder::default().with_updates(retain_updates);
                let walker = TrieWalker::new(trie_cursor, self.prefix_sets.account_prefix_set)
                    .with_updates(retain_updates);
                let node_iter = TrieNodeIter::new(walker, hashed_account_cursor);
                (hash_builder, node_iter)
            }
        };

        let mut account_rlp = Vec::with_capacity(128);
        let mut hashed_entries_walked = 0;
        while let Some(node) = account_node_iter.try_next()? {
            match node {
                TrieElement::Branch(node) => {
                    tracker.inc_branch();
                    hash_builder.add_branch(node.key, node.value, node.children_are_in_trie);
                }
                TrieElement::Leaf(hashed_address, account) => {
                    tracker.inc_leaf();
                    hashed_entries_walked += 1;

                    // We assume we can always calculate a storage root without
                    // OOMing. This opens us up to a potential DOS vector if
                    // a contract had too many storage entries and they were
                    // all buffered w/o us returning and committing our intermediate
                    // progress.
                    // TODO: We can consider introducing the TrieProgress::Progress/Complete
                    // abstraction inside StorageRoot, but let's give it a try as-is for now.
                    let storage_root_calculator = StorageRoot::new_hashed(
                        self.trie_cursor_factory.clone(),
                        self.hashed_cursor_factory.clone(),
                        hashed_address,
                        #[cfg(feature = "metrics")]
                        self.metrics.storage_trie.clone(),
                    )
                    .with_prefix_set(
                        self.prefix_sets
                            .storage_prefix_sets
                            .get(&hashed_address)
                            .cloned()
                            .unwrap_or_default(),
                    );

                    let storage_root = if retain_updates {
                        let (root, storage_slots_walked, updates) =
                            storage_root_calculator.root_with_updates()?;
                        hashed_entries_walked += storage_slots_walked;
                        trie_updates.extend(updates);
                        root
                    } else {
                        storage_root_calculator.root()?
                    };

                    account_rlp.clear();
                    let account = TrieAccount::from((account, storage_root));
                    account.encode(&mut account_rlp as &mut dyn BufMut);
                    hash_builder.add_leaf(Nibbles::unpack(hashed_address), &account_rlp);

                    // Decide if we need to return intermediate progress.
                    let total_updates_len = trie_updates.len() +
                        account_node_iter.walker.updates_len() +
                        hash_builder.updates_len();
                    if retain_updates && total_updates_len as u64 >= self.threshold {
                        let (walker_stack, walker_updates) = account_node_iter.walker.split();
                        let (hash_builder, hash_builder_updates) = hash_builder.split();

                        let state = IntermediateStateRootState {
                            hash_builder,
                            walker_stack,
                            last_account_key: hashed_address,
                        };

                        trie_updates.extend(walker_updates);
                        trie_updates.extend_with_account_updates(hash_builder_updates);

                        return Ok(StateRootProgress::Progress(
                            Box::new(state),
                            hashed_entries_walked,
                            trie_updates,
                        ))
                    }
                }
            }
        }

        let root = hash_builder.root();

        trie_updates.finalize_state_updates(
            account_node_iter.walker,
            hash_builder,
            self.prefix_sets.destroyed_accounts,
        );

        let stats = tracker.finish();

        #[cfg(feature = "metrics")]
        self.metrics.state_trie.record(stats);

        trace!(
            target: "trie::state_root",
            %root,
            duration = ?stats.duration(),
            branches_added = stats.branches_added(),
            leaves_added = stats.leaves_added(),
            "calculated state root"
        );

        tracing::info!(target: "trie::state_root", time=?start.elapsed(),"Huy: state root completed");
        Ok(StateRootProgress::Complete(root, hashed_entries_walked, trie_updates))
    }
```

## PEVM

parallel Ethereum virtual machine

https://github.com/risechain/pevm