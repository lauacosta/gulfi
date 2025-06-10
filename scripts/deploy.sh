#!/usr/bin/env bash
set -euo pipefail

APP_NAME="gulfi"
REMOTE_USER="${SERVER_USER:-}"
REMOTE_HOST="${SERVER_HOST:-}"
REMOTE_DIR="/home/$REMOTE_USER/$APP_NAME"
SYSTEMD_SERVICE="$APP_NAME.service"

echo "[2/5] Stripping binary..."
strip target/release/$APP_NAME

echo "[3/5] Compressing binary..."
tar czf $APP_NAME.tar.gz -C target/release $APP_NAME

BACKUP_DIR="backup_$(date +%F_%T)"
echo "[4/5] Uploading with rsync..."
rsync -az --progress --partial --backup --backup-dir=$BACKUP_DIR \
  $APP_NAME.tar.gz "$REMOTE_USER@$REMOTE_HOST:$REMOTE_DIR/"

echo "[5/5] Extracting and restarting on server..."
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
