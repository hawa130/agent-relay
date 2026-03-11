set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

dist_dir := justfile_directory() + "/dist"
macos_target := "aarch64-apple-darwin"
linux_target := "x86_64-unknown-linux-gnu"

default:
    @just --list

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

test:
    just test-rust
    just test-macos

test-rust:
    cargo test

test-macos:
    /bin/zsh -lc 'cd apps/relay-macos && HOME=$PWD/.swift-test-home SWIFTPM_MODULECACHE_OVERRIDE=$PWD/.swift-cache/swiftpm CLANG_MODULE_CACHE_PATH=$PWD/.swift-cache/clang swift test'

release:
    mkdir -p "{{dist_dir}}"
    just fmt-check
    just test-rust
    cargo build --release -p relay-cli --bin relay
    cp "target/release/relay" "{{dist_dir}}/relay"
    chmod +x "{{dist_dir}}/relay"
    @echo "built cli: {{dist_dir}}/relay"

cli target:
    mkdir -p "{{dist_dir}}/cli/{{target}}"
    cargo build --release -p relay-cli --bin relay --target "{{target}}"
    cp "target/{{target}}/release/relay" "{{dist_dir}}/cli/{{target}}/relay"
    chmod +x "{{dist_dir}}/cli/{{target}}/relay"
    @echo "built cli: {{dist_dir}}/cli/{{target}}/relay"

macos:
    just cli "{{macos_target}}"

linux:
    just cli "{{linux_target}}"

all:
    just macos
    just linux

app:
    ./apps/relay-macos/scripts/build-app.sh
