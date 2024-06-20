# EVMOS-BUILD-GENESIS
This folder helps user build genesis file for EVMOS and generate 4 nodes ready-for-run.

## How to use
- Install `jq` on your machine. (More info)[https://stedolan.github.io/jq/download/]
- Prepare `evmos/node` docker image:
    - Clone EVMOS repository: `git clone https://github.com/evmos/evmos.git`
    - Checkout to `v18.0.0` tag: `git checkout v18.0.0`
    - Build docker image: `make localnet-build`
- Copy this folder to evmos repository
- cd evmos_build_env
- Run `evmos-create-genesis-file.sh` script: `./evmos-create-genesis-file.sh`
- Run docker-compose: `docker-compose -f evmos-docker-compose.yml up -d`

## Prepare genesis file for Scalar testnet (evmos executor)
- Create the addresses map from hex addresses in address_manager.json file to evmos addresses
```bash
npm install
node convert_hex_to_evmos_addresses.js
```

- Create the genesis file
```bash
./testnet-evmos-scalar-create-genesis-file.sh
```