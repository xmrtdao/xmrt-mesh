#!/bin/bash
set -e
# Run on a machine with >5GB free space (NOT on phone)
# Verifies xmrt-mesh builds cleanly

echo "Cloning xmrt-mesh..."
TMP=$(mktemp -d)
git clone --depth 1 https://github.com/xmrtdao/xmrt-mesh.git "$TMP/xmrt-mesh"
cd "$TMP/xmrt-mesh"

echo "Checking toolchain..."
rustc --version
cargo --version

echo "Building release..."
cargo build --release 2>&1 | tee build.log

echo "Binary size:"
ls -lh target/release/xmrt-mesh 2>/dev/null || ls -lh target/release/

echo "Done. Binary at target/release/xmrt-mesh"
