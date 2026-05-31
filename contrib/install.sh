#!/bin/bash
set -euo pipefail

BINARY=/usr/local/bin/check-my-server
SERVICE=/etc/systemd/system/check-my-server.service
REPO=jujolabs/check-my-server

ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  SUFFIX="x86_64-linux" ;;
  aarch64) SUFFIX="aarch64-linux" ;;
  *)        echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

echo "Installing check-my-server ($ARCH)..."

curl -fsSL "https://github.com/$REPO/releases/latest/download/check-my-server-$SUFFIX" \
  -o "$BINARY"
chmod +x "$BINARY"

curl -fsSL "https://raw.githubusercontent.com/$REPO/main/contrib/check-my-server.service" \
  -o "$SERVICE"

systemctl daemon-reload
systemctl enable --now check-my-server

echo "Done. check-my-server running on port 9100."
echo "Status: systemctl status check-my-server"
