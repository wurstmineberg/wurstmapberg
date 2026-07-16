#!/bin/bash

set -e

echo "cleaning up old cargo-install tempfiles..."
find /tmp '!' -readable -prune -o -name 'cargo-install*' -type d -mtime +3 -exec rm -rf '{}' +
echo "updating worlds..."
cd /opt/wurstmineberg/maps
/opt/git/github.com/wurstmineberg/wurstmapberg/main/assets/update_world.sh wurstmineberg
cd /home/wurstmineberg
echo "updating Rust..."
.cargo/bin/rustup update stable
echo "updating wurstmapberg CLI..."
.cargo/bin/cargo install-update --all --git
echo "rendering map..."
.cargo/bin/wurstmapberg-cli /opt/wurstmineberg/maps/wurstmineberg/world/world
echo "done."
