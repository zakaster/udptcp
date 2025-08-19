#!/usr/bin/env bash
set -euo pipefail

# Notice:
#   when starting app with this script there are no terminal windows open
#
# Setup:
#   change file permission with +x to add
#   executable bit to let OS allows executing the script as program
#   chmod +x start_many.sh stop_many.sh
#   
# Usage:
#   ./start_many.sh [COUNT]
#   ./start_many.sh --release [COUNT]
#
# Examples:
#   ./start_many.sh 4
#   ./start_many.sh --release 4

COUNT="4"
PROFILE="debug"

if [[ "${1:-}" == "--release" ]]; then
  PROFILE="release"
  COUNT="${2:-4}"
else
  COUNT="${1:-4}"
fi

echo "Building ($PROFILE)..."
if [[ "$PROFILE" == "release" ]]; then
  cargo build --release
  BIN="target/release/udptcp"
else
  cargo build
  BIN="target/debug/udptcp"
fi

if [[ ! -x "$BIN" ]]; then
  echo "Binary not found: $BIN"
  exit 1
fi

echo "Launching $COUNT instances..."
pids=()
for ((i=1;i<=COUNT;i++)); do
  "$BIN" >/dev/null 2>&1 &
  pids+=($!)
done

echo "Started PIDs: ${pids[*]}"