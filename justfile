name := 'cosmic-randr'
rootdir := ''
prefix := '/usr'

cargo-target-dir := env('CARGO_TARGET_DIR', 'target')

bin-src := cargo-target-dir / 'release' / name
bin-dst := clean(rootdir / prefix) / 'bin' / name

default: build-release

install:
    install -Dm0755 {{bin-src}} {{bin-dst}}

# Remove Cargo build artifacts
clean:
    cargo clean

# Also remove .cargo and vendored dependencies
clean-dist: clean
    rm -rf .cargo vendor vendor.tar target

# Compile with debug profile
build-debug *args:
    cargo build {{args}}

# Compile with release profile
build-release *args: (build-debug '--release' args)

# Compile with a vendored tarball
build-vendored *args: vendor-extract (build-release '--frozen --offline' args)

# Check for errors and linter warnings
check *args:
    cargo clippy --all-features {{args}} -- -W clippy::pedantic

# Runs a check with JSON message format for IDE integration
check-json: (check '--message-format=json')

# Run the application for testing purposes
run *args:
    env RUST_LOG=debug RUST_BACKTRACE=full cargo run --release {{args}}

# Run `cargo test`
test:
    cargo test

# Vendor Cargo dependencies locally
vendor:
    mkdir -p .cargo
    cargo vendor --sync Cargo.toml \
        | head -n -1 > .cargo/config
    echo 'directory = "vendor"' >> .cargo/config
    tar pcf vendor.tar vendor
    rm -rf vendor

# Extracts vendored dependencies
[private]
vendor-extract:
    rm -rf vendor
    tar pxf vendor.tar
