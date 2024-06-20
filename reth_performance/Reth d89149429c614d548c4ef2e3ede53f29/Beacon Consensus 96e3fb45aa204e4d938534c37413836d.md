# Beacon Consensus

Here is a step-by-step guide to the tasks involved in the Beacon Consensus Engine in Reth:

## **Step 1: Initialize the Beacon Consensus Engine**

1. **Initialization**: The Beacon Consensus Engine is initialized by creating an instance of the **`BeaconConsensusEngine`** struct in the **`reth::beacon_consensus`** module.

## **Step 2: Receive Forkchoice Updated Messages**

1. **Forkchoice Updated Messages**: The Beacon Consensus Engine waits for **`BeaconEngineMessage::ForkchoiceUpdated`** messages from the consensus client. This message initiates the sync process and triggers the engine to start processing blocks.

## **Step 3: Process Blocks**

1. **Block Processing**: The Beacon Consensus Engine processes each block by executing the following steps:
    - **Propose Block**: The engine proposes a new block by selecting a validator and assigning it the responsibility of proposing a new block.
    - **Attest Block**: The engine validates the proposed block by checking its validity and ensuring that it is correctly formatted.
    - **Finalize Block**: The engine finalizes the block by adding it to the blockchain and updating the state of the blockchain.

## **Step 4: Update State**

1. **State Update**: The Beacon Consensus Engine updates the state of the blockchain by executing the transactions in the latest block. This involves updating the world state, transaction trie, and receipts trie.

## **Step 5: Handle Epochs**

1. **Epoch Handling**: The Beacon Consensus Engine handles epochs by processing a bundle of 32 slots. Each epoch represents a complete round of the POS protocol and determines the schedule of events on the blockchain network.

## **Step 6: Finalize Epochs**

1. **Epoch Finalization**: The Beacon Consensus Engine finalizes epochs by justifying and then finalizing previous blocks. This ensures that the blockchain remains consistent and secure.

## **Step 7: Repeat the Process**

1. **Repeat**: The Beacon Consensus Engine repeats the process of receiving forkchoice updated messages, processing blocks, updating state, handling epochs, and finalizing epochs to maintain the integrity and consistency of the Ethereum blockchain.

These steps provide a detailed overview of the tasks involved in the Beacon Consensus Engine in Reth.

```rust
/// On initialization, the consensus engine will poll the message receiver and return
/// [Poll::Pending] until the first forkchoice update message is received.
///
/// As soon as the consensus engine receives the first forkchoice updated message and updates the
/// local forkchoice state, it will launch the pipeline to sync to the head hash.
/// While the pipeline is syncing, the consensus engine will keep processing messages from the
/// receiver and forwarding them to the blockchain tree.
impl<DB, BT, Client, EngineT> Future for BeaconConsensusEngine<DB, BT, Client, EngineT>
where
    DB: Database + Unpin + 'static,
    Client: HeadersClient + BodiesClient + Clone + Unpin + 'static,
    BT: BlockchainTreeEngine
        + BlockReader
        + BlockIdReader
        + CanonChainTracker
        + StageCheckpointReader
        + ChainSpecProvider
        + Unpin
        + 'static,
    EngineT: EngineTypes + Unpin + 'static,
{
    type Output = Result<(), BeaconConsensusEngineError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        // Control loop that advances the state
        'main: loop {
            // Poll a running hook with db write access (if any) and CL messages first, draining
            // both and then proceeding to polling other parts such as SyncController and hooks.
            loop {
                // Poll a running hook with db write access first, as we will not be able to process
                // any engine messages until it's finished.
                if let Poll::Ready(result) =
                    this.hooks.poll_active_db_write_hook(cx, this.current_engine_hook_context()?)?
                {
                    this.on_hook_result(result)?;
                    continue
                }

                // Process any blockchain tree action result as set forth during engine message
                // processing.
                if let Some(action) = this.blockchain_tree_action.take() {
                    match this.on_blockchain_tree_action(action) {
                        Ok(EngineEventOutcome::Processed) => {}
                        Ok(EngineEventOutcome::ReachedMaxBlock) => return Poll::Ready(Ok(())),
                        Err(error) => {
                            error!(target: "consensus::engine", %error, "Encountered fatal error");
                            return Poll::Ready(Err(error.into()))
                        }
                    };

                    // Blockchain tree action handler might set next action to take.
                    continue
                }

                // Process one incoming message from the CL. We don't drain the messages right away,
                // because we want to sneak a polling of running hook in between them.
                //
                // These messages can affect the state of the SyncController and they're also time
                // sensitive, hence they are polled first.
                if let Poll::Ready(Some(msg)) = this.engine_message_stream.poll_next_unpin(cx) {
                    match msg {
                        BeaconEngineMessage::ForkchoiceUpdated { state, payload_attrs, tx } => {
                            this.on_forkchoice_updated(state, payload_attrs, tx);
                        }
                        BeaconEngineMessage::NewPayload { payload, cancun_fields, tx } => {
                            this.on_new_payload(payload, cancun_fields, tx);
                        }
                        BeaconEngineMessage::TransitionConfigurationExchanged => {
                            this.blockchain.on_transition_configuration_exchanged();
                        }
                    }
                    continue
                }

                // Both running hook with db write access and engine messages are pending,
                // proceed to other polls
                break
            }

            // process sync events if any
            if let Poll::Ready(sync_event) = this.sync.poll(cx) {
                match this.on_sync_event(sync_event)? {
                    // Sync event was successfully processed
                    EngineEventOutcome::Processed => (),
                    // Max block has been reached, exit the engine loop
                    EngineEventOutcome::ReachedMaxBlock => return Poll::Ready(Ok(())),
                }

                // this could have taken a while, so we start the next cycle to handle any new
                // engine messages
                continue 'main
            }

            // at this point, all engine messages and sync events are fully drained

            // Poll next hook if all conditions are met:
            // 1. Engine and sync messages are fully drained (both pending)
            // 2. Latest FCU status is not INVALID
            if !this.forkchoice_state_tracker.is_latest_invalid() {
                if let Poll::Ready(result) = this.hooks.poll_next_hook(
                    cx,
                    this.current_engine_hook_context()?,
                    this.sync.is_pipeline_active(),
                )? {
                    this.on_hook_result(result)?;

                    // ensure we're polling until pending while also checking for new engine
                    // messages before polling the next hook
                    continue 'main
                }
            }

            // incoming engine messages and sync events are drained, so we can yield back
            // control
            return Poll::Pending
        }
    }
}
```

# References

[execution-apis/src/engine at main · ethereum/execution-apis (github.com)](https://github.com/ethereum/execution-apis/tree/main/src/engine)