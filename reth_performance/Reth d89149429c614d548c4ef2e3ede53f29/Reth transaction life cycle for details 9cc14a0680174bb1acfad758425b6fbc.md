# Reth transaction life cycle for details

1. Initiation: A user initiates a transaction by signing it with their private key.
    - eth_sendRawTransaction(): send transaction as Bytes
2. Propagation: The transaction is broadcasted to the network: Broadcast ? (because running in 1 node)
3. Validation: The transaction is validated by nodes in the network.
    - impl EthTransactions for EthApi: send_raw_transaction()
    
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
    
            // submit the transaction to the pool with a `Local` originz
            let hash = self.pool().add_transaction(TransactionOrigin::Local, pool_transaction).await?;
    
            Ok(hash)
        }
    ```
    
    - impl TransactionPool for Pool: add_transaction()
    
    ```rust
    async fn add_transaction(
            &self,
            origin: TransactionOrigin,
            transaction: Self::Transaction,
        ) -> PoolResult<TxHash> {
            let (_, tx) = self.validate(origin, transaction).await;
            let mut results = self.pool.add_transactions(origin, std::iter::once(tx));
            results.pop().expect("result length is the same as the input")
        }
    
    ```
    
- validate() →  impl validate_transaction():
    
    ```rust
    /// Validates the transaction and returns a [`TransactionValidationOutcome`] describing the
    /// validity of the given transaction.
    ///
    /// This will be used by the transaction-pool to check whether the transaction should be
    /// inserted into the pool or discarded right away.
    ///
    /// Implementers of this trait must ensure that the transaction is well-formed, i.e. that it
    /// complies at least all static constraints, which includes checking for:
    ///
    ///    * chain id
    ///    * gas limit
    ///    * max cost
    ///    * nonce >= next nonce of the sender
    ///    * ...
    ///
    /// See [InvalidTransactionError](reth_primitives::InvalidTransactionError) for common errors
    /// variants.
    ///
    /// The transaction pool makes no additional assumptions about the validity of the transaction
    /// at the time of this call before it inserts it into the pool. However, the validity of
    /// this transaction is still subject to future (dynamic) changes enforced by the pool, for
    /// example nonce or balance changes. Hence, any validation checks must be applied in this
    /// function.
    ///
    /// See [TransactionValidationTaskExecutor] for a reference implementation.
    ```
    
    - Struct PoolInner: add_transactions()
    
    ```rust
    pub fn add_transactions(
            &self,
            origin: TransactionOrigin,
            transactions: impl IntoIterator<Item = TransactionValidationOutcome<T::Transaction>>,
        ) -> Vec<PoolResult<TxHash>> {
            let mut added =
                transactions.into_iter().map(|tx| self.add_transaction(origin, tx)).collect::<Vec<_>>();
    
            // If at least one transaction was added successfully, then we enforce the pool size limits.
            let discarded =
                if added.iter().any(Result::is_ok) { self.discard_worst() } else { Default::default() };
    
            if discarded.is_empty() {
                return added
            }
    
            {
                let mut listener = self.event_listener.write();
                discarded.iter().for_each(|tx| listener.discarded(tx));
            }
    
            // It may happen that a newly added transaction is immediately discarded, so we need to
            // adjust the result here
            for res in added.iter_mut() {
                if let Ok(hash) = res {
                    if discarded.contains(hash) {
                        *res = Err(PoolError::new(*hash, PoolErrorKind::DiscardedOnInsert))
                    }
                }
            }
    
            added
        }
    ```
    
    - Struct PoolInner: add_transaction()
    
    ```rust
    /// Add a single validated transaction into the pool.
      ///
      /// Note: this is only used internally by [`Self::add_transactions()`], all new transaction(s)
      /// come in through that function, either as a batch or `std::iter::once`.
      fn add_transaction(
          &self,
          origin: TransactionOrigin,
          tx: TransactionValidationOutcome<T::Transaction>,
      ) -> PoolResult<TxHash> {
          match tx {
              TransactionValidationOutcome::Valid {
                  balance,
                  state_nonce,
                  transaction,
                  propagate,
              } => {
                  let sender_id = self.get_sender_id(transaction.sender());
                  let transaction_id = TransactionId::new(sender_id, transaction.nonce());
    
                  // split the valid transaction and the blob sidecar if it has any
                  let (transaction, maybe_sidecar) = match transaction {
                      ValidTransaction::Valid(tx) => (tx, None),
                      ValidTransaction::ValidWithSidecar { transaction, sidecar } => {
                          debug_assert!(
                              transaction.is_eip4844(),
                              "validator returned sidecar for non EIP-4844 transaction"
                          );
                          (transaction, Some(sidecar))
                      }
                  };
    
                  let tx = ValidPoolTransaction {
                      transaction,
                      transaction_id,
                      propagate,
                      timestamp: Instant::now(),
                      origin,
                  };
    
                  let added = self.pool.write().add_transaction(tx, balance, state_nonce)?;
                  let hash = *added.hash();
    
                  // transaction was successfully inserted into the pool
                  if let Some(sidecar) = maybe_sidecar {
                      // notify blob sidecar listeners
                      self.on_new_blob_sidecar(&hash, &sidecar);
                      // store the sidecar in the blob store
                      self.insert_blob(hash, sidecar);
                  }
    
                  if let Some(replaced) = added.replaced_blob_transaction() {
                      // delete the replaced transaction from the blob store
                      self.delete_blob(replaced);
                  }
    
                  // Notify about new pending transactions
                  if let Some(pending) = added.as_pending() {
                      self.on_new_pending_transaction(pending);
                  }
    
                  // Notify tx event listeners
                  self.notify_event_listeners(&added);
    
                  if let Some(discarded) = added.discarded_transactions() {
                      self.delete_discarded_blobs(discarded.iter());
                  }
    
                  // Notify listeners for _all_ transactions
                  self.on_new_transaction(added.into_new_transaction_event());
    
                  Ok(hash)
              }
              TransactionValidationOutcome::Invalid(tx, err) => {
                  let mut listener = self.event_listener.write();
                  listener.discarded(tx.hash());
                  Err(PoolError::new(*tx.hash(), err))
              }
              TransactionValidationOutcome::Error(tx_hash, err) => {
                  let mut listener = self.event_listener.write();
                  listener.discarded(&tx_hash);
                  Err(PoolError::other(tx_hash, err))
              }
          }
      }
    ```
    
    - Impl <Transaction: T>TxPool<T>: add_transaction()
    
    ```rust
        /// Adds the transaction into the pool.
        ///
        /// This pool consists of four sub-pools: `Queued`, `Pending`, `BaseFee`, and `Blob`.
        ///
        /// The `Queued` pool contains transactions with gaps in its dependency tree: It requires
        /// additional transactions that are note yet present in the pool. And transactions that the
        /// sender can not afford with the current balance.
        ///
        /// The `Pending` pool contains all transactions that have no nonce gaps, and can be afforded by
        /// the sender. It only contains transactions that are ready to be included in the pending
        /// block. The pending pool contains all transactions that could be listed currently, but not
        /// necessarily independently. However, this pool never contains transactions with nonce gaps. A
        /// transaction is considered `ready` when it has the lowest nonce of all transactions from the
        /// same sender. Which is equals to the chain nonce of the sender in the pending pool.
        ///
        /// The `BaseFee` pool contains transactions that currently can't satisfy the dynamic fee
        /// requirement. With EIP-1559, transactions can become executable or not without any changes to
        /// the sender's balance or nonce and instead their `feeCap` determines whether the
        /// transaction is _currently_ (on the current state) ready or needs to be parked until the
        /// `feeCap` satisfies the block's `baseFee`.
        ///
        /// The `Blob` pool contains _blob_ transactions that currently can't satisfy the dynamic fee
        /// requirement, or blob fee requirement. Transactions become executable only if the
        /// transaction `feeCap` is greater than the block's `baseFee` and the `maxBlobFee` is greater
        /// than the block's `blobFee`.
        pub(crate) fn add_transaction(
            &mut self,
            tx: ValidPoolTransaction<T::Transaction>,
            on_chain_balance: U256,
            on_chain_nonce: u64,
        ) -> PoolResult<AddedTransaction<T::Transaction>> {
            if self.contains(tx.hash()) {
                return Err(PoolError::new(*tx.hash(), PoolErrorKind::AlreadyImported))
            }
    
            // Update sender info with balance and nonce
            self.sender_info
                .entry(tx.sender_id())
                .or_default()
                .update(on_chain_nonce, on_chain_balance);
    
            match self.all_transactions.insert_tx(tx, on_chain_balance, on_chain_nonce) {
                Ok(InsertOk { transaction, move_to, replaced_tx, updates, .. }) => {
                    // replace the new tx and remove the replaced in the subpool(s)
                    self.add_new_transaction(transaction.clone(), replaced_tx.clone(), move_to);
                    // Update inserted transactions metric
                    self.metrics.inserted_transactions.increment(1);
                    let UpdateOutcome { promoted, discarded } = self.process_updates(updates);
    
                    let replaced = replaced_tx.map(|(tx, _)| tx);
    
                    // This transaction was moved to the pending pool.
                    let res = if move_to.is_pending() {
                        AddedTransaction::Pending(AddedPendingTransaction {
                            transaction,
                            promoted,
                            discarded,
                            replaced,
                        })
                    } else {
                        AddedTransaction::Parked { transaction, subpool: move_to, replaced }
                    };
    
                    // Update size metrics after adding and potentially moving transactions.
                    self.update_size_metrics();
    
                    Ok(res)
                }
                Err(err) => {
                    // Update invalid transactions metric
                    self.metrics.invalid_transactions.increment(1);
                    match err {
                        InsertErr::Underpriced { existing: _, transaction } => Err(PoolError::new(
                            *transaction.hash(),
                            PoolErrorKind::ReplacementUnderpriced,
                        )),
                        InsertErr::FeeCapBelowMinimumProtocolFeeCap { transaction, fee_cap } => {
                            Err(PoolError::new(
                                *transaction.hash(),
                                PoolErrorKind::FeeCapBelowMinimumProtocolFeeCap(fee_cap),
                            ))
                        }
                        InsertErr::ExceededSenderTransactionsCapacity { transaction } => {
                            Err(PoolError::new(
                                *transaction.hash(),
                                PoolErrorKind::SpammerExceededCapacity(transaction.sender()),
                            ))
                        }
                        InsertErr::TxGasLimitMoreThanAvailableBlockGas {
                            transaction,
                            block_gas_limit,
                            tx_gas_limit,
                        } => Err(PoolError::new(
                            *transaction.hash(),
                            PoolErrorKind::InvalidTransaction(
                                InvalidPoolTransactionError::ExceedsGasLimit(
                                    block_gas_limit,
                                    tx_gas_limit,
                                ),
                            ),
                        )),
                        InsertErr::BlobTxHasNonceGap { transaction } => Err(PoolError::new(
                            *transaction.hash(),
                            PoolErrorKind::InvalidTransaction(
                                Eip4844PoolTransactionError::Eip4844NonceGap.into(),
                            ),
                        )),
                        InsertErr::Overdraft { transaction } => Err(PoolError::new(
                            *transaction.hash(),
                            PoolErrorKind::InvalidTransaction(InvalidPoolTransactionError::Overdraft),
                        )),
                        InsertErr::TxTypeConflict { transaction } => Err(PoolError::new(
                            *transaction.hash(),
                            PoolErrorKind::ExistingConflictingTransactionType(
                                transaction.sender(),
                                transaction.tx_type(),
                            ),
                        )),
                    }
                }
            }
        }
    ```
    
    - impl <T: PoolTransaction> AllTransaction<T>: insert_tx()
    
    ```rust
     /// Inserts a new _valid_ transaction into the pool.
        ///
        /// If the transaction already exists, it will be replaced if not underpriced.
        /// Returns info to which sub-pool the transaction should be moved.
        /// Also returns a set of pool updates triggered by this insert, that need to be handled by the
        /// caller.
        ///
        /// These can include:
        ///      - closing nonce gaps of descendant transactions
        ///      - enough balance updates
        ///
        /// Note: For EIP-4844 blob transactions additional constraints are enforced:
        ///      - new blob transactions must not have any nonce gaps
        ///      - blob transactions cannot go into overdraft
        ///
        /// ## Transaction type Exclusivity
        ///
        /// The pool enforces exclusivity of eip-4844 blob vs non-blob transactions on a per sender
        /// basis:
        ///  - If the pool already includes a blob transaction from the `transaction`'s sender, then the
        ///    `transaction` must also be a blob transaction
        ///  - If the pool already includes a non-blob transaction from the `transaction`'s sender, then
        ///    the `transaction` must _not_ be a blob transaction.
        ///
        /// In other words, the presence of blob transactions exclude non-blob transactions and vice
        /// versa.
        ///
        /// ## Replacements
        ///
        /// The replacement candidate must satisfy given price bump constraints: replacement candidate
        /// must not be underpriced
        pub(crate) fn insert_tx(
            &mut self,
            transaction: ValidPoolTransaction<T>,
            on_chain_balance: U256,
            on_chain_nonce: u64,
        ) -> InsertResult<T> {
            assert!(on_chain_nonce <= transaction.nonce(), "Invalid transaction");
    
            let mut transaction = self.ensure_valid(transaction)?;
    
            let inserted_tx_id = *transaction.id();
            let mut state = TxState::default();
            let mut cumulative_cost = U256::ZERO;
            let mut updates = Vec::new();
    
            // Current tx does not exceed block gas limit after ensure_valid check
            state.insert(TxState::NOT_TOO_MUCH_GAS);
    
            // identifier of the ancestor transaction, will be None if the transaction is the next tx of
            // the sender
            let ancestor = TransactionId::ancestor(
                transaction.transaction.nonce(),
                on_chain_nonce,
                inserted_tx_id.sender,
            );
    
            // before attempting to insert a blob transaction, we need to ensure that additional
            // constraints are met that only apply to blob transactions
            if transaction.is_eip4844() {
                state.insert(TxState::BLOB_TRANSACTION);
    
                transaction =
                    self.ensure_valid_blob_transaction(transaction, on_chain_balance, ancestor)?;
                let blob_fee_cap = transaction.transaction.max_fee_per_blob_gas().unwrap_or_default();
                if blob_fee_cap >= self.pending_fees.blob_fee {
                    state.insert(TxState::ENOUGH_BLOB_FEE_CAP_BLOCK);
                }
            } else {
                // Non-EIP4844 transaction always satisfy the blob fee cap condition
                state.insert(TxState::ENOUGH_BLOB_FEE_CAP_BLOCK);
            }
    
            let transaction = Arc::new(transaction);
    
            // If there's no ancestor tx then this is the next transaction.
            if ancestor.is_none() {
                state.insert(TxState::NO_NONCE_GAPS);
                state.insert(TxState::NO_PARKED_ANCESTORS);
            }
    
            // Check dynamic fee
            let fee_cap = transaction.max_fee_per_gas();
    
            if fee_cap < self.minimal_protocol_basefee as u128 {
                return Err(InsertErr::FeeCapBelowMinimumProtocolFeeCap { transaction, fee_cap })
            }
            if fee_cap >= self.pending_fees.base_fee as u128 {
                state.insert(TxState::ENOUGH_FEE_CAP_BLOCK);
            }
    
            // placeholder for the replaced transaction, if any
            let mut replaced_tx = None;
    
            let pool_tx = PoolInternalTransaction {
                transaction: Arc::clone(&transaction),
                subpool: state.into(),
                state,
                cumulative_cost,
            };
    
            // try to insert the transaction
            match self.txs.entry(*transaction.id()) {
                Entry::Vacant(entry) => {
                    // Insert the transaction in both maps
                    self.by_hash.insert(*pool_tx.transaction.hash(), pool_tx.transaction.clone());
                    entry.insert(pool_tx);
                }
                Entry::Occupied(mut entry) => {
                    // Transaction with the same nonce already exists: replacement candidate
                    let existing_transaction = entry.get().transaction.as_ref();
                    let maybe_replacement = transaction.as_ref();
    
                    // Ensure the new transaction is not underpriced
                    if Self::is_underpriced(existing_transaction, maybe_replacement, &self.price_bumps)
                    {
                        return Err(InsertErr::Underpriced {
                            transaction: pool_tx.transaction,
                            existing: *entry.get().transaction.hash(),
                        })
                    }
                    let new_hash = *pool_tx.transaction.hash();
                    let new_transaction = pool_tx.transaction.clone();
                    let replaced = entry.insert(pool_tx);
                    self.by_hash.remove(replaced.transaction.hash());
                    self.by_hash.insert(new_hash, new_transaction);
                    // also remove the hash
                    replaced_tx = Some((replaced.transaction, replaced.subpool));
                }
            }
    
            // The next transaction of this sender
            let on_chain_id = TransactionId::new(transaction.sender_id(), on_chain_nonce);
            {
                // get all transactions of the sender's account
                let mut descendants = self.descendant_txs_mut(&on_chain_id).peekable();
    
                // Tracks the next nonce we expect if the transactions are gapless
                let mut next_nonce = on_chain_id.nonce;
    
                // We need to find out if the next transaction of the sender is considered pending
                let mut has_parked_ancestor = if ancestor.is_none() {
                    // the new transaction is the next one
                    false
                } else {
                    // The transaction was added above so the _inclusive_ descendants iterator
                    // returns at least 1 tx.
                    let (id, tx) = descendants.peek().expect("includes >= 1");
                    if id.nonce < inserted_tx_id.nonce {
                        !tx.state.is_pending()
                    } else {
                        true
                    }
                };
    
                // Traverse all transactions of the sender and update existing transactions
                for (id, tx) in descendants {
                    let current_pool = tx.subpool;
    
                    // If there's a nonce gap, we can shortcircuit
                    if next_nonce != id.nonce {
                        break
                    }
    
                    // close the nonce gap
                    tx.state.insert(TxState::NO_NONCE_GAPS);
    
                    // set cumulative cost
                    tx.cumulative_cost = cumulative_cost;
    
                    // Update for next transaction
                    cumulative_cost = tx.next_cumulative_cost();
    
                    if cumulative_cost > on_chain_balance {
                        // sender lacks sufficient funds to pay for this transaction
                        tx.state.remove(TxState::ENOUGH_BALANCE);
                    } else {
                        tx.state.insert(TxState::ENOUGH_BALANCE);
                    }
    
                    // Update ancestor condition.
                    if has_parked_ancestor {
                        tx.state.remove(TxState::NO_PARKED_ANCESTORS);
                    } else {
                        tx.state.insert(TxState::NO_PARKED_ANCESTORS);
                    }
                    has_parked_ancestor = !tx.state.is_pending();
    
                    // update the pool based on the state
                    tx.subpool = tx.state.into();
    
                    if inserted_tx_id.eq(id) {
                        // if it is the new transaction, track its updated state
                        state = tx.state;
                    } else {
                        // check if anything changed
                        if current_pool != tx.subpool {
                            updates.push(PoolUpdate {
                                id: *id,
                                hash: *tx.transaction.hash(),
                                current: current_pool,
                                destination: Destination::Pool(tx.subpool),
                            })
                        }
                    }
    
                    // increment for next iteration
                    next_nonce = id.next_nonce();
                }
            }
    
            // If this wasn't a replacement transaction we need to update the counter.
            if replaced_tx.is_none() {
                self.tx_inc(inserted_tx_id.sender);
            }
    
            self.update_size_metrics();
    
            Ok(InsertOk { transaction, move_to: state.into(), state, replaced_tx, updates })
        }
    
    ```
    
1. Confirmation: Once validated, the transaction gets included in a block and added to the blockchain.
    - MiningTask: poll()
    
    ```rust
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.get_mut();
    
            // this drives block production and
            loop {
                if let Poll::Ready(transactions) = this.miner.poll(&this.pool, cx) {
                    tracing::info!("Found new set of transactions to mine: {}", transactions.len());
                    // miner returned a set of transaction that we feed to the producer
                    this.queued.push_back(transactions);
                }
    
                if this.insert_task.is_none() {
                    if this.queued.is_empty() {
                        // nothing to insert
                        break
                    }
    
                    // ready to queue in new insert task
                    let storage = this.storage.clone();
                    let transactions = this.queued.pop_front().expect("not empty");
    
                    let to_engine = this.to_engine.clone();
                    let client = this.client.clone();
                    let chain_spec = Arc::clone(&this.chain_spec);
                    let pool = this.pool.clone();
                    let events = this.pipe_line_events.take();
                    let canon_state_notification = this.canon_state_notification.clone();
                    let executor = this.block_executor.clone();
    
                    // Create the mining future that creates a block, notifies the engine that drives
                    // the pipeline
                    this.insert_task = Some(Box::pin(async move {
                        // start time estimate
                        let start = tokio::time::Instant::now();
    
                        let mut storage = storage.write().await;
    
                        let (transactions, senders): (Vec<_>, Vec<_>) = transactions
                            .into_iter()
                            .map(|tx| {
                                let recovered = tx.to_recovered_transaction();
                                let signer = recovered.signer();
                                (recovered.into_signed(), signer)
                            })
                            .unzip();
                        let ommers = vec![];
                        // todo(onbjerg): these two dont respect chainspec
                        let withdrawals = Some(Withdrawals::default());
                        let requests = Some(Requests::default());
    
                        match storage.build_and_execute(
                            transactions.clone(),
                            ommers.clone(),
                            withdrawals.clone(),
                            requests.clone(),
                            &client,
                            chain_spec,
                            &executor,
                        ) {
                            Ok((new_header, bundle_state)) => {
                                tracing::info!(target: "consensus::auto", num_txs=?transactions.len(), "Transactions in block");
    
                                // clear all transactions from pool
                                pool.remove_transactions(
                                    transactions.iter().map(|tx| tx.hash()).collect(),
                                );
    
                                let state = ForkchoiceState {
                                    head_block_hash: new_header.hash(),
                                    finalized_block_hash: new_header.hash(),
                                    safe_block_hash: new_header.hash(),
                                };
                                drop(storage);
    
                                // TODO: make this a future
                                // await the fcu call rx for SYNCING, then wait for a VALID response
                                loop {
                                    // send the new update to the engine, this will trigger the engine
                                    // to download and execute the block we just inserted
                                    let (tx, rx) = oneshot::channel();
                                    let _ = to_engine.send(BeaconEngineMessage::ForkchoiceUpdated {
                                        state,
                                        payload_attrs: None,
                                        tx,
                                    });
                                    debug!(target: "consensus::auto", ?state, "Sent fork choice update");
    
                                    match rx.await.unwrap() {
                                        Ok(fcu_response) => {
                                            match fcu_response.forkchoice_status() {
                                                ForkchoiceStatus::Valid => break,
                                                ForkchoiceStatus::Invalid => {
                                                    error!(target: "consensus::auto", ?fcu_response, "Forkchoice update returned invalid response");
                                                    return None
                                                }
                                                ForkchoiceStatus::Syncing => {
                                                    debug!(target: "consensus::auto", ?fcu_response, "Forkchoice update returned SYNCING, waiting for VALID");
                                                    // wait for the next fork choice update
                                                    continue
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            error!(target: "consensus::auto", %err, "Autoseal fork choice update failed");
                                            return None
                                        }
                                    }
                                }
    
                                // seal the block
                                let block = Block {
                                    header: new_header.clone().unseal(),
                                    body: transactions,
                                    ommers,
                                    withdrawals,
                                    requests,
                                };
                                let sealed_block = block.seal_slow();
    
                                let sealed_block_with_senders =
                                    SealedBlockWithSenders::new(sealed_block, senders)
                                        .expect("senders are valid");
    
                                // update canon chain for rpc
                                client.set_canonical_head(new_header.clone());
                                client.set_safe(new_header.clone());
                                client.set_finalized(new_header.clone());
    
                                debug!(target: "consensus::auto", header=?sealed_block_with_senders.hash(), "sending block notification");
    
                                let chain = Arc::new(Chain::new(
                                    vec![sealed_block_with_senders],
                                    bundle_state,
                                    None,
                                ));
    
                                // send block notification
                                let _ = canon_state_notification
                                    .send(reth_provider::CanonStateNotification::Commit { new: chain });
    
                                tracing::info!(target: "consensus::auto", time=?start.elapsed(), "Inserted block");
                            }
                            Err(err) => {
                                warn!(target: "consensus::auto", %err, "failed to execute block")
                            }
                        }
                        events
                    }));
                }
    
                if let Some(mut fut) = this.insert_task.take() {
                    match fut.poll_unpin(cx) {
                        Poll::Ready(events) => {
                            this.pipe_line_events = events;
                        }
                        Poll::Pending => {
                            this.insert_task = Some(fut);
                            break
                        }
                    }
                }
            }
    
            Poll::Pending
        }
    ```
    
    - this.miner.poll(): take transaction from PendingPool (tx ready for block production)
        - with MiningMode::FixedBlockTime → return transactions each interval
    - If no block inserting (this.insert_task.in_none()) → take 1 vec of transactions from queued and call storage.build_and_excute() for new block
    
    ```rust
        /// Builds and executes a new block with the given transactions, on the provided executor.
        ///
        /// This returns the header of the executed block, as well as the poststate from execution.
        #[allow(clippy::too_many_arguments)]
        pub(crate) fn build_and_execute<Provider, Executor>(
            &mut self,
            transactions: Vec<TransactionSigned>,
            ommers: Vec<Header>,
            withdrawals: Option<Withdrawals>,
            requests: Option<Requests>,
            provider: &Provider,
            chain_spec: Arc<ChainSpec>,
            executor: &Executor,
        ) -> Result<(SealedHeader, BundleStateWithReceipts), BlockExecutionError>
        where
            Executor: BlockExecutorProvider,
            Provider: StateProviderFactory,
        {
            let header = self.build_header_template(
                &transactions,
                &ommers,
                withdrawals.as_ref(),
                requests.as_ref(),
                chain_spec,
            );
    
            let block = Block {
                header,
                body: transactions,
                ommers: ommers.clone(),
                withdrawals: withdrawals.clone(),
                requests: requests.clone(),
            }
            .with_recovered_senders()
            .ok_or(BlockExecutionError::Validation(BlockValidationError::SenderRecoveryError))?;
    
            trace!(target: "consensus::auto", transactions=?&block.body, "executing transactions");
    
            let mut db = StateProviderDatabase::new(
                provider.latest().map_err(BlockExecutionError::LatestBlock)?,
            );
    
            // execute the block
            let BlockExecutionOutput { state, receipts, .. } =
                executor.executor(&mut db).execute((&block, U256::ZERO).into())?;
            let bundle_state = BundleStateWithReceipts::new(
                state,
                Receipts::from_block_receipt(receipts),
                block.number,
            );
    
            // todo(onbjerg): we should not pass requests around as this is building a block, which
            // means we need to extract the requests from the execution output and compute the requests
            // root here
    
            let Block { mut header, body, .. } = block.block;
            let body = BlockBody { transactions: body, ommers, withdrawals, requests };
    
            trace!(target: "consensus::auto", ?bundle_state, ?header, ?body, "executed block, calculating state root and completing header");
    
            // calculate the state root
            header.state_root = db.state_root(bundle_state.state())?;
            trace!(target: "consensus::auto", root=?header.state_root, ?body, "calculated root");
    
            // finally insert into storage
            self.insert_new_block(header.clone(), body);
    
            // set new header with hash that should have been updated by insert_new_block
            let new_header = header.seal(self.best_hash);
    
            Ok((new_header, bundle_state))
        }
    }
    
    ```
    
    - impl Executor<DB> for EthBlockExecutor<EvmConfig, DB> : execute()
    
    ```rust
    impl<EvmConfig, DB> Executor<DB> for EthBlockExecutor<EvmConfig, DB>
    where
        EvmConfig: ConfigureEvm,
        DB: Database<Error = ProviderError>,
    {
        type Input<'a> = BlockExecutionInput<'a, BlockWithSenders>;
        type Output = BlockExecutionOutput<Receipt>;
        type Error = BlockExecutionError;
    
        /// Executes the block and commits the state changes.
        ///
        /// Returns the receipts of the transactions in the block.
        ///
        /// Returns an error if the block could not be executed or failed verification.
        ///
        /// State changes are committed to the database.
        fn execute(mut self, input: Self::Input<'_>) -> Result<Self::Output, Self::Error> {,
            let BlockExecutionInput { block, total_difficulty } = input;
            let EthExecuteOutput { receipts, requests, gas_used } =
                self.execute_without_verification(block, total_difficulty)?;
    
            // NOTE: we need to merge keep the reverts for the bundle retention
            self.state.merge_transitions(BundleRetention::Reverts);
    
            Ok(BlockExecutionOutput { state: self.state.take_bundle(), receipts, requests, gas_used })
        }
    }
    ```
    
    - impl EthBlockExecutor<EvmConfig, DB>: execute_without_verification()
    
    ```
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
    
    - impl EthEvmExecutor<EvmConfig>: execute_state_transitions
        - This function is the function sequentially execute each transaction → need parallel hear
        - evm is in another repository → take it to continue
    
    ```
       /// Executes the transactions in the block and returns the receipts.
        ///
        /// This applies the pre-execution changes, and executes the transactions.
        ///
        /// # Note
        ///
        /// It does __not__ apply post-execution changes.
        fn execute_state_transitions<Ext, DB>(
            &self,
            block: &BlockWithSenders,
            mut evm: Evm<'_, Ext, &mut State<DB>>,
        ) -> Result<EthExecuteOutput, BlockExecutionError>
        where
            DB: Database<Error = ProviderError>,
        {
            // apply pre execution changes
            apply_beacon_root_contract_call(
                &self.chain_spec,
                block.timestamp,
                block.number,
                block.parent_beacon_block_root,
                &mut evm,
            )?;
    
            // execute transactions
            let mut cumulative_gas_used = 0;
            let mut receipts = Vec::with_capacity(block.body.len());
            for (sender, transaction) in block.transactions_with_sender() {
                // The sum of the transaction’s gas limit, Tg, and the gas utilized in this block prior,
                // must be no greater than the block’s gasLimit.
                let block_available_gas = block.header.gas_limit - cumulative_gas_used;
                if transaction.gas_limit() > block_available_gas {
                    return Err(BlockValidationError::TransactionGasLimitMoreThanAvailableBlockGas {
                        transaction_gas_limit: transaction.gas_limit(),
                        block_available_gas,
                    }
                    .into())
                }
    
                EvmConfig::fill_tx_env(evm.tx_mut(), transaction, *sender);
    
                // Execute transaction.
                let ResultAndState { result, state } = evm.transact().map_err(move |err| {
                    // Ensure hash is calculated for error log, if not already done
                    BlockValidationError::EVM {
                        hash: transaction.recalculate_hash(),
                        error: err.into(),
                    }
                })?;
                evm.db_mut().commit(state);
    
                // append gas used
                cumulative_gas_used += result.gas_used();
    
                // Push transaction changeset and calculate header bloom filter for receipt.
                receipts.push(
                    #[allow(clippy::needless_update)] // side-effect of optimism fields
                    Receipt {
                        tx_type: transaction.tx_type(),
                        // Success flag was added in `EIP-658: Embedding transaction status code in
                        // receipts`.
                        success: result.is_success(),
                        cumulative_gas_used,
                        // convert to reth log
                        logs: result.into_logs(),
                        ..Default::default()
                    },
                );
            }
            drop(evm);
    
            Ok(EthExecuteOutput { receipts, requests: vec![], gas_used: cumulative_gas_used })
        }
    }
    ```
    
2. Finalization: The transaction is finalized once it's included in the blockchain. The state of the blockchain is updated to reflect the transaction.

# BlockchainTree

blocktrain_tree.rs

```rust
/// # Main functions
/// * [BlockchainTree::insert_block]: Connect a block to a chain, execute it, and if valid, insert
///   the block into the tree.
/// * [BlockchainTree::finalize_block]: Remove chains that branch off of the now finalized block.
/// * [BlockchainTree::make_canonical]: Check if we have the hash of a block that is the current
///   canonical head and commit it to db.
```