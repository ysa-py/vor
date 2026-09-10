#!/usr/bin/env bash
# MICAFP UnifiedShield VIP-ULTRA — Identity Rotation Script
# Forces immediate rotation of ephemeral P2P identity via IPC.
set -euo pipefail
echo "Sending rotate-identity command to daemon..."
if command -v shield-daemon &>/dev/null; then
  echo '{"cmd":"rotate_identity"}' | nc -U /var/run/shield-daemon.sock
  echo "Identity rotated successfully."
else
  echo "Daemon not running or IPC socket unavailable."
  exit 1
fi
