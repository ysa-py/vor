#!/usr/bin/env bash
# audit-apk-permissions.sh — Play Protect risk-signal gate for shipped Vor APKs.
#
# Usage: audit-apk-permissions.sh <apk-path> <module>
#   module: "app" | "license-manager"
#
# Background (real-world defect, 2026-09): v1.0.0's main APK shipped with
# QUERY_ALL_PACKAGES (own declaration) plus AD_ID / ACCESS_ADSERVICES_* /
# BIND_GET_INSTALL_REFERRER_SERVICE (injected by the firebase-analytics AAR
# manifest). Unknown sideloaded APK + brand-new certificate + ad-identifier +
# broad package visibility is precisely the heuristic profile Google Play
# Protect's cloud scanner associates with hidden-ads/spyware — users got the
# hard "App blocked to protect your device" dialog.
#
# This gate compares the FINAL merged manifest of a built APK against an exact
# per-module allowlist. It is an allowlist (not a denylist) on purpose: any
# permission outside the approved set — whether added by our own code or
# silently injected by an upgraded/transitive library AAR — fails the build and
# forces a conscious allowlist update. Works locally and in GitHub Actions
# (aapt2 from PATH, ANDROID_HOME, or ~/android-sdk; ::error:: annotations
# degrade to plain lines outside CI).

set -euo pipefail

APK="${1:-}"
MODULE="${2:-}"

if [ -z "$APK" ] || [ ! -f "$APK" ]; then
  echo "::error::audit-apk-permissions: APK not found: '${APK}'"
  exit 2
fi

case "$MODULE" in
  app)
    # Exactly the permissions the main app needs (see
    # android/app/src/main/AndroidManifest.xml). Anything else = gate failure.
    ALLOWED=(
      android.permission.INTERNET
      android.permission.ACCESS_NETWORK_STATE
      android.permission.FOREGROUND_SERVICE
      android.permission.FOREGROUND_SERVICE_SPECIAL_USE
      android.permission.POST_NOTIFICATIONS
      android.permission.RECEIVE_BOOT_COMPLETED
      android.permission.CAMERA
      android.permission.WAKE_LOCK
    )
    SELF_PREFIXES="com.vor.app"
    ;;
  license-manager)
    # The license manager needs NO Android permissions at all — pasting and
    # verifying an offline license token is pure local computation.
    ALLOWED=()
    SELF_PREFIXES="com.vor.licensemanager"
    ;;
  *)
    echo "::error::audit-apk-permissions: unknown module '${MODULE}' (use 'app' or 'license-manager')"
    exit 2
    ;;
esac

# ---- locate aapt2 ----------------------------------------------------------
AAPT2="${AAPT2:-}"
if [ -z "$AAPT2" ] || [ ! -x "$AAPT2" ]; then
  for c in "$(command -v aapt2 2>/dev/null || true)" \
           ${ANDROID_HOME:+$ANDROID_HOME/build-tools/*/aapt2} \
           /usr/local/lib/android/sdk/build-tools/*/aapt2 \
           "$HOME"/android-sdk/build-tools/*/aapt2 \
           "$HOME"/Android/Sdk/build-tools/*/aapt2; do
    if [ -n "$c" ] && [ -x "$c" ]; then AAPT2="$c"; break; fi
  done
fi
if [ -z "$AAPT2" ] || [ ! -x "$AAPT2" ]; then
  echo "::error::audit-apk-permissions: aapt2 not found (set AAPT2 or ANDROID_HOME)"
  exit 2
fi
echo "audit-apk-permissions: using aapt2 at $AAPT2"

# ---- dump the merged manifest's uses-permission set ------------------------
DUMP="$(mktemp)"
trap 'rm -f "$DUMP" "$DUMP.got" "$DUMP.allowed" 2>/dev/null || true' EXIT

"$AAPT2" dump permissions "$APK" > "$DUMP" 2>/dev/null || {
  echo "::error::audit-apk-permissions: aapt2 failed to dump permissions for $(basename "$APK")"
  exit 2
}

# Extract "uses-permission: name='X'" lines only (skip our own declared
# custom permissions, which are package-prefixed and always benign).
sed -n "s/^uses-permission: *name='\([^']*\)'.*$/\1/p" "$DUMP" > "$DUMP.got" || true

# Drop self-declared custom permissions (DYNAMIC_RECEIVER_NOT_EXPORTED etc.) —
# they are defined by the app itself and are not risk signals.
for p in $SELF_PREFIXES; do
  sed -i "/^${p//./\\.}\./d" "$DUMP.got" 2>/dev/null || true
done

printf '%s\n' "${ALLOWED[@]}" > "$DUMP.allowed" 2>/dev/null || : > "$DUMP.allowed"
sort -u -o "$DUMP.got" "$DUMP.got"
sort -u -o "$DUMP.allowed" "$DUMP.allowed"

# ---- forbidden hot-list (better error messages for the usual suspects) ------
FORBIDDEN_RE='QUERY_ALL_PACKAGES|\.AD_ID|ACCESS_ADSERVICES|INSTALL_REFERRER|SMS|MMS|WAP_PUSH|ACCESSIBILITY|REQUEST_INSTALL_PACKAGES|SYSTEM_ALERT_WINDOW|CONTACTS|CALL_LOG|READ_PHONE_STATE|RECORD_AUDIO|LOCATION|EXTERNAL_STORAGE|PACKAGE_USAGE_STATS|MANAGE_EXTERNAL_STORAGE|BODY_SENSORS|READ_CALENDAR'

# ---- diff against the allowlist --------------------------------------------
EXTRA="$(comm -13 "$DUMP.allowed" "$DUMP.got")"
MISSING="$(comm -23 "$DUMP.allowed" "$DUMP.got")"
FAIL=0

if [ -n "$EXTRA" ]; then
  FAIL=1
  echo "::error::audit-apk-permissions: $(basename "$APK") requests permission(s) OUTSIDE the '$MODULE' allowlist:"
  while IFS= read -r p; do
    if echo "$p" | grep -Eq "$FORBIDDEN_RE"; then
      echo "::error::  + $p   <-- known Play Protect risk signal (remove or justify + update allowlist)"
    else
      echo "::error::  + $p"
    fi
  done <<< "$EXTRA"
  echo "::error::If intentional: update ALLOWED in scripts/audit-apk-permissions.sh AND android/app/src/main/AndroidManifest.xml comments."
fi

if [ -n "$MISSING" ]; then
  FAIL=1
  echo "::error::audit-apk-permissions: $(basename "$APK") is MISSING allowlisted permission(s) (verify the module mapping):"
  while IFS= read -r p; do echo "::error::  - $p"; done <<< "$MISSING"
fi

if [ "$FAIL" -ne 0 ]; then
  echo "audit-apk-permissions: FAILED for $(basename "$APK")"
  echo "--- merged uses-permission set ---"
  cat "$DUMP.got"
  exit 1
fi

echo "audit-apk-permissions: PASS — $(basename "$APK") requests exactly the approved $(wc -l < "$DUMP.got" | tr -d ' ') permission(s):"
sed 's/^/  /' "$DUMP.got"
