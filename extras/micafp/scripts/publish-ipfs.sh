#!/usr/bin/env bash
# ══════════════════════════════════════════════════════════════
# MICAFP-UnifiedShield v6.0 — IPFS Config Publisher
# Publishes signed config updates to IPFS for distribution
# ══════════════════════════════════════════════════════════════

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_DIR="$SCRIPT_DIR/configs"

# Ed25519 signing key (MUST be set as environment variable)
SIGNING_KEY="${RELEASE_SIGNING_KEY:-}"

if [ -z "$SIGNING_KEY" ]; then
    echo "ERROR: RELEASE_SIGNING_KEY environment variable not set"
    echo "Generate a key: openssl genpkey -algorithm ED25519 -out signing.key"
    exit 1
fi

echo "═══════════════════════════════════════════════════════════════"
echo "  MICAFP-UnifiedShield v6.0 — IPFS Config Publisher"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Step 1: Create config bundle
echo "[1] Creating config bundle..."
BUNDLE_DIR=$(mktemp -d)
cp "$CONFIG_DIR/cdn-endpoints.json" "$BUNDLE_DIR/"
cp "$CONFIG_DIR/p2p-bootstrap-peers.json" "$BUNDLE_DIR/"
cp "$CONFIG_DIR/isp-profiles.json" "$BUNDLE_DIR/"

# Add version and timestamp
cat > "$BUNDLE_DIR/metadata.json" <<EOF
{
  "version": "6.0.0",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "type": "config_update"
}
EOF

# Step 2: Sign the bundle
echo "[2] Signing config bundle..."
BUNDLE_HASH=$(cd "$BUNDLE_DIR" && find . -type f -sort | xargs sha256sum | sha256sum | cut -d' ' -f1)
echo "$BUNDLE_HASH" | openssl pkeyutl -sign -pkeyopt digest:sha256 -inkey "$SIGNING_KEY" -out "$BUNDLE_DIR/signature.bin"
echo "  Bundle hash: $BUNDLE_HASH"
echo "  Signature: $(xxd -p "$BUNDLE_DIR/signature.bin" | head -c 64)..."

# Step 3: Pack and add to IPFS
echo "[3] Adding to IPFS..."
tar czf "$BUNDLE_DIR/bundle.tar.gz" -C "$BUNDLE_DIR" .
CID=$(ipfs add -Q "$BUNDLE_DIR/bundle.tar.gz" 2>/dev/null || echo "ipfs-not-available")

if [ "$CID" != "ipfs-not-available" ]; then
    echo "  CID: $CID"
    echo ""
    echo "  Pin URL: https://ipfs.io/ipfs/$CID"
    echo ""
    echo "[4] Updating hardcoded CID in daemon..."
    # In production: update the hardcoded CID in ipfs_updater.rs
    echo "  CID=$CID" > "$CONFIG_DIR/latest-cid.env"
else
    echo "  IPFS not available locally. Upload manually:"
    echo "  File: $BUNDLE_DIR/bundle.tar.gz"
fi

# Step 5: Pin to Pinata
echo "[5] Pinning to Pinata..."
if [ -n "${PINATA_JWT:-}" ]; then
    curl -s -X POST "https://api.pinata.cloud/pinning/pinFileToIPFS" \
        -H "Authorization: Bearer $PINATA_JWT" \
        -F "file=@$BUNDLE_DIR/bundle.tar.gz" | jq .
else
    echo "  PINATA_JWT not set, skipping Pinata pinning"
fi

# Cleanup
rm -rf "$BUNDLE_DIR"

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  Config publishing complete!"
echo "═══════════════════════════════════════════════════════════════"
