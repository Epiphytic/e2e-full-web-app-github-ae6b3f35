#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PRIVATE_KEY="$PROJECT_ROOT/certs/private/jwt-ca.key"

usage() {
	echo "Usage: $0 <username> [expiry_seconds]"
	echo "  username:       Subject claim for the JWT"
	echo "  expiry_seconds: Token lifetime in seconds (default: 3600)"
	exit 1
}

if [ $# -lt 1 ]; then
	usage
fi

USERNAME="$1"
EXPIRY_SECONDS="${2:-3600}"

if [ ! -f "$PRIVATE_KEY" ]; then
	echo "Error: Private key not found at $PRIVATE_KEY"
	echo "Run scripts/generate-keys.sh first."
	exit 1
fi

NOW=$(date +%s)
EXP=$((NOW + EXPIRY_SECONDS))

# Build JWT header and payload
HEADER=$(printf '{"alg":"RS256","typ":"JWT"}' | openssl base64 -e -A | tr '+/' '-_' | tr -d '=')
PAYLOAD=$(printf '{"sub":"%s","iat":%d,"exp":%d}' "$USERNAME" "$NOW" "$EXP" | openssl base64 -e -A | tr '+/' '-_' | tr -d '=')

# Sign with RSA private key
SIGNATURE=$(printf '%s.%s' "$HEADER" "$PAYLOAD" | openssl dgst -sha256 -sign "$PRIVATE_KEY" -binary | openssl base64 -e -A | tr '+/' '-_' | tr -d '=')

TOKEN="$HEADER.$PAYLOAD.$SIGNATURE"
echo "$TOKEN"
