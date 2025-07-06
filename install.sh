#!/bin/sh
cargo build --profile release
cp target/release/eurydice ~/.local/bin/
