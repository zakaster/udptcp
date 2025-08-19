#!/usr/bin/env bash
set -euo pipefail
# Kills all running 'udptcp' processes
pkill -x udptcp || true
echo "Killed udptcp processes (if any)."