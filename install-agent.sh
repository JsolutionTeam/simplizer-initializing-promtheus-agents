#!/bin/bash

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

log() { echo -e "${CYAN}📋 [$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"; }
success() { echo -e "${GREEN}✅ $1${NC}"; }
warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
error() { echo -e "${RED}❌ [ERROR] $1${NC}" >&2; exit 1; }

if [[ $EUID -ne 0 ]]; then
   error "This script must be run as root (use sudo)"
fi

VERSION="v0.1.7"
REPO="JsolutionTeam/simplizer-initializing-promtheus-agents"
BINARY_NAME="process-prometheus-linux-amd64"
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/$BINARY_NAME"
INSTALL_DIR="/opt/prometheus"
SERVICE_NAME="prometheus-agents"
LOG_FILE="/var/log/prometheus-agents-install.log"

exec 1> >(tee -a "$LOG_FILE")
exec 2>&1

echo -e "${MAGENTA}${BOLD}╔════════════════════════════════════════════╗${NC}"
echo -e "${MAGENTA}${BOLD}║     Prometheus Agents Installer ${VERSION}     ║${NC}"
echo -e "${MAGENTA}${BOLD}╚════════════════════════════════════════════╝${NC}"
echo ""

log "Starting installation of Prometheus Agents ${GREEN}$VERSION${NC}"

if systemctl is-active --quiet $SERVICE_NAME; then
    warning "Stopping existing service..."
    systemctl stop $SERVICE_NAME
fi

info "Creating installation directory: ${YELLOW}$INSTALL_DIR${NC}"
mkdir -p $INSTALL_DIR
cd $INSTALL_DIR

log "Downloading from: ${BLUE}$DOWNLOAD_URL${NC}"
echo -e "${YELLOW}⬇️  Downloading binary...${NC}"
if ! curl -LO $DOWNLOAD_URL; then
    error "Failed to download binary"
fi
success "Download completed!"

chmod +x $BINARY_NAME
success "Binary permissions set"

echo -e "${YELLOW}🔧 Running initial setup...${NC}"
if ! ./$BINARY_NAME; then
    warning "Initial setup may have encountered issues"
fi

echo -e "${BLUE}📝 Creating systemd service...${NC}"
cat > /etc/systemd/system/$SERVICE_NAME.service << EOF
[Unit]
Description=Prometheus Agents Setup
After=network.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/$BINARY_NAME
Restart=always
RestartSec=10
User=root
WorkingDirectory=$INSTALL_DIR
StandardOutput=journal
StandardError=journal
SyslogIdentifier=$SERVICE_NAME

LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
EOF
success "Service file created"

echo -e "${YELLOW}🚀 Enabling and starting service...${NC}"
systemctl daemon-reload
systemctl enable $SERVICE_NAME
systemctl start $SERVICE_NAME

echo -e "${CYAN}⏳ Verifying service status...${NC}"
sleep 2
if systemctl is-active --quiet $SERVICE_NAME; then
    success "Service is running successfully!"
else
    error "Service failed to start. Check: journalctl -u $SERVICE_NAME"
fi

echo ""
echo -e "${GREEN}${BOLD}╔════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}${BOLD}║    🎉 Installation Completed Successfully! ║${NC}"
echo -e "${GREEN}${BOLD}╚════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${CYAN}📊 Service Status:${NC}"
systemctl status $SERVICE_NAME --no-pager

echo ""
echo -e "${MAGENTA}${BOLD}Useful Commands:${NC}"
echo -e "  ${CYAN}►${NC} Check status:  ${YELLOW}sudo systemctl status $SERVICE_NAME${NC}"
echo -e "  ${CYAN}►${NC} View logs:     ${YELLOW}sudo journalctl -u $SERVICE_NAME -f${NC}"
echo -e "  ${CYAN}►${NC} Restart:       ${YELLOW}sudo systemctl restart $SERVICE_NAME${NC}"
echo -e "  ${CYAN}►${NC} Stop:          ${YELLOW}sudo systemctl stop $SERVICE_NAME${NC}"