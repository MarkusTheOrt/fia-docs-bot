#!/usr/bin/env python3
"""utility that generates Ed25519 key and a JWT for testing

the public key is stored in jwt_key.pem (in PEM format) and jwt_key.base64 (raw
base64 format) and the JWT is printed to stdout
"""
import jwt
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

with open("jwt_private.pem", "rb") as file:
    private_key = serialization.load_pem_private_key(file.read(), None)


privkey_pem = private_key.private_bytes(
    encoding=serialization.Encoding.PEM,
    format=serialization.PrivateFormat.PKCS8,
    encryption_algorithm=serialization.NoEncryption(),
)

claims = {
}
token = jwt.encode(claims, privkey_pem, "EdDSA")

print(f"Full access: {token}")
