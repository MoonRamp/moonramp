# Source

To install from source:

## Step 1 - Rust

[Install Rust](https://www.rust-lang.org/tools/install)

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Step 2 - Get the source

### 2a - MoonRamp
```
git clone git@github.com:MoonRamp/moonramp.git
```

### 2b - Required libaries

Please install `sqlite-dev` and `mysql-dev` packages from your systems package manager. MoonRamp uses [rust-tls](https://github.com/rustls/rustls) and as such `openssl-dev` is not needed.

## Step 3 - Build bins

```
cd moonramp && cargo build --release
```

## Step 4 - Build WASM programs

```
cargo --manifest-path=programs/default-sale/Cargo.toml build --release --target=wasm32-wasi
```

### Notes

MacOS DOES NOT ship with a WASM supported version of clang. Please install clang via homebrew and set `CC` and `AR` env vars.

#### Example

```
PATH="/usr/local/opt/llvm/bin:${PATH}" CC=/usr/local/opt/llvm/bin/clang AR=/usr/local/opt/llvm/bin/llvm-ar cargo [COMMAND]
```

