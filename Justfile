# Builds all the components for the dapp
build: _build-substrate-node _build-voting-contract _build-upgraded-voting-contract

# Builds substrate node
_build-substrate-node:
    - cd substrate-contracts-node && cargo build --release

# Builds voting smart contract
_build-voting-contract:
    - cd contracts/voting_contract && cargo contract build --release

# Builds upgraded voting smart contract
_build-upgraded-voting-contract:
    - cd contracts/upgraded_voting_contract && cargo contract build --release

# Tests all the components for the dapp
test: _test-voting-contract _test-upgraded-voting-contract

# Tests voting smart contract
_test-voting-contract:
    - cd contracts/voting_contract && cargo test

# Tests upgraded voting smart contract
_test-upgraded-voting-contract:
    - cd contracts/upgraded_voting_contract && cargo test

# Runs substrate node
run-substrate-node: _build-substrate-node
    - ./substrate-contracts-node/target/release/substrate-contracts-node

# Removes all the blockchain data from previous runs
purge-substrate-node: _build-substrate-node
    - ./substrate-contracts-node/target/release/substrate-contracts-node purge-chain -y

# Builds and deploys voting smart contract to the blockchain (a local blockchain node must be running for this to succeed)
deploy-voting-contract: _build-voting-contract _upload-voting-contract _instantiate-voting-contract

# Uploads the voting smart contract to the blockchain
_upload-voting-contract:
    - cargo contract upload --execute --suri //Alice ./contracts/voting_contract/target/ink/voting_contract.contract

# Instantiates uploded voting contract on the blockchain
_instantiate-voting-contract:
    - cargo contract instantiate --execute --constructor default --skip-confirm --output-json --suri //Alice ./contracts/voting_contract/target/ink/voting_contract.json > ./instantiated-voting-contract.json

# Runs all the steps necessary to upgrade voting smart contract (a local blockchain node must be running with deployed voting contract for this to succeed)
upgrade-voting-contract: _build-upgraded-voting-contract _upload-upgraded-voting-contract _instantiate-upgraded-voting-contract

# Uploads upgraded voting smart contract
_upload-upgraded-voting-contract:
    - cargo contract upload --execute --suri //Alice ./contracts/upgraded_voting_contract/target/ink/upgraded_voting_contract.contract

# Instantiates upgraded voting smart contract
_instantiate-upgraded-voting-contract:
    - cargo contract call --contract $(jql '"contract"' ./instantiated-voting-contract.json --raw-string) --message set_code --args $(jql '"source"' ./contracts/upgraded_voting_contract/target/ink/upgraded_voting_contract.json --raw-string | jql '"hash"' --raw-string) --execute --skip-confirm --suri //Alice ./contracts/voting_contract/target/ink/voting_contract.json
