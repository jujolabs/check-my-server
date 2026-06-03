#!/bin/bash
set -euo pipefail

BINARY=/usr/local/bin/check-my-server
SERVICE=/etc/systemd/system/check-my-server.service
REPO=jujolabs/check-my-server

if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
  echo "Error: must run as root (use sudo)" >&2
  exit 1
fi

ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  SUFFIX="x86_64-linux" ;;
  aarch64) SUFFIX="aarch64-linux" ;;
  *)        echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
  | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

if [[ -z "$TAG" ]]; then
  echo "Error: could not determine latest release tag" >&2
  exit 1
fi

echo "Installing check-my-server $TAG ($ARCH)..."

BASE_URL="https://github.com/$REPO/releases/download/$TAG"
ARTIFACT="check-my-server-$SUFFIX"

WORK_DIR=$(mktemp -d)
trap 'rm -rf "$WORK_DIR"' EXIT

curl -fsSL "$BASE_URL/$ARTIFACT"     -o "$WORK_DIR/$ARTIFACT"
curl -fsSL "$BASE_URL/checksums.txt" -o "$WORK_DIR/checksums.txt"

if ! (cd "$WORK_DIR" && grep -F "$ARTIFACT" checksums.txt | sha256sum --check --status); then
  echo "Error: checksum verification failed — binary may be corrupted or tampered" >&2
  exit 1
fi

if systemctl is-active --quiet check-my-server 2>/dev/null; then
  systemctl stop check-my-server
fi

ENABLED=$(systemctl is-enabled check-my-server 2>/dev/null || true)
if [[ "$ENABLED" == "masked" ]]; then
  systemctl unmask check-my-server
fi

install -m 755 "$WORK_DIR/$ARTIFACT" "$BINARY"

curl -fsSL "https://raw.githubusercontent.com/$REPO/$TAG/contrib/check-my-server.service" \
  -o "$SERVICE"

systemctl daemon-reload
systemctl enable check-my-server
systemctl start check-my-server

echo "Done. check-my-server $TAG running on port 9100."
echo "Status: systemctl status check-my-server"
