debug:
	PATH="/usr/local/opt/llvm/bin:${PATH}" CC=/usr/local/opt/llvm/bin/clang AR=/usr/local/opt/llvm/bin/llvm-ar ${HOME}/.cargo/bin/cargo build --target wasm32-wasi
	wasm-opt -Os -o moonramp_program_test_sale.wasm target/wasm32-wasi/debug/moonramp_program_test_sale.wasm

release:
	PATH="/usr/local/opt/llvm/bin:${PATH}" CC=/usr/local/opt/llvm/bin/clang AR=/usr/local/opt/llvm/bin/llvm-ar ${HOME}/.cargo/bin/cargo build --release --target wasm32-wasi
	wasm-opt -Oz -o moonramp_program_test_sale.wasm target/wasm32-wasi/release/moonramp_program_test_sale.wasm
