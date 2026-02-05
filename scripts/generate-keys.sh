#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CERTS_DIR="$PROJECT_ROOT/certs"
PRIVATE_DIR="$CERTS_DIR/private"

mkdir -p "$PRIVATE_DIR"

PRIVATE_KEY="$PRIVATE_DIR/jwt-ca.key"
CERTIFICATE="$CERTS_DIR/jwt-ca.crt"
PUBLIC_KEY="$CERTS_DIR/jwt-ca.pub"

if [ -f "$PRIVATE_KEY" ] && [ -f "$PUBLIC_KEY" ]; then
	echo "Keys already exist. Use -f to force regeneration."
	if [ "${1:-}" != "-f" ]; then
		exit 0
	fi
	echo "Force regeneration requested."
fi

echo "Generating RSA 2048-bit private key..."
openssl genrsa -out "$PRIVATE_KEY" 2048 2>/dev/null

echo "Generating self-signed certificate..."
openssl req -new -x509 -key "$PRIVATE_KEY" -out "$CERTIFICATE" \
	-days 365 -subj "/CN=sqlite-editor-jwt-ca/O=sqlite-editor" 2>/dev/null

echo "Extracting public key..."
openssl rsa -in "$PRIVATE_KEY" -pubout -out "$PUBLIC_KEY" 2>/dev/null

chmod 600 "$PRIVATE_KEY"
chmod 644 "$PUBLIC_KEY" "$CERTIFICATE"

echo "Keys generated successfully:"
echo "  Private key: $PRIVATE_KEY"
echo "  Certificate: $CERTIFICATE"
echo "  Public key:  $PUBLIC_KEY"
