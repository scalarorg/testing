#!/bin/bash

# Set the required variables
CHAIN=evmos
CHAIND=${CHAIN}d
KEYRING=test
KEYALGO=eth_secp256k1
MNEMONIC="orphan assume tent east october arena empower light notice border scatter off thought hawk link lion coconut void huge elegant crucial decline adjust pride"
NUM_ADDRESSES=10  # Number of addresses to generate
HOMEDIR=$(pwd)/$CHAIND

$CHAIND keys list --keyring-backend $KEYRING --home $HOMEDIR | grep name | awk '{print $2}' | while read -r key_name; do
    $CHAIND keys delete "$key_name" --keyring-backend $KEYRING --home $HOMEDIR -y
done

# Import keys from the mnemonic
for i in $(seq 0 $((NUM_ADDRESSES - 1))); do
    echo "$MNEMONIC" | $CHAIND keys add "key_$i" --recover --keyring-backend $KEYRING --algo $KEYALGO --home $HOMEDIR --index $i
done

for i in $(seq 0 $((NUM_ADDRESSES - 1))); do
    $CHAIND add-genesis-account "$($CHAIND keys show "key_$i" -a --keyring-backend $KEYRING --home $HOMEDIR)" 100000000000000000000000000aevmos --keyring-backend $KEYRING --home $HOMEDIR
done

