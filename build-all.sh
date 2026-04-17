#!/bin/sh

mkdir release-build-staging
mkdir release-build-staging/x86_64-unknown-linux-gnu
mkdir release-build-staging/aarch64-uknown-linux-gnu

set -e

cargo build --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/clearcore-flash-rs release-build-staging/x86_64-unknown-linux-gnu/clearcore-flash-rs

cargo build --release --target aarch64-unknown-linux-gnu
cp target/aarch64-unknown-linux-gnu/release/clearcore-flash-rs release-build-staging/aarch64-uknown-linux-gnu/clearcore-flash-rs

