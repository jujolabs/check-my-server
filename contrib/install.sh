#!/bin/bash
set -euo pipefail

BINARY=/usr/local/bin/check-my-server
SERVICE=/etc/systemd/system/check-my-server.service
REPO=jujolabs/check-my-server

echo "Installing check-my-server..."

curl -fsSL "https://github.com/$REPO/releases/latest/download/check-my-server" \
  -o "$BINARY"
chmod +x "$BINARY"

curl -fsSL "https://raw.githubusercontent.com/$REPO/main/contrib/check-my-server.service" \
  -o "$SERVICE"

systemctl daemon-reload
systemctl enable --now check-my-server

echo "Done. check-my-server running on port 9100."
echo "Status: systemctl status check-my-server"
