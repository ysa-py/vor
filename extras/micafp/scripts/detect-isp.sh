#!/usr/bin/env bash
# ══════════════════════════════════════════════════════════════
# MICAFP-UnifiedShield v6.0 — ISP Detection Script
# Automatically detects the current Iranian ISP and outputs
# the recommended configuration profile
# ══════════════════════════════════════════════════════════════

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ISP_PROFILES="$SCRIPT_DIR/configs/isp-profiles.json"

echo "═══════════════════════════════════════════════════════════════"
echo "  MICAFP-UnifiedShield v6.0 — ISP Detection"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Method 1: Check external IP ASN
echo "[1] Detecting ISP via IP ASN lookup..."
IP_INFO=$(curl -s https://ipinfo.io/json 2>/dev/null || echo "{}")
ISP_ORG=$(echo "$IP_INFO" | jq -r '.org // "unknown"' 2>/dev/null || echo "unknown")
IP_ADDR=$(echo "$IP_INFO" | jq -r '.ip // "unknown"' 2>/dev/null || echo "unknown")
COUNTRY=$(echo "$IP_INFO" | jq -r '.country // "unknown"' 2>/dev/null || echo "unknown")

echo "  IP: $IP_ADDR"
echo "  Country: $COUNTRY"
echo "  Organization: $ISP_ORG"

# Method 2: Check DNS resolution (Iranian DNS returns injected results)
echo ""
echo "[2] Checking DNS injection..."
INJECTED=false
for domain in youtube.com twitter.com facebook.com; do
    DIG_RESULT=$(dig +short "$domain" @5.200.200.200 2>/dev/null | head -1)
    if echo "$DIG_RESULT" | grep -qE "^10\." ; then
        INJECTED=true
        echo "  DNS injection detected for $domain → $DIG_RESULT"
    fi
done

if [ "$INJECTED" = false ]; then
    echo "  No DNS injection detected (or not on Iranian network)"
fi

# Method 3: Check if Iranian NTP servers are reachable
echo ""
echo "[3] Checking NTP accessibility..."
NTP_OK=$(ntpdate -q ntp.sntp.ir 2>/dev/null && echo "reachable" || echo "unreachable")
echo "  ntp.sntp.ir: $NTP_OK"

# Method 4: Check if Arvan Cloud is reachable (domestic)
echo ""
echo "[4] Checking domestic CDN accessibility..."
ARVAN_OK=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 5 https://www.arvancloud.ir 2>/dev/null || echo "000")
if [ "$ARVAN_OK" = "200" ] || [ "$ARVAN_OK" = "301" ] || [ "$ARVAN_OK" = "302" ]; then
    echo "  Arvan Cloud: reachable (HTTP $ARVAN_OK)"
else
    echo "  Arvan Cloud: unreachable (HTTP $ARVAN_OK)"
fi

# Match ISP profile
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  Detection Results"
echo "═══════════════════════════════════════════════════════════════"

DETECTED_ISP="unknown"
if echo "$ISP_ORG" | grep -qi "mci\|hamrah\|mobile communication company"; then
    DETECTED_ISP="mci"
elif echo "$ISP_ORG" | grep -qi "irancell\|mtn"; then
    DETECTED_ISP="irancell"
elif echo "$ISP_ORG" | grep -qi "rightel"; then
    DETECTED_ISP="rightel"
elif echo "$ISP_ORG" | grep -qi "shatel"; then
    DETECTED_ISP="shatel"
elif echo "$ISP_ORG" | grep -qi "pars online\|parsonline"; then
    DETECTED_ISP="pars_online"
elif echo "$ISP_ORG" | grep -qi "mokhaberat\|telecommunication company\|tci"; then
    DETECTED_ISP="mokhaberat"
fi

echo "  Detected ISP: $DETECTED_ISP"
echo "  DNS Injection: $INJECTED"
echo "  Country: $COUNTRY"
echo ""

if [ "$DETECTED_ISP" != "unknown" ] && [ -f "$ISP_PROFILES" ]; then
    echo "  Recommended profile from isp-profiles.json:"
    jq ".isp_profiles[] | select(.id == \"$DETECTED_ISP\")" "$ISP_PROFILES" 2>/dev/null || echo "  Profile not found"
fi
