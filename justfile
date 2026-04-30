set shell := ["bash", "-cu"]

default:
  @just --list

dev:
  trunk serve

build:
  trunk build --release

check:
  cargo check --target wasm32-unknown-unknown

fmt:
  cargo fmt

lint:
  cargo clippy --target wasm32-unknown-unknown -- -D warnings

test:
  cargo test

browser-test:
  wasm-pack test --headless --firefox

browser-test-chrome:
  wasm-pack test --headless --chrome

clean:
  cargo clean
