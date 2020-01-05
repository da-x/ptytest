#!/bin/bash

set -e

cargo build --example ptytest-check_me
cargo run --example ptytest-tester -- ./target/debug/examples/ptytest-check_me
