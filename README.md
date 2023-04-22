# near_payment_receiver

## Quick Start

- Install [Rustup](https://rustup.rs/)
```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
- Add the wasm toolchain
```shell
rustup target add wasm32-unknown-unknown
```
- Install wasm-opt 0.110.0
```shell
cargo install --version 0.110.0 wasm-opt
```
- Build contract
```shell
./build.sh
```
- Run tests
```shell
cargo test
```