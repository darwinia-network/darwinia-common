readonly TEST_CRATES=(
    'balances'
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
