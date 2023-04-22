#!/bin/bash
set -e

cd "`dirname $0`"

rustup target add wasm32-unknown-unknown
cargo build -p near_payment_receiver --target wasm32-unknown-unknown --release

cp target/wasm32-unknown-unknown/release/*.wasm ./res/

wasm-opt -O4 ./res/near_payment_receiver.wasm -o ./res/near_payment_receiver.wasm --strip-debug
