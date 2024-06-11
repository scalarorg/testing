#!/bin/bash
CHAIN=${CHAIN:-evmos}
CHAIN_ID=${CHAIN_ID:-${CHAIN}_9000-1}
CHAIND=${CHAIN}d
HOMEDIR=$(pwd)/$CHAIND
OUTPUTDIR=$(pwd)/build
KEYRING=test
KEYALGO=eth_secp256k1
MONIKER=orchestrator
# validator-scalar1 address 0xc6fe5d33615a1c52c08018c47e8bc53646a0e101 | evmos1cml96vmptgw99syqrrz8az79xer2pcgp84pdun
GENESIS_VAL1_KEY=validator-scalar1
GENESIS_VAL1_MNEMONIC="copper push brief egg scan entry inform record adjust fossil boss egg comic alien upon aspect dry avoid interest fury window hint race symptom"
# validator-scalar2 address 0x963ebdf2e1f8db8707d05fc75bfeffba1b5bac17 | evmos1jcltmuhplrdcwp7stlr4hlhlhgd4htqh3a79sq
GENESIS_VAL2_KEY=validator-scalar2
GENESIS_VAL2_MNEMONIC="maximum display century economy unlock van census kite error heart snow filter midnight usage egg venture cash kick motor survey drastic edge muffin visual"
# validator-scalar3 address 0x40a0cb1C63e026A81B55EE1308586E21eec1eFa9 | evmos1gzsvk8rruqn2sx64acfsskrwy8hvrmafqkaze8
GENESIS_VAL3_KEY="validator-scalar3"
GENESIS_VAL3_MNEMONIC="will wear settle write dance topic tape sea glory hotel oppose rebel client problem era video gossip glide during yard balance cancel file rose"
# validator-scalar4 address 0x498B5AeC5D439b733dC2F58AB489783A23FB26dA | evmos1fx944mzagwdhx0wz7k9tfztc8g3lkfk6rrgv6l
GENESIS_VAL4_KEY="validator-scalar4"
GENESIS_VAL4_MNEMONIC="doll midnight silk carpet brush boring pluck office gown inquiry duck chief aim exit gain never tennis crime fragile ship cloud surface exotic patch"
# user1 address 0x7cb61d4117ae31a12e393a1cfa3bac666481d02e | evmos10jmp6sgh4cc6zt3e8gw05wavvejgr5pwjnpcky
USER1_KEY="user1"
USER1_MNEMONIC="gesture inject test cycle original hollow east ridge hen combine junk child bacon zero hope comfort vacuum milk pitch cage oppose unhappy lunar seat"
# user2 address 0xCC1E8f1732F7c9FC4Ed046dC7eB8c64Ad412Ed6D | evmos1es0g79ej7lylcnksgmw8awxxft2p9mtdx27lgr
USER2_KEY="user2"
USER2_MNEMONIC="coil ship indoor sadness luxury evolve timber ring urban multiply deny fall retire cement online write ugly forget hazard live wedding rookie among camp"
# user3 address | evmos1gtzwfmmtkyrfwn8hn9cyp07vf7f25dj64ftgwm
USER3_KEY="user3"
USER3_MNEMONIC="cloth page safe pistol brain coconut ivory wasp stable dinner wreck isolate garment gift exclude oblige tiger doctor will banner barrel helmet course once"
# user4 address | evmos1etz7wl4v4kzkyqa45x6yd79728jkzun0jzh0q9
USER4_KEY="user4"
USER4_MNEMONIC="multiply stuff buzz seed resist clinic enable spare lab wink notice cattle clean disease sister lesson salon best velvet ramp rare sunset what tide"

LOGLEVEL="info"
# to trace evm
#TRACE="--trace"
TRACE=""

# feemarket params basefee
BASEFEE=1000000000

# Path variables
CONFIG=$HOMEDIR/config/config.toml
APP_TOML=$HOMEDIR/config/app.toml
GENESIS=$HOMEDIR/config/genesis.json
TMP_GENESIS=$HOMEDIR/config/tmp_genesis.json

# validate dependencies are installed
command -v jq >/dev/null 2>&1 || {
	echo >&2 "jq not installed. More info: https://stedolan.github.io/jq/download/"
	exit 1
}

# used to exit on first error (any non-zero exit code)
set -e

# delete existing homedir
rm -rf $HOMEDIR
rm -rf $OUTPUTDIR

# set client config
$CHAIND config keyring-backend $KEYRING --home $HOMEDIR
$CHAIND config chain-id $CHAIN_ID --home "$HOMEDIR"

# import keys from mnemonics
echo $USER1_MNEMONIC | $CHAIND keys add $USER1_KEY --recover --keyring-backend $KEYRING --algo $KEYALGO --home $HOMEDIR
echo $USER2_MNEMONIC | $CHAIND keys add $USER2_KEY --recover --keyring-backend $KEYRING --algo $KEYALGO --home $HOMEDIR
echo $USER3_MNEMONIC | $CHAIND keys add $USER3_KEY --recover --keyring-backend $KEYRING --algo $KEYALGO --home $HOMEDIR
echo $USER4_MNEMONIC | $CHAIND keys add $USER4_KEY --recover --keyring-backend $KEYRING --algo $KEYALGO --home $HOMEDIR
echo $GENESIS_VAL1_MNEMONIC | $CHAIND keys add $GENESIS_VAL1_KEY --recover --keyring-backend $KEYRING --algo $KEYALGO --home $HOMEDIR
echo $GENESIS_VAL2_MNEMONIC | $CHAIND keys add $GENESIS_VAL2_KEY --recover --keyring-backend $KEYRING --algo $KEYALGO --home $HOMEDIR
echo $GENESIS_VAL3_MNEMONIC | $CHAIND keys add $GENESIS_VAL3_KEY --recover --keyring-backend $KEYRING --algo $KEYALGO --home $HOMEDIR
echo $GENESIS_VAL4_MNEMONIC | $CHAIND keys add $GENESIS_VAL4_KEY --recover --keyring-backend $KEYRING --algo $KEYALGO --home $HOMEDIR

# Store the validator address in a variable to use it later
val1_address=$(evmosd keys show -a "$GENESIS_VAL1_KEY" --keyring-backend "$KEYRING" --home "$HOMEDIR")
val2_address=$(evmosd keys show -a "$GENESIS_VAL2_KEY" --keyring-backend "$KEYRING" --home "$HOMEDIR")
val3_address=$(evmosd keys show -a "$GENESIS_VAL3_KEY" --keyring-backend "$KEYRING" --home "$HOMEDIR")
val4_address=$(evmosd keys show -a "$GENESIS_VAL4_KEY" --keyring-backend "$KEYRING" --home "$HOMEDIR")

# Set moniker and chain-id for Evmos (Moniker can be anything, chain-id must be an integer)
$CHAIND init $MONIKER -o --chain-id "$CHAIN_ID" --home "$HOMEDIR"

# Change parameter token denominations to aevmos
jq '.app_state["staking"]["params"]["bond_denom"]="aevmos"' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"
jq '.app_state["crisis"]["constant_fee"]["denom"]="aevmos"' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"
jq '.app_state["gov"]["deposit_params"]["min_deposit"][0]["denom"]="aevmos"' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"
# When upgrade to cosmos-sdk v0.47, use gov.params to edit the deposit params
jq '.app_state["gov"]["params"]["min_deposit"][0]["denom"]="aevmos"' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"
jq '.app_state["evm"]["params"]["evm_denom"]="aevmos"' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"
jq '.app_state["inflation"]["params"]["mint_denom"]="aevmos"' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"

# Set gas limit in genesis
jq '.consensus_params["block"]["max_gas"]="10000000"' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"

# Set claims start time
current_date=$(date -u +"%Y-%m-%dT%TZ")
jq -r --arg current_date "$current_date" '.app_state["claims"]["params"]["airdrop_start_time"]=$current_date' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"

# Set claims records for validator account
amount_to_claim=10000
jq -r --arg val1_address "$val1_address" --arg amount_to_claim "$amount_to_claim" '.app_state["claims"]["claims_records"]=[{"initial_claimable_amount":$amount_to_claim, "actions_completed":[false, false, false, false],"address":$val1_address}]' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"
jq -r --arg val2_address "$val2_address" --arg amount_to_claim "$amount_to_claim" '.app_state["claims"]["claims_records"]=[{"initial_claimable_amount":$amount_to_claim, "actions_completed":[false, false, false, false],"address":$val2_address}]' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"
jq -r --arg val3_address "$val3_address" --arg amount_to_claim "$amount_to_claim" '.app_state["claims"]["claims_records"]=[{"initial_claimable_amount":$amount_to_claim, "actions_completed":[false, false, false, false],"address":$val3_address}]' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"
jq -r --arg val4_address "$val4_address" --arg amount_to_claim "$amount_to_claim" '.app_state["claims"]["claims_records"]=[{"initial_claimable_amount":$amount_to_claim, "actions_completed":[false, false, false, false],"address":$val4_address}]' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"

# Set claims decay
jq '.app_state["claims"]["params"]["duration_of_decay"]="1000000s"' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"
jq '.app_state["claims"]["params"]["duration_until_decay"]="100000s"' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"

# Claim module account:
# 0xA61808Fe40fEb8B3433778BBC2ecECCAA47c8c47 || evmos15cvq3ljql6utxseh0zau9m8ve2j8erz89m5wkz
jq -r --arg amount_to_claim "$amount_to_claim" '.app_state["bank"]["balances"] += [{"address":"evmos15cvq3ljql6utxseh0zau9m8ve2j8erz89m5wkz","coins":[{"denom":"aevmos", "amount":$amount_to_claim}]}]' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"

# Set base fee in genesis
jq '.app_state["feemarket"]["params"]["base_fee"]="'${BASEFEE}'"' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"

if [[ $1 == "pending" ]]; then
	if [[ "$OSTYPE" == "darwin"* ]]; then
		sed -i '' 's/timeout_propose = "3s"/timeout_propose = "30s"/g' "$CONFIG"
		sed -i '' 's/timeout_propose_delta = "500ms"/timeout_propose_delta = "5s"/g' "$CONFIG"
		sed -i '' 's/timeout_prevote = "1s"/timeout_prevote = "10s"/g' "$CONFIG"
		sed -i '' 's/timeout_prevote_delta = "500ms"/timeout_prevote_delta = "5s"/g' "$CONFIG"
		sed -i '' 's/timeout_precommit = "1s"/timeout_precommit = "10s"/g' "$CONFIG"
		sed -i '' 's/timeout_precommit_delta = "500ms"/timeout_precommit_delta = "5s"/g' "$CONFIG"
		sed -i '' 's/timeout_commit = "5s"/timeout_commit = "150s"/g' "$CONFIG"
		sed -i '' 's/timeout_broadcast_tx_commit = "10s"/timeout_broadcast_tx_commit = "150s"/g' "$CONFIG"
	else
		sed -i 's/timeout_propose = "3s"/timeout_propose = "30s"/g' "$CONFIG"
		sed -i 's/timeout_propose_delta = "500ms"/timeout_propose_delta = "5s"/g' "$CONFIG"
		sed -i 's/timeout_prevote = "1s"/timeout_prevote = "10s"/g' "$CONFIG"
		sed -i 's/timeout_prevote_delta = "500ms"/timeout_prevote_delta = "5s"/g' "$CONFIG"
		sed -i 's/timeout_precommit = "1s"/timeout_precommit = "10s"/g' "$CONFIG"
		sed -i 's/timeout_precommit_delta = "500ms"/timeout_precommit_delta = "5s"/g' "$CONFIG"
		sed -i 's/timeout_commit = "5s"/timeout_commit = "150s"/g' "$CONFIG"
		sed -i 's/timeout_broadcast_tx_commit = "10s"/timeout_broadcast_tx_commit = "150s"/g' "$CONFIG"
	fi
fi

# enable prometheus metrics and all APIs for dev node
if [[ "$OSTYPE" == "darwin"* ]]; then
	sed -i '' 's/prometheus = false/prometheus = true/' "$CONFIG"
	sed -i '' 's/prometheus-retention-time = 0/prometheus-retention-time  = 1000000000000/g' "$APP_TOML"
	sed -i '' 's/enabled = false/enabled = true/g' "$APP_TOML"
	sed -i '' 's/enable = false/enable = true/g' "$APP_TOML"
	# Don't enable memiavl by default
	grep -q -F '[memiavl]' "$APP_TOML" && sed -i '' '/\[memiavl\]/,/^\[/ s/enable = true/enable = false/' "$APP_TOML"
else
	sed -i 's/prometheus = false/prometheus = true/' "$CONFIG"
	sed -i 's/prometheus-retention-time  = "0"/prometheus-retention-time  = "1000000000000"/g' "$APP_TOML"
	sed -i 's/enabled = false/enabled = true/g' "$APP_TOML"
	sed -i 's/enable = false/enable = true/g' "$APP_TOML"
	# Don't enable memiavl by default
	grep -q -F '[memiavl]' "$APP_TOML" && sed -i '/\[memiavl\]/,/^\[/ s/enable = true/enable = false/' "$APP_TOML"
fi

# Change proposal periods to pass within a reasonable time for local testing
sed -i.bak 's/"max_deposit_period": "172800s"/"max_deposit_period": "30s"/g' "$GENESIS"
sed -i.bak 's/"voting_period": "172800s"/"voting_period": "30s"/g' "$GENESIS"

# set custom pruning settings
sed -i.bak 's/pruning = "default"/pruning = "custom"/g' "$APP_TOML"
sed -i.bak 's/pruning-keep-recent = "0"/pruning-keep-recent = "2"/g' "$APP_TOML"
sed -i.bak 's/pruning-interval = "0"/pruning-interval = "10"/g' "$APP_TOML"

# allocate genesis accounts (cosmos formatted addresses)
$CHAIND add-genesis-account "$($CHAIND keys show $USER1_KEY -a --keyring-backend $KEYRING --home $HOMEDIR)" 100000000000000000000000000aevmos --keyring-backend $KEYRING --home $HOMEDIR
$CHAIND add-genesis-account "$($CHAIND keys show $USER2_KEY -a --keyring-backend $KEYRING --home $HOMEDIR)" 100000000000000000000000000aevmos --keyring-backend $KEYRING --home $HOMEDIR
$CHAIND add-genesis-account "$($CHAIND keys show $USER3_KEY -a --keyring-backend $KEYRING --home $HOMEDIR)" 100000000000000000000000000aevmos --keyring-backend $KEYRING --home $HOMEDIR
$CHAIND add-genesis-account "$($CHAIND keys show $USER4_KEY -a --keyring-backend $KEYRING --home $HOMEDIR)" 100000000000000000000000000aevmos --keyring-backend $KEYRING --home $HOMEDIR
$CHAIND add-genesis-account "$($CHAIND keys show $GENESIS_VAL1_KEY -a --keyring-backend $KEYRING --home $HOMEDIR)" 100000000000000000000000000aevmos --keyring-backend $KEYRING --home $HOMEDIR
$CHAIND add-genesis-account "$($CHAIND keys show $GENESIS_VAL2_KEY -a --keyring-backend $KEYRING --home $HOMEDIR)" 100000000000000000000000000aevmos --keyring-backend $KEYRING --home $HOMEDIR
$CHAIND add-genesis-account "$($CHAIND keys show $GENESIS_VAL3_KEY -a --keyring-backend $KEYRING --home $HOMEDIR)" 100000000000000000000000000aevmos --keyring-backend $KEYRING --home $HOMEDIR
$CHAIND add-genesis-account "$($CHAIND keys show $GENESIS_VAL4_KEY -a --keyring-backend $KEYRING --home $HOMEDIR)" 100000000000000000000000000aevmos --keyring-backend $KEYRING --home $HOMEDIR

# bc is required to add these big numbers
# NOTE: we have the validator account (1e26) plus 4 (1e21) accounts
#       plus the claimed amount (1e4)
total_supply=800000000000000000000010000
jq -r --arg total_supply "$total_supply" '.app_state["bank"]["supply"][0]["amount"]=$total_supply' "$GENESIS" >"$TMP_GENESIS" && mv "$TMP_GENESIS" "$GENESIS"

# Sign genesis transaction
## In case you want to create multiple validators at genesis
## 1. Back to `evmosd keys add` step, init more keys
## 2. Back to `evmosd add-genesis-account` step, add balance for those
## 3. Clone this ~/.evmosd home directory into some others, let's say `~/.clonedEvmosd`
## 4. Run `gentx` in each of those folders
## 5. Copy the `gentx-*` folders under `~/.clonedEvmosd/config/gentx/` folders into the original `~/.evmosd/config/gentx`

## Clone the $HOMEDIR directory to into 4 other directories in $OUTPUTDIR, 
## let's say $OUTPUTDIR/node1/$CHAIND, $OUTPUTDIR/node2/$CHAIND, $OUTPUTDIR/node3/$CHAIND, $OUTPUTDIR/node4/$CHAIND

GENTX_DIR=$OUTPUTDIR/gentxs
mkdir -p $GENTX_DIR
for i in 1 2 3 4
do
	# new node directory
	NODE_MONIKER="node$i"
	NODE_DIR=$OUTPUTDIR/$NODE_MONIKER
	BUILD_DIR=$NODE_DIR/$CHAIND

	# copy the evmosd directory
	mkdir -p $NODE_DIR
	cp -r $HOMEDIR $NODE_DIR

	# remove duplicate key files, let the node create them by gentx command
	rm -rf $BUILD_DIR/config/priv_validator_key.json
	rm -rf $BUILD_DIR/config/node_key.json

	# update moniker and seeds in config.toml
	NODE_CONFIG=$BUILD_DIR/config/config.toml
	MONIKER_PATTERN='s/moniker = "'$MONIKER'"/moniker = "'$NODE_MONIKER'"/g'
	SEEDS_PATTERN='s/seeds = /seeds = "" #/g'
	if [[ "$OSTYPE" == "darwin"* ]]; then
		sed -i '' "$MONIKER_PATTERN" "$NODE_CONFIG"
		sed -i '' "$SEEDS_PATTERN" "$NODE_CONFIG"
	else
		sed -i "$MONIKER_PATTERN" "$NODE_CONFIG"
		sed -i "$SEEDS_PATTERN" "$NODE_CONFIG"
	fi

	# sign genesis transaction
	KEYNAME="GENESIS_VAL${i}_KEY"
	$CHAIND gentx ${!KEYNAME} 1000000000000000000000aevmos --gas-prices ${BASEFEE}aevmos --keyring-backend "$KEYRING" --chain-id "$CHAIN_ID" --home "$BUILD_DIR" --moniker $NODE_MONIKER --ip 192.167.10.$((1 + $i))

	# move the gentx file to the gentx directory
	cp $BUILD_DIR/config/gentx/* $GENTX_DIR/$NODE_MONIKER.json

	# remove unnecessary files
	rm -rf $BUILD_DIR/config/gentx
	rm -rf $BUILD_DIR/config/app.toml.bak
	rm -rf $BUILD_DIR/config/genesis.json.bak
done

# Collect genesis tx
$CHAIND collect-gentxs --home "$HOMEDIR" --gentx-dir $GENTX_DIR

# Run this to ensure everything worked and that the genesis file is setup correctly
$CHAIND validate-genesis --home "$HOMEDIR"

for i in 1 2 3 4
do
	NODE_DIR=$OUTPUTDIR/node$i
	# Collect genesis tx
	$CHAIND collect-gentxs --home "$NODE_DIR/$CHAIND" --gentx-dir $GENTX_DIR
	# Run this to ensure everything worked and that the genesis file is setup correctly
	$CHAIND validate-genesis --home "$NODE_DIR/$CHAIND"
done

# Copy the genesis file to the output directory
cp $GENESIS $OUTPUTDIR/genesis.json

# remove the $HOMEDIR directory after the genesis file is created
rm -rf $HOMEDIR

