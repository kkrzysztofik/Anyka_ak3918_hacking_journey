#!/bin/bash

###############################################################################
# Anyka Camera Deployment Script
#
# Deploys compiled ARM binaries to Anyka cameras via SSH or SD card
#
# Usage:
#   ./deploy.sh <camera_ip> [username] [project]
#   ./deploy.sh sdcard /dev/sdX [project]
#
# Examples:
#   ./deploy.sh 192.168.1.100                    # Deploy to 192.168.1.100 as root
#   ./deploy.sh 192.168.1.100 admin              # Deploy as admin user
#   ./deploy.sh 192.168.1.100 root onvif-rust    # Deploy onvif-rust project
#   ./deploy.sh sdcard /dev/sdb                  # Deploy to SD card at /dev/sdb
###############################################################################

set -euo pipefail

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Default values
CAMERA_IP=""
CAMERA_USER="root"
CAMERA_PATH="/anyka_hack"
PROJECT="${3:-onvif-rust}"
DEPLOYMENT_TYPE="ssh"  # ssh or sdcard

# Parse arguments
if [ $# -lt 1 ]; then
    echo -e "${RED}❌ Usage: $0 <camera_ip|sdcard> [username|device] [project]${NC}"
    echo ""
    echo "Examples:"
    echo "  $0 192.168.1.100                    # SSH deploy to camera"
    echo "  $0 192.168.1.100 admin              # SSH as different user"
    echo "  $0 192.168.1.100 root onvif-rust    # Specific project"
    echo "  $0 sdcard /dev/sdb                  # SD card deployment"
    exit 1
fi

if [ "$1" = "sdcard" ]; then
    DEPLOYMENT_TYPE="sdcard"
    SD_DEVICE="${2:?SD device required (e.g., /dev/sdb)}"
else
    CAMERA_IP="$1"
    CAMERA_USER="${2:-root}"
fi

# Verify project directory
PROJECT_DIR="$PROJECT_ROOT/cross-compile/$PROJECT"
if [ ! -d "$PROJECT_DIR" ]; then
    echo -e "${RED}❌ Project directory not found: $PROJECT_DIR${NC}"
    exit 1
fi

echo -e "${YELLOW}📦 Anyka Camera Deployment${NC}"
echo "Project: $PROJECT"
echo "Type: $DEPLOYMENT_TYPE"

###############################################################################
# SSH Deployment
###############################################################################
deploy_via_ssh() {
    local camera_ip="$1"
    local camera_user="$2"
    local project="$3"

    echo -e "${YELLOW}📡 SSH Deployment to $camera_user@$camera_ip${NC}"

    # Find binary
    local binary_path="$PROJECT_DIR/target/armv5te-unknown-linux-uclibceabi/release/$project"

    if [ ! -f "$binary_path" ]; then
        # Try debug version
        binary_path="$PROJECT_DIR/target/armv5te-unknown-linux-uclibceabi/debug/$project"
        if [ ! -f "$binary_path" ]; then
            echo -e "${RED}❌ Binary not found${NC}"
            echo "Searched:"
            echo "  - $PROJECT_DIR/target/armv5te-unknown-linux-uclibceabi/release/$project"
            echo "  - $PROJECT_DIR/target/armv5te-unknown-linux-uclibceabi/debug/$project"
            echo ""
            echo "Did you forget to build? Run:"
            echo "  cd $PROJECT_DIR && cargo build --release --target armv5te-unknown-linux-uclibceabi"
            exit 1
        fi
    fi

    binary_name=$(basename "$binary_path")
    echo "Binary: $binary_path"
    echo "Size: $(du -h "$binary_path" | cut -f1)"

    # Verify SSH connectivity
    if ! ssh -o ConnectTimeout=5 "$camera_user@$camera_ip" "echo 'SSH OK'" > /dev/null 2>&1; then
        echo -e "${RED}❌ Cannot connect to $camera_user@$camera_ip${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ SSH connection verified${NC}"

    # Create remote directory if needed
    ssh "$camera_user@$camera_ip" "mkdir -p $CAMERA_PATH"

    # Copy binary
    echo -e "${YELLOW}📥 Copying binary...${NC}"
    scp -q "$binary_path" "$camera_user@$camera_ip:$CAMERA_PATH/$binary_name"

    # Make executable
    ssh "$camera_user@$camera_ip" "chmod +x $CAMERA_PATH/$binary_name"

    # Optional: stop old process and start new one
    read -p "Start service after deployment? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${YELLOW}🔄 Restarting service...${NC}"
        ssh "$camera_user@$camera_ip" "pkill -9 '$binary_name' || true; sleep 1; nohup $CAMERA_PATH/$binary_name > /tmp/$binary_name.log 2>&1 &"
        echo -e "${GREEN}✅ Service started${NC}"

        # Show logs
        sleep 2
        echo -e "${YELLOW}📋 Recent logs:${NC}"
        ssh "$camera_user@$camera_ip" "tail -10 /tmp/$binary_name.log" || true
    fi

    echo -e "${GREEN}✅ Deployment complete!${NC}"
    echo "Camera: http://$camera_ip:8080/onvif/device_service"
}

###############################################################################
# SD Card Deployment
###############################################################################
deploy_via_sdcard() {
    local sd_device="$1"
    local project="$2"

    echo -e "${YELLOW}💾 SD Card Deployment to $sd_device${NC}"

    # Safety checks
    if [ ! -e "$sd_device" ]; then
        echo -e "${RED}❌ Device not found: $sd_device${NC}"
        exit 1
    fi

    # Verify it's not the system drive
    if [ "$sd_device" = "/dev/sda" ] || [ "$sd_device" = "/dev/nvme0n1" ]; then
        echo -e "${RED}❌ Refusing to write to system drive: $sd_device${NC}"
        exit 1
    fi

    # Check if any partitions are mounted
    if mount | grep -q "$sd_device"; then
        echo -e "${YELLOW}⚠️  SD card is currently mounted${NC}"
        read -p "Unmount automatically? (y/n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            sudo umount "${sd_device}"* 2>/dev/null || true
            sleep 1
        else
            exit 1
        fi
    fi

    # Find binary
    local binary_path="$PROJECT_DIR/target/armv5te-unknown-linux-uclibceabi/release/$project"
    if [ ! -f "$binary_path" ]; then
        binary_path="$PROJECT_DIR/target/armv5te-unknown-linux-uclibceabi/debug/$project"
    fi

    if [ ! -f "$binary_path" ]; then
        echo -e "${RED}❌ Binary not found at $binary_path${NC}"
        exit 1
    fi

    binary_name=$(basename "$binary_path")

    echo "Binary: $binary_path"
    echo "Device: $sd_device"
    echo "Size: $(du -h "$binary_path" | cut -f1)"

    read -p "Continue with SD card deployment? This will modify $sd_device. (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Cancelled."
        exit 0
    fi

    # Mount SD card
    echo -e "${YELLOW}🔧 Mounting SD card partitions...${NC}"

    MOUNT_BOOT="/tmp/sd_boot_$$"
    MOUNT_ROOT="/tmp/sd_root_$$"

    mkdir -p "$MOUNT_BOOT" "$MOUNT_ROOT"

    sudo mount "${sd_device}1" "$MOUNT_BOOT" 2>/dev/null || {
        echo -e "${RED}❌ Failed to mount boot partition${NC}"
        exit 1
    }

    sudo mount "${sd_device}2" "$MOUNT_ROOT" 2>/dev/null || {
        sudo umount "$MOUNT_BOOT"
        echo -e "${RED}❌ Failed to mount root partition${NC}"
        exit 1
    }

    echo -e "${GREEN}✅ SD card mounted${NC}"

    # Copy binary
    echo -e "${YELLOW}📥 Copying binary to SD card...${NC}"
    sudo mkdir -p "$MOUNT_ROOT/anyka_hack"
    sudo cp "$binary_path" "$MOUNT_ROOT/anyka_hack/$binary_name"
    sudo chmod +x "$MOUNT_ROOT/anyka_hack/$binary_name"

    # Copy configuration if exists
    if [ -f "$PROJECT_DIR/config/config.toml" ]; then
        echo -e "${YELLOW}📋 Copying configuration...${NC}"
        sudo cp "$PROJECT_DIR/config/config.toml" "$MOUNT_ROOT/anyka_hack/"
    fi

    # Create/update startup script
    echo -e "${YELLOW}📝 Creating startup script...${NC}"
    sudo tee "$MOUNT_ROOT/anyka_hack/start.sh" > /dev/null << 'EOF'
#!/bin/sh
cd /anyka_hack
exec ./$BINARY_NAME --config config.toml
EOF

    sudo sed -i "s|\$BINARY_NAME|$binary_name|g" "$MOUNT_ROOT/anyka_hack/start.sh"
    sudo chmod +x "$MOUNT_ROOT/anyka_hack/start.sh"

    # Unmount
    echo -e "${YELLOW}🔒 Unmounting SD card...${NC}"
    sudo umount "$MOUNT_BOOT"
    sudo umount "$MOUNT_ROOT"

    sync

    # Cleanup
    rm -rf "$MOUNT_BOOT" "$MOUNT_ROOT"

    echo -e "${GREEN}✅ SD card deployment complete!${NC}"
    echo "Next steps:"
    echo "  1. Insert SD card into camera"
    echo "  2. Power on camera"
    echo "  3. Wait for boot (30-60 seconds)"
    echo "  4. Access camera at http://192.168.1.X:8080"
}

###############################################################################
# Main
###############################################################################

case "$DEPLOYMENT_TYPE" in
    ssh)
        deploy_via_ssh "$CAMERA_IP" "$CAMERA_USER" "$PROJECT"
        ;;
    sdcard)
        deploy_via_sdcard "$SD_DEVICE" "$PROJECT"
        ;;
    *)
        echo -e "${RED}❌ Unknown deployment type: $DEPLOYMENT_TYPE${NC}"
        exit 1
        ;;
esac
