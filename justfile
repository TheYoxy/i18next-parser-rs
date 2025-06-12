bench directory:
    RUSTFLAGS="-C force-frame-pointers=yes" CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph --bin i18next-parser -- {{directory}} -g

test:
    RUST_LOG=trace cargo test
