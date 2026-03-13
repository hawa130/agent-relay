set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

dist_dir := justfile_directory() + "/dist"
macos_target := "aarch64-apple-darwin"
linux_target := "x86_64-unknown-linux-gnu"

default:
    @just --list

fmt:
    cargo fmt --all
    swiftformat apps/relay-macos --config .swiftformat

fmt-check:
    cargo fmt --all --check
    swiftformat --lint apps/relay-macos --config .swiftformat

test:
    just test-rust
    just test-macos

test-rust:
    cargo test

test-macos:
    /bin/zsh -lc 'cd apps/relay-macos && HOME=$PWD/.swift-test-home SWIFTPM_MODULECACHE_OVERRIDE=$PWD/.swift-cache/swiftpm CLANG_MODULE_CACHE_PATH=$PWD/.swift-cache/clang swift test'

lint-rust:
    cargo clippy --workspace --all-targets -- -D warnings

lint-swift:
    swiftlint lint --config .swiftlint.yml

lint:
    just lint-rust
    just lint-swift

check:
    just fmt-check
    just lint
    just test

build-release:
    mkdir -p "{{dist_dir}}"
    just fmt-check
    just test-rust
    cargo build --release -p agrelay-cli --bin agrelay
    cp "target/release/agrelay" "{{dist_dir}}/agrelay"
    chmod +x "{{dist_dir}}/agrelay"
    @echo "built cli: {{dist_dir}}/agrelay"

build-cli target:
    mkdir -p "{{dist_dir}}/cli/{{target}}"
    cargo build --release -p agrelay-cli --bin agrelay --target "{{target}}"
    cp "target/{{target}}/release/agrelay" "{{dist_dir}}/cli/{{target}}/agrelay"
    chmod +x "{{dist_dir}}/cli/{{target}}/agrelay"
    @echo "built cli: {{dist_dir}}/cli/{{target}}/agrelay"

build-macos:
    just build-cli "{{macos_target}}"

build-linux:
    just build-cli "{{linux_target}}"

build-all:
    just build-macos
    just build-linux

build-app:
    ./apps/relay-macos/scripts/build-app.sh
