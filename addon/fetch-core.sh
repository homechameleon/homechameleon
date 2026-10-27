#!/bin/sh
# Place homechameleon-core where the add-on server's path dependency expects it.
# A build can pass this commit's crate in server/.core-snapshot. Otherwise the
# crate is cloned from this repository so the add-on does not carry a second copy.
set -eu
dest=/crates/homechameleon-core
snapshot=/src/server/.core-snapshot
mkdir -p /crates
rm -rf "$dest"
if [ -f "$snapshot/src/lib.rs" ]; then
  cp -a "$snapshot" "$dest"
else
  git clone --depth 1 https://github.com/homechameleon/homechameleon.git /tmp/homechameleon-src
  cp -a /tmp/homechameleon-src/crates/homechameleon-core "$dest"
fi
# The crate inherits edition and version from the workspace manifest.
cat > /crates/Cargo.toml <<'EOF'
[workspace]
resolver = "2"
members = ["homechameleon-core"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
rust-version = "1.97"
authors = ["HomeChameleon contributors"]
repository = "https://github.com/homechameleon/homechameleon"
description = "Theme generator for Home Assistant"
EOF
