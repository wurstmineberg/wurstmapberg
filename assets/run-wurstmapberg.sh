#!/bin/bash

set -e

echo "updating worlds..."
cd /opt/wurstmineberg/maps
./update_world wurstmineberg
cd /home/wurstmineberg
echo "updating Rust..."
.cargo/bin/rustup update stable
echo "updating wurstmapberg CLI..."
.cargo/bin/cargo install-update --all --git
echo "rendering map..."
.cargo/bin/wurstmapberg-cli /opt/wurstmineberg/maps/wurstmineberg/world/world
echo "done."
