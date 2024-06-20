# Merkle tree in Reth

Reth: merkle.rs

## Simple Merkle tree (binary)

![Untitled](Merkle%20tree%20in%20Reth%209f112c767d264ffd98d89341c6ce4195/Untitled.png)

## **MERKLE PATRICIA TRIE**

In the near future, Ethereum plans to migrate to a [Verkle Tree(opens in a new tab)](https://ethereum.org/en/roadmap/verkle-trees) structure, which will open up many new possibilities for future protocol improvements.

## In Reth

![Untitled](Merkle%20tree%20in%20Reth%209f112c767d264ffd98d89341c6ce4195/Untitled%201.png)

![Untitled](Merkle%20tree%20in%20Reth%209f112c767d264ffd98d89341c6ce4195/Untitled%202.png)

![Untitled](Merkle%20tree%20in%20Reth%209f112c767d264ffd98d89341c6ce4195/Untitled%203.png)

## Execute without verification

fn execute_without_verification

- Prepare block state and config evm
- execute_state_transitions
    - for each transaction in block ⇒ load to evm and commit ⇒ apply_transitions (in memory)
    - Revm support in memory storage (CacheDB) only read an address 1 time and update memory state
- post_execution
    - update state at the end of block
    - increment_balances

```rust
    /// Execute a single block and apply the state changes to the internal state.
    ///
    /// Returns the receipts of the transactions in the block and the total gas used.
    ///
    /// Returns an error if execution fails.
    fn execute_without_verification(
        &mut self,
        block: &BlockWithSenders,
        total_difficulty: U256,
    ) -> Result<EthExecuteOutput, BlockExecutionError> {
        // 1. prepare state on new block
        self.on_new_block(&block.header);

        // 2. configure the evm and execute
        let env = self.evm_env_for_block(&block.header, total_difficulty);

        let output = {
            let evm = self.executor.evm_config.evm_with_env(&mut self.state, env);
            self.executor.execute_state_transitions(block, evm)
        }?;

        // 3. apply post execution changes
        self.post_execution(block, total_difficulty)?;

        Ok(output)
    }
```

```rust

    /// Iterate over received balances and increment all account balances.
    /// If account is not found inside cache state it will be loaded from database.
    ///
    /// Update will create transitions for all accounts that are updated.
    ///
    /// Like [CacheAccount::increment_balance], this assumes that incremented balances are not
    /// zero, and will not overflow once incremented. If using this to implement withdrawals, zero
    /// balances must be filtered out before calling this function.
    pub fn increment_balances(
        &mut self,
        balances: impl IntoIterator<Item = (Address, u128)>,
    ) -> Result<(), DB::Error> {
        // make transition and update cache state
        let mut transitions = Vec::new();
        for (address, balance) in balances {
            if balance == 0 {
                continue;
            }
            let original_account = self.load_cache_account(address)?;
            transitions.push((
                address,
                original_account
                    .increment_balance(balance)
                    .expect("Balance is not zero"),
            ))
        }
        // append transition
        if let Some(s) = self.transition_state.as_mut() {
            s.add_transitions(transitions)
        }
        Ok(())
    }
```

Blockchain Tree in Reth

```rust
/// A Tree of chains.
///
/// The flowchart represents all the states a block can have inside the tree.
///
/// - Green blocks belong to the canonical chain and are saved inside the database.
/// - Pending blocks and sidechains are found in-memory inside [`BlockchainTree`].
///
/// Both pending chains and sidechains have the same mechanisms, the only difference is when they
/// get committed to the database.
///
/// For pending, it is an append operation, but for sidechains they need to move the current
/// canonical blocks to the tree (by removing them from the database), and commit the sidechain
/// blocks to the database to become the canonical chain (reorg).
```

```rust
/// # Main functions
/// * [BlockchainTree::insert_block]: Connect a block to a chain, execute it, and if valid, insert
///   the block into the tree.
/// * [BlockchainTree::finalize_block]: Remove chains that branch off of the now finalized block.
/// * [BlockchainTree::make_canonical]: Check if we have the hash of a block that is the current
///   canonical head and commit it to db.
```

When execute block:

```rust
/// Walks the intermediate nodes of existing state trie (if any) and hashed entries. Feeds the
    /// nodes into the hash builder.
    ///
    /// # Returns
    ///
    /// The state root hash.
    pub fn root(self) -> Result<B256, StateRootError> {
        tracing::info!("Huy: hello");
        match self.calculate(false)? {
            StateRootProgress::Complete(root, _, _) => Ok(root),
            StateRootProgress::Progress(..) => unreachable!(), // update retenion is disabled
        }
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

When execute block:

- Parallel state root: Precalculate storage root in parallel ? (comment in code)
- **Hot fix: this parallel calculate will not run**

```rust
impl<DB, Provider> ParallelStateRoot<DB, Provider>
where
    DB: Database,
    Provider: DatabaseProviderFactory<DB> + Send + Sync,
{
    /// Calculate incremental state root in parallel.
    pub fn incremental_root(self) -> Result<B256, ParallelStateRootError> {
        self.calculate(false).map(|(root, _)| root)
    }

    /// Calculate incremental state root with updates in parallel.
    pub fn incremental_root_with_updates(
        self,
    ) -> Result<(B256, TrieUpdates), ParallelStateRootError> {
        self.calculate(true)
    }

    fn calculate(
        self,
        retain_updates: bool,
    ) -> Result<(B256, TrieUpdates), ParallelStateRootError> {
        let mut tracker = ParallelTrieTracker::default();
        let prefix_sets = self.hashed_state.construct_prefix_sets();
        let storage_root_targets = StorageRootTargets::new(
            self.hashed_state.accounts.keys().copied(),
            prefix_sets.storage_prefix_sets,
        );
        let hashed_state_sorted = self.hashed_state.into_sorted();

        // Pre-calculate storage roots in parallel for accounts which were changed.
        tracker.set_precomputed_storage_roots(storage_root_targets.len() as u64);
        debug!(target: "trie::parallel_state_root", len = storage_root_targets.len(), "pre-calculating storage roots");
        let mut storage_roots = storage_root_targets
            .into_par_iter()
            .map(|(hashed_address, prefix_set)| {
                let provider_ro = self.view.provider_ro()?;
                let storage_root_result = StorageRoot::new_hashed(
                    provider_ro.tx_ref(),
                    HashedPostStateCursorFactory::new(provider_ro.tx_ref(), &hashed_state_sorted),
                    hashed_address,
                    #[cfg(feature = "metrics")]
                    self.metrics.storage_trie.clone(),
                )
                .with_prefix_set(prefix_set)
                .calculate(retain_updates);
                Ok((hashed_address, storage_root_result?))
            })
            .collect::<Result<HashMap<_, _>, ParallelStateRootError>>()?;

        trace!(target: "trie::parallel_state_root", "calculating state root");
        let mut trie_updates = TrieUpdates::default();

        let provider_ro = self.view.provider_ro()?;
        let hashed_cursor_factory =
            HashedPostStateCursorFactory::new(provider_ro.tx_ref(), &hashed_state_sorted);
        let trie_cursor_factory = provider_ro.tx_ref();

        let walker = TrieWalker::new(
            trie_cursor_factory.account_trie_cursor().map_err(ProviderError::Database)?,
            prefix_sets.account_prefix_set,
        )
        .with_updates(retain_updates);
        let mut account_node_iter = TrieNodeIter::new(
            walker,
            hashed_cursor_factory.hashed_account_cursor().map_err(ProviderError::Database)?,
        );

        let mut hash_builder = HashBuilder::default().with_updates(retain_updates);
        let mut account_rlp = Vec::with_capacity(128);
        while let Some(node) = account_node_iter.try_next().map_err(ProviderError::Database)? {
            match node {
                TrieElement::Branch(node) => {
                    hash_builder.add_branch(node.key, node.value, node.children_are_in_trie);
                }
                TrieElement::Leaf(hashed_address, account) => {
                    let (storage_root, _, updates) = match storage_roots.remove(&hashed_address) {
                        Some(result) => result,
                        // Since we do not store all intermediate nodes in the database, there might
                        // be a possibility of re-adding a non-modified leaf to the hash builder.
                        None => {
                            tracker.inc_missed_leaves();
                            StorageRoot::new_hashed(
                                trie_cursor_factory,
                                hashed_cursor_factory.clone(),
                                hashed_address,
                                #[cfg(feature = "metrics")]
                                self.metrics.storage_trie.clone(),
                            )
                            .calculate(retain_updates)?
                        }
                    };

                    if retain_updates {
                        trie_updates.extend(updates.into_iter());
                    }

                    account_rlp.clear();
                    let account = TrieAccount::from((account, storage_root));
                    account.encode(&mut account_rlp as &mut dyn BufMut);
                    hash_builder.add_leaf(Nibbles::unpack(hashed_address), &account_rlp);
                }
            }
        }

        let root = hash_builder.root();

        trie_updates.finalize_state_updates(
            account_node_iter.walker,
            hash_builder,
            prefix_sets.destroyed_accounts,
        );

        let stats = tracker.finish();

        #[cfg(feature = "metrics")]
        self.metrics.record_state_trie(stats);

        trace!(
            target: "trie::parallel_state_root",
            %root,
            duration = ?stats.duration(),
            branches_added = stats.branches_added(),
            leaves_added = stats.leaves_added(),
            missed_leaves = stats.missed_leaves(),
            precomputed_storage_roots = stats.precomputed_storage_roots(),
            "calculated state root"
        );

        Ok((root, trie_updates))
    }
```

# References

[Merkling in Ethereum | Ethereum Foundation Blog](https://blog.ethereum.org/2015/11/15/merkling-in-ethereum)

[Merkle Patricia Trie | ethereum.org](https://ethereum.org/vi/developers/docs/data-structures-and-encoding/patricia-merkle-trie/)

[https://github.com/gabrocheleau/merkle-patricia-trees-examples](https://github.com/gabrocheleau/merkle-patricia-trees-examples)

[How does Ethereum work, anyway?. Introduction | by Preethi Kasireddy | Medium](https://preethikasireddy.medium.com/how-does-ethereum-work-anyway-22d1df506369)