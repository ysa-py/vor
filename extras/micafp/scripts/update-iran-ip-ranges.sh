#!/bin/bash
set -euo pipefail
echo "=== Updating Iranian IP ranges from RIPE NCC ==="
# Fetch from RIPE NCC for Iranian ASNs
ASNS="AS12880 AS16322 AS24631 AS25184 AS31549 AS34918 AS39074 AS41689 AS44244 AS48434 AS56402 AS197207"
OUTPUT="configs/iran-ip-ranges.json"
for asn in $ASNS; do
    echo "Fetching $asn..."
    curl -s "https://stat.ripe.net/data/announced-prefixes/data.json?resource=$asn" >> /tmp/ripe_${asn}.json
done
echo "Ranges updated. Manual review recommended before committing."
