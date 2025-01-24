alias t := cargo-test

defauslt:
    @just --choose

cargo-test:
    nix develop --command cargo test

cargo-clippy:
    nix develop --command cargo clippy --all-targets --all-features

cargo-tarpaulin:
    nix develop --command cargo tarpaulin --out html
