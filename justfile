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

perf:
  cargo test perf_snapshot -- --nocapture

clean:
  cargo clean
