CHAIN_ID=${CHAIN_ID:-evmos_9000-1}
/evmosd start --home /evmos --chain-id $CHAIN_ID --json-rpc.enable --json-rpc.address 0.0.0.0:8545