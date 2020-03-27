readonly TEST_CRATES=(
    'kton'
    'ring'
    'staking'
    'treasury'
    'eth-relay'
    'eth-backing'
    'header-mmr'
);

function main() {
    cargo build

    for crate in ${TEST_CRATES[@]}
    do
	cargo test -p "darwinia-$crate"
    done
}

main
