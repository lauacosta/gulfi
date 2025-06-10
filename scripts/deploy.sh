#!/usr/bin/env bash
set -euo pipefail

APP_NAME="gulfi"
REMOTE_USER="${SERVER_USER:-}"
REMOTE_HOST="${SERVER_HOST:-}"
REMOTE_DIR="/home/$REMOTE_USER/$APP_NAME"
SYSTEMD_SERVICE="$APP_NAME.service"


echo "[1/6] Setting up SSH configuration..."
mkdir -p ~/.ssh
chmod 700 ~/.ssh

if [[ -n "${KNOWN_HOSTS:-}" ]]; then
  echo "$KNOWN_HOSTS" >> ~/.ssh/known_hosts
  chmod 644 ~/.ssh/known_hosts
  echo "✅ Added pre-verified host keys"
else
  echo "❌ SSH_KNOWN_HOSTS environment variable not set!"
  echo "To get your host keys, run: ssh-keyscan -H $REMOTE_HOST"
  exit 1
fi


if [[ -f ~/.ssh/id_ed25519 ]]; then
  chmod 600 ~/.ssh/id_ed25519
fi
echo "[2/6] Stripping binary..."
strip target/release/$APP_NAME

echo "[3/6] Compressing binary..."
tar czf $APP_NAME.tar.gz -C target/release $APP_NAME

BACKUP_DIR="backup_$(date +%F_%T)"

echo "[4/6] Uploading with rsync..."
rsync -az --progress --partial --backup --backup-dir=$BACKUP_DIR \
  $APP_NAME.tar.gz "$REMOTE_USER@$REMOTE_HOST:$REMOTE_DIR/"

echo "[5/6] Extracting and restarting on server..."
ssh "$REMOTE_USER@$REMOTE_HOST" bash <<EOF
  set -e
  cd "$REMOTE_DIR"
  
  rollback() {
    echo "❌ Deployment failed! Rolling back..."
    if [[ -d "$BACKUP_DIR" && -f "$BACKUP_DIR/$APP_NAME.tar.gz" ]]; then
      echo "Restoring from rsync backup..."
      tar xzf "$BACKUP_DIR/$APP_NAME.tar.gz"
      chmod +x "$APP_NAME"
      echo "Restarting service with previous version..."
      sudo systemctl restart $SYSTEMD_SERVICE || true
      echo "Rollback complete - service running with previous version"
      echo "Error logs from failed deployment:"
      journalctl -u $SYSTEMD_SERVICE -n 10 --no-pager
    else
      echo "No rsync backup found to rollback to"
    fi
    exit 1
  }
  
  trap rollback ERR
  
  echo "Stopping service..."
  sudo systemctl stop $SYSTEMD_SERVICE || true
  
  echo "Extracting new binary..."
  tar xzf $APP_NAME.tar.gz
  rm $APP_NAME.tar.gz
  
  # Make binary executable
  chmod +x $APP_NAME
  
  echo "Testing service restart..."
  sudo systemctl restart $SYSTEMD_SERVICE
  
  sleep 2
  
  if sudo systemctl is-active --quiet $SYSTEMD_SERVICE; then
    echo "✅ Service started successfully!"
    rm -rf "$BACKUP_DIR"
    echo "Recent logs:"
    journalctl -u $SYSTEMD_SERVICE -n 5 --no-pager
  else
    echo "Service failed to start properly"
    exit 1
  fi
  
  trap - ERR
EOF

echo "✅ Deploy complete!"
