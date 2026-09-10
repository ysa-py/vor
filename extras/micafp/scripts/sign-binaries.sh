#!/bin/bash
set -euo pipefail
echo "=== Signing binaries ==="
if [ -n "${GPG_KEY_ID:-}" ]; then
    find . -name "unifiedshield-daemon" -o -name "*.apk" -o -name "*.exe" | while read f; do
        gpg --default-key "$GPG_KEY_ID" --detach-sign "$f"
    done
fi
echo "=== Signing complete ==="
