#!/bin/bash
set -euo pipefail
echo "=== Generating SHA256 checksums ==="
find . -type f \( -name "*.apk" -o -name "*.aab" -o -name "*.exe" -o -name "*.deb" -o -name "*.rpm" -o -name "*.ipk" -o -name "unifiedshield-daemon" -o -name "*.xpi" -o -name "*.zip" \) | while read f; do
    sha256sum "$f" > "$f.sha256"
done
cat *.sha256 > SHA256SUMS.txt
echo "=== Checksums generated ==="
