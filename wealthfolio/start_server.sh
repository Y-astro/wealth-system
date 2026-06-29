#!/bin/bash
export PATH="$HOME/.cargo/bin:$PATH"
export BUILD_TARGET=web
export WF_ENABLE_VITE_PROXY=true
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/apps/server"
set -a
source ../../.env.web
set +a
# Wipe database to ensure it uses only data given from wealthmanager local ai and doesnt save the data itself
rm -f ./db/web-dev.db*
cargo run > "$SCRIPT_DIR/server.log" 2>&1
