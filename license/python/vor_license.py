#!/usr/bin/env python3
"""Vor offline license reference implementation (Ed25519).

Used by:
  - the GitHub Actions issuance workflow (.github/workflows/issue-license.yml)
  - CI test-vector generation and verification
  - local testing

Canonical serialization keeps the emitted tokens stable across invocations.

Subcommands:
  keygen  [--out DIR]            generate an Ed25519 keypair (dev tooling)
  issue   --key FILE --expires RFC3339 [--id UUID] [--tier NAME]
                                  [--platforms a,b,c] [--json]  print token
  verify  --pub FILE --token S   verify a token (exit 0/1/2)
  vectors --pub FILE --pair FILE emit test vectors JSON to stdout

Requires: cryptography >= 41 (Ed25519 support).
"""
from __future__ import annotations

import argparse
import base64
import datetime as dt
import json
import sys
import uuid

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey, Ed25519PublicKey
from cryptography.exceptions import InvalidSignature

TOKEN_PREFIX = "VOR1"
PRODUCT = "vor"
PAYLOAD_VERSION = 1
CANONICAL_FIELDS = ("v", "id", "product", "issued_at", "expires_at", "entitlements")
ENTITLEMENT_FIELDS = ("tier", "platforms")


class LicenseError(Exception):
    pass


# ---------------------------------------------------------------- encoding

def b64url_encode(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode("ascii")


def b64url_decode(text: str) -> bytes:
    pad = "=" * (-len(text) % 4)
    return base64.urlsafe_b64decode(text + pad)


# ---------------------------------------------------------------- keys

def load_private_key(path: str) -> Ed25519PrivateKey:
    with open(path, "rb") as fh:
        seed = b64url_decode(fh.read().decode("ascii").strip())
    if len(seed) != 32:
        raise LicenseError(f"private key seed must be 32 bytes, got {len(seed)}")
    return Ed25519PrivateKey.from_private_bytes(seed)


def load_public_key(path: str) -> Ed25519PublicKey:
    with open(path, "rb") as fh:
        raw = b64url_decode(fh.read().decode("ascii").strip())
    if len(raw) != 32:
        raise LicenseError(f"public key must be 32 bytes, got {len(raw)}")
    return Ed25519PublicKey.from_public_bytes(raw)


def cmd_keygen(args: argparse.Namespace) -> int:
    import os
    priv = Ed25519PrivateKey.generate()
    seed = priv.private_bytes_raw()
    pub = priv.public_key().public_bytes_raw()
    os.makedirs(args.out, exist_ok=True)
    priv_path = os.path.join(args.out, "VOR_LICENSE_PRIVATE_KEY.txt")
    pub_path = os.path.join(args.out, "VOR_LICENSE_PUBLIC_KEY.txt")
    with open(priv_path, "w") as fh:
        fh.write(b64url_encode(seed) + "\n")
    with open(pub_path, "w") as fh:
        fh.write(b64url_encode(pub) + "\n")
    print(f"wrote {priv_path}")
    print(f"wrote {pub_path}")
    return 0


# ---------------------------------------------------------------- payloads

def parse_rfc3339(value: str) -> dt.datetime:
    text = value.strip()
    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    when = dt.datetime.fromisoformat(text)
    if when.tzinfo is None:
        when = when.replace(tzinfo=dt.timezone.utc)
    return when.astimezone(dt.timezone.utc)


def canonical_payload(payload: dict) -> bytes:
    ordered = {}
    for field in CANONICAL_FIELDS:
        if field not in payload:
            continue
        value = payload[field]
        if field == "entitlements" and isinstance(value, dict):
            ent = {}
            for k in ENTITLEMENT_FIELDS:
                if k in value:
                    ent[k] = value[k]
            for k in sorted(value.keys()):
                if k not in ENTITLEMENT_FIELDS:
                    ent[k] = value[k]
            value = ent
        ordered[field] = value
    return json.dumps(ordered, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def build_payload(license_id: str, issued_at: str, expires_at: str,
                  tier: str, platforms: list) -> dict:
    return {
        "v": PAYLOAD_VERSION,
        "id": license_id,
        "product": PRODUCT,
        "issued_at": issued_at,
        "expires_at": expires_at,
        "entitlements": {"tier": tier, "platforms": platforms},
    }


# ---------------------------------------------------------------- issue

def issue_token(private_key: Ed25519PrivateKey, payload: dict) -> str:
    payload_bytes = canonical_payload(payload)
    signature = private_key.sign(payload_bytes)
    return f"{TOKEN_PREFIX}.{b64url_encode(payload_bytes)}.{b64url_encode(signature)}"


def cmd_issue(args: argparse.Namespace) -> int:
    private_key = load_private_key(args.key)
    expires_at = parse_rfc3339(args.expires).strftime("%Y-%m-%dT%H:%M:%SZ")
    issued_at = parse_rfc3339(args.issued).strftime("%Y-%m-%dT%H:%M:%SZ") if args.issued \
        else dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    license_id = args.id or str(uuid.uuid4())
    platforms = [p.strip() for p in args.platforms.split(",") if p.strip()] if args.platforms \
        else ["android", "windows", "linux", "openwrt", "ios"]
    payload = build_payload(license_id, issued_at, expires_at, args.tier, platforms)
    token = issue_token(private_key, payload)
    if args.json:
        print(json.dumps({"token": token, "payload": payload}, indent=2))
    else:
        print(token)
    return 0


# ---------------------------------------------------------------- verify

def verify_token(public_key: Ed25519PublicKey, token: str,
                 now=None):
    """Returns (status, payload). status in {VALID, EXPIRED, INVALID}."""
    try:
        parts = token.strip().split(".")
        if len(parts) != 3 or parts[0] != TOKEN_PREFIX:
            return "INVALID", None
        payload_bytes = b64url_decode(parts[1])
        signature = b64url_decode(parts[2])
        if len(signature) != 64:
            return "INVALID", None
        public_key.verify(signature, payload_bytes)
        payload = json.loads(payload_bytes.decode("utf-8"))
        if payload.get("v") != PAYLOAD_VERSION or payload.get("product") != PRODUCT:
            return "INVALID", None
        if "id" not in payload or "issued_at" not in payload or "expires_at" not in payload:
            return "INVALID", None
        expires = parse_rfc3339(payload["expires_at"])
        now = now or dt.datetime.now(dt.timezone.utc)
        return ("EXPIRED" if now >= expires else "VALID"), payload
    except (InvalidSignature, LicenseError, ValueError, KeyError):
        return "INVALID", None


def cmd_verify(args: argparse.Namespace) -> int:
    public_key = load_public_key(args.pub)
    status, payload = verify_token(public_key, args.token)
    print(json.dumps({"status": status, "payload": payload}, indent=2))
    return {"VALID": 0, "EXPIRED": 1, "INVALID": 2}.get(status, 2)


# ---------------------------------------------------------------- vectors

def cmd_vectors(args: argparse.Namespace) -> int:
    public_key = load_public_key(args.pub)
    pub_b64 = b64url_encode(public_key.public_bytes_raw())
    now = parse_rfc3339("2026-09-10T12:00:00Z")
    priv = Ed25519PrivateKey.generate()  # wrong key for negative tests
    wrong_pub_b64 = b64url_encode(priv.public_key().public_bytes_raw())

    good = build_payload("11111111-2222-3333-4444-555555555555",
                         "2026-01-01T00:00:00Z", "2027-01-01T00:00:00Z",
                         "standard", ["android", "windows", "linux", "openwrt", "ios"])
    expiring = build_payload("22222222-3333-4444-5555-666666666666",
                             "2025-01-01T00:00:00Z", "2026-06-01T00:00:00Z",
                             "standard", ["android"])
    no_ent = {"v": 1, "id": "33333333-4444-5555-6666-777777777777",
              "product": "vor", "issued_at": "2026-01-01T00:00:00Z",
              "expires_at": "2027-01-01T00:00:00Z"}

    vectors = {
        "spec": {"now": "2026-09-10T12:00:00Z", "product": PRODUCT},
        "public_key": pub_b64,
        "cases": [],
    }
    cases = vectors["cases"]

    pair = load_private_key(args.pair)

    tok = issue_token(pair, good)
    cases.append({"name": "valid_standard",
                  "token": tok, "expected": "VALID"})

    tok = issue_token(pair, expiring)
    cases.append({"name": "expired",
                  "token": tok, "expected": "EXPIRED"})

    tok = issue_token(pair, no_ent)
    cases.append({"name": "valid_no_entitlements",
                  "token": tok, "expected": "VALID"})

    # tampered payload (same signature, different payload bytes)
    tampered_payload = json.loads(canonical_payload(good).decode())
    tampered_payload["id"] = "99999999-9999-9999-9999-999999999999"
    forged = f"{TOKEN_PREFIX}.{b64url_encode(canonical_payload(tampered_payload))}." \
             f"{tok.split('.')[2]}"
    cases.append({"name": "tampered_payload_same_sig",
                  "token": forged, "expected": "INVALID"})

    # signed by the wrong key
    wrong = issue_token(priv, good)
    cases.append({"name": "wrong_signing_key",
                  "token": wrong, "expected": "INVALID", "note": wrong_pub_b64})

    tok = issue_token(pair, good)
    cases.append({"name": "not_base64",
                  "token": "VOR1.!!not-base64!!.AAAA", "expected": "INVALID"})
    cases.append({"name": "wrong_prefix",
                  "token": "VORX." + tok.split(".")[1] + "." + tok.split(".")[2],
                  "expected": "INVALID"})
    cases.append({"name": "missing_segments",
                  "token": "VOR1." + tok.split(".")[1], "expected": "INVALID"})
    cases.append({"name": "empty",
                  "token": "", "expected": "INVALID"})
    cases.append({"name": "short_signature",
                  "token": "VOR1." + tok.split(".")[1] + ".AAAA", "expected": "INVALID"})

    # product mismatch, correctly signed
    other = dict(good)
    other["product"] = "notvor"
    tok = issue_token(pair, other)
    cases.append({"name": "wrong_product", "token": tok, "expected": "INVALID"})

    # version mismatch, correctly signed
    other = dict(good)
    other["v"] = 2
    tok = issue_token(pair, other)
    cases.append({"name": "wrong_version", "token": tok, "expected": "INVALID"})

    # boundary: expires exactly at "now"
    boundary = build_payload("44444444-5555-6666-7777-888888888888",
                             "2026-01-01T00:00:00Z", "2026-09-10T12:00:00Z",
                             "standard", ["linux"])
    tok = issue_token(pair, boundary)
    cases.append({"name": "expiry_exact_boundary",
                  "token": tok, "expected": "EXPIRED",
                  "note": "now == expires_at counts as expired"})

    print(json.dumps(vectors, indent=2))
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("keygen")
    p.add_argument("--out", default=".")
    p.set_defaults(func=cmd_keygen)

    p = sub.add_parser("issue")
    p.add_argument("--key", required=True, help="private key seed file (b64url)")
    p.add_argument("--expires", required=True, help="RFC3339 expiry timestamp")
    p.add_argument("--issued", default=None, help="RFC3339 issue timestamp (default: now UTC)")
    p.add_argument("--id", default=None, help="license id (default: random uuid4)")
    p.add_argument("--tier", default="standard")
    p.add_argument("--platforms", default="android,windows,linux,openwrt,ios")
    p.add_argument("--json", action="store_true")
    p.set_defaults(func=cmd_issue)

    p = sub.add_parser("verify")
    p.add_argument("--pub", required=True, help="public key file (b64url)")
    p.add_argument("--token", required=True)
    p.set_defaults(func=cmd_verify)

    p = sub.add_parser("vectors")
    p.add_argument("--pub", required=True, help="public key file (b64url)")
    p.add_argument("--pair", required=True, help="private key file of the SAME keypair (b64url)")
    p.set_defaults(func=cmd_vectors)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
