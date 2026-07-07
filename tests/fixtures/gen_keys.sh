#!/usr/bin/env bash
# Regenerate the RSA-2048 keypair + JWKS used by tests/layer.rs.
# Outputs (committed alongside this script):
#   test_priv.pem  — PKCS#1 RSA private key (PEM)
#   test_pub.pem   — SubjectPublicKeyInfo (PEM)
#   test_jwks.json — single-key JWKS with kid "test-kid", alg RS256
#
# Requires: openssl, python3 with the `cryptography` package.
set -euo pipefail
cd "$(dirname "$0")"

openssl genrsa -out test_priv.pem 2048
openssl rsa -in test_priv.pem -pubout -out test_pub.pem

python3 - <<'PY' > test_jwks.json
from cryptography.hazmat.primitives import serialization
import base64, json

key = serialization.load_pem_public_key(open("test_pub.pem", "rb").read())
nums = key.public_numbers()

def b64(i: int) -> str:
    return base64.urlsafe_b64encode(
        i.to_bytes((i.bit_length() + 7) // 8, "big")
    ).rstrip(b"=").decode()

print(json.dumps({
    "keys": [{
        "use": "sig",
        "kty": "RSA",
        "kid": "test-kid",
        "alg": "RS256",
        "n": b64(nums.n),
        "e": b64(nums.e),
    }]
}, indent=2))
PY

echo "Wrote test_priv.pem, test_pub.pem, test_jwks.json"
