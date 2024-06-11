# EVMOS-BUILD-GENESIS
This folder helps user build genesis file for EVMOS and generate 4 nodes ready-for-run.

## How to use
- Install `jq` on your machine. (More info)[https://stedolan.github.io/jq/download/]
- Run `create-genesis-file.sh` script: `./create-genesis-file.sh`
- Prepare `evmos/node` docker image:
    - Clone EVMOS repository: `git clone https://github.com/evmos/evmos.git`
    - Checkout to `v18.0.0` tag: `git checkout v18.0.0`
    - Build docker image: `make localnet-build`
- Run docker-compose: `docker compose up -d`