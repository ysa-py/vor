#!/usr/bin/env python3
"""Conformance test for the Vor license reference implementation.

Runs every case in license/vectors.json and asserts the expected status.
Exit code 0 = all vectors agree.
"""
from __future__ import annotations

import datetime as dt
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from vor_license import load_public_key, verify_token, parse_rfc3339  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parents[2]
VECTORS = ROOT / "license" / "vectors.json"
DEV_PUB = ROOT / "license" / "keys" / "dev" / "VOR_LICENSE_PUBLIC_KEY.txt"


def main() -> int:
    vectors = json.loads(VECTORS.read_text())
    now = parse_rfc3339(vectors["spec"]["now"])
    public_key = load_public_key(str(DEV_PUB))
    assert vectors["public_key"] == open(DEV_PUB).read().strip(), "vector key mismatch"

    failures = 0
    for case in vectors["cases"]:
        status, _ = verify_token(public_key, case["token"], now=now)
        ok = status == case["expected"]
        print(f"{'PASS' if ok else 'FAIL'}  {case['name']}: got {status}, want {case['expected']}")
        failures += 0 if ok else 1
    print(f"\n{len(vectors['cases']) - failures}/{len(vectors['cases'])} vectors passed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
