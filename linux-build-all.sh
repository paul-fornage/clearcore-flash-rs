#!/bin/sh

mkdir release-build-staging

set -e

cargo build --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/clearcore-flash-rs release-build-staging/clearcore-flash-rs-x86_64-unknown-linux-gnu

cargo build --release --target aarch64-unknown-linux-gnu
cp target/aarch64-unknown-linux-gnu/release/clearcore-flash-rs release-build-staging/clearcore-flash-rs-aarch64-unknown-linux-gnu
