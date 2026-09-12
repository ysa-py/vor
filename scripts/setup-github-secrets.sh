#!/usr/bin/env bash
#
# setup-github-secrets.sh
#
# One-shot / re-runnable bootstrap that pushes everything ci.yml and
# release.yml need into GitHub — as Actions secrets (sensitive) and Actions
# variables (non-sensitive) — via the GitHub CLI, instead of pasting values
# into Settings -> Secrets and variables.
#
# Matches what the Vor workflows ACTUALLY read (checked against release.yml
# line by line):
#
#   Secrets:
#     GOOGLE_SERVICES_JSON   - raw contents of app/google-services.json
#                              (optional: ci.yml/release.yml fall back to the
#                              committed placeholder when unset)
#     KEYSTORE_BASE64        - base64 of the release .jks
#     KEYSTORE_PASSWORD      - store AND key password (release.yml uses one
#                              password for both)
#     KEYSTORE_ALIAS         - key alias inside the keystore
#     VOR_LICENSE_PUBLIC_KEY - Ed25519 public key embedded at build time in
#                              Android/desktop/iOS (see license/keys/; NOT
#                              generated here, see step 3 below)
#   Variable:
#     ADDONS_REPO            - e.g. ysa-py/Vor (read by both ci.yml and release.yml)
#
# Requirements: gh, keytool (JDK), base64, openssl.
#
# Usage:
#   ./scripts/setup-github-secrets.sh
#
# Env overrides (all optional):
#   REPO                        owner/name (default: inferred from git remote)
#   ADDONS_REPO_VALUE           value for the ADDONS_REPO variable (default: ysa-py/Vor)
#   KEYSTORE_PATH               path to reuse/create the .jks (default: ./release.jks)
#   KEYSTORE_ALIAS_VALUE        alias to use for a newly generated keystore (default: vor-release)
#   GOOGLE_SERVICES_JSON_PATH   local path to an existing google-services.json
#   VOR_LICENSE_PUBLIC_KEY_PATH local path to the license public key value (see step 3)

set -euo pipefail

log()  { printf '\033[36m[setup]\033[0m %s\n' "$*"; }
warn() { printf '\033[33m[warn ]\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[31m[fail ]\033[0m %s\n' "$*" >&2; exit 1; }

# ---------------------------------------------------------------------------
# 0. Preconditions
# ---------------------------------------------------------------------------
command -v gh      >/dev/null 2>&1 || die "GitHub CLI (gh) is not installed. Install it, then re-run."
command -v keytool >/dev/null 2>&1 || die "keytool not found (needs a JDK on PATH)."
command -v base64  >/dev/null 2>&1 || die "base64 not found."
command -v openssl >/dev/null 2>&1 || die "openssl not found."

if ! gh auth status >/dev/null 2>&1; then
  log "gh is not authenticated yet -- launching login once (this replaces ever pasting a token)."
  gh auth login
fi

REPO="${REPO:-$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null || true)}"
[ -n "$REPO" ] || die "Could not infer REPO. Run inside the repo, or pass REPO=owner/name."
log "Target repository: $REPO"

KEYSTORE_PATH="${KEYSTORE_PATH:-./release.jks}"
KEYSTORE_ALIAS_VALUE="${KEYSTORE_ALIAS_VALUE:-vor-release}"
ADDONS_REPO_VALUE="${ADDONS_REPO_VALUE:-ysa-py/Vor}"
GOOGLE_SERVICES_JSON_PATH="${GOOGLE_SERVICES_JSON_PATH:-./android/app/google-services.json}"
VOR_LICENSE_PUBLIC_KEY_PATH="${VOR_LICENSE_PUBLIC_KEY_PATH:-}"

# ---------------------------------------------------------------------------
# 1. Release keystore -- generate if missing, one password for store+key
#    (release.yml's "Release keystore" step only reads KEYSTORE_PASSWORD
#    once and reuses it for both storePassword and keyPassword, so this
#    script generates the matching single password, not two).
# ---------------------------------------------------------------------------
if [ -f "$KEYSTORE_PATH" ]; then
  log "Reusing existing keystore at $KEYSTORE_PATH"
  read -rp "  Password for existing keystore (store + key): " -s KEYSTORE_PASSWORD; echo
  read -rp "  Key alias: " KEYSTORE_ALIAS_VALUE
else
  log "No keystore found -- generating a new one with a strong random password."
  KEYSTORE_PASSWORD="$(openssl rand -base64 24 | tr -d '=+/')"

  keytool -genkeypair \
    -v \
    -keystore "$KEYSTORE_PATH" \
    -alias "$KEYSTORE_ALIAS_VALUE" \
    -keyalg RSA \
    -keysize 4096 \
    -validity 10950 \
    -storepass "$KEYSTORE_PASSWORD" \
    -keypass "$KEYSTORE_PASSWORD" \
    -dname "CN=Vor, OU=Dev, O=Vor, L=, ST=, C=IR" \
    >/dev/null

  log "Generated $KEYSTORE_PATH (alias: $KEYSTORE_ALIAS_VALUE)."
fi

# ---------------------------------------------------------------------------
# 2. google-services.json -- must exist locally; not auto-generated
#    (Firebase project config is account-specific -- nothing can invent it
#    for you the way a keystore password can be randomly generated).
# ---------------------------------------------------------------------------
if [ -f "$GOOGLE_SERVICES_JSON_PATH" ]; then
  log "Using existing google-services.json at $GOOGLE_SERVICES_JSON_PATH"
else
  warn "google-services.json not found at $GOOGLE_SERVICES_JSON_PATH."
  warn "Skipping the GOOGLE_SERVICES_JSON secret -- ci.yml/release.yml both already"
  warn "fall back to app/google-services.json.example when this secret is unset, so"
  warn "this is safe to skip if you don't have a Firebase project configured yet."
  GOOGLE_SERVICES_JSON_PATH=""
fi

# ---------------------------------------------------------------------------
# 3. VOR_LICENSE_PUBLIC_KEY -- NOT generated here
#    The exact encoding the app code expects (raw base64 of 32 bytes, hex,
#    or PEM SubjectPublicKeyInfo) is defined by the verification code
#    (android core-license module / desktop internal/license / ios Vor
#    Info.plist key). Point this at a file containing the value in the
#    format the verifier actually reads, or set it manually with:
#      gh secret set VOR_LICENSE_PUBLIC_KEY --repo "$REPO" < path/to/key.txt
# ---------------------------------------------------------------------------
if [ -n "$VOR_LICENSE_PUBLIC_KEY_PATH" ] && [ -f "$VOR_LICENSE_PUBLIC_KEY_PATH" ]; then
  log "Uploading VOR_LICENSE_PUBLIC_KEY secret from $VOR_LICENSE_PUBLIC_KEY_PATH..."
  gh secret set VOR_LICENSE_PUBLIC_KEY --repo "$REPO" < "$VOR_LICENSE_PUBLIC_KEY_PATH"
else
  warn "VOR_LICENSE_PUBLIC_KEY_PATH not set/found -- skipping that secret this run."
fi

# ---------------------------------------------------------------------------
# 4. Push secrets (encrypted, never shown in logs)
# ---------------------------------------------------------------------------
if [ -n "$GOOGLE_SERVICES_JSON_PATH" ]; then
  log "Uploading GOOGLE_SERVICES_JSON secret..."
  gh secret set GOOGLE_SERVICES_JSON --repo "$REPO" < "$GOOGLE_SERVICES_JSON_PATH"
fi

log "Uploading KEYSTORE_BASE64 secret..."
base64 -w0 "$KEYSTORE_PATH" | gh secret set KEYSTORE_BASE64 --repo "$REPO"

log "Uploading KEYSTORE_PASSWORD secret..."
printf '%s' "$KEYSTORE_PASSWORD" | gh secret set KEYSTORE_PASSWORD --repo "$REPO"

log "Uploading KEYSTORE_ALIAS secret..."
printf '%s' "$KEYSTORE_ALIAS_VALUE" | gh secret set KEYSTORE_ALIAS --repo "$REPO"

# ---------------------------------------------------------------------------
# 5. Push non-sensitive config as a repo variable, not a secret
#    (read identically by both ci.yml and release.yml)
# ---------------------------------------------------------------------------
log "Setting ADDONS_REPO variable to '$ADDONS_REPO_VALUE'..."
gh variable set ADDONS_REPO --repo "$REPO" --body "$ADDONS_REPO_VALUE"

log "Done. Verify with:"
echo "    gh secret list --repo $REPO"
echo "    gh variable list --repo $REPO"

warn "release.jks and (if used) google-services.json now live locally too -- keep them out of version control."
warn "The keystore password was only ever held in memory + pushed as a secret -- it was not written to any local file by this script."
