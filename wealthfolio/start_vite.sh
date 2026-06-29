#!/bin/bash
export BUILD_TARGET=web
export WF_ENABLE_VITE_PROXY=true
export VITE_API_TARGET=http://127.0.0.1:8080
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/apps/frontend"
npx --yes pnpm run dev
