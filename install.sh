#!/bin/bash
set -euo pipefail

REPO="k-hashimoto/tiny_data_warehouse"
APP_NAME="Tiny Data Ware House.app"
INSTALL_DIR="/Applications"

echo "Fetching latest release info..."
RELEASE_JSON=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest")

VERSION=$(echo "$RELEASE_JSON" | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
DMG_URL=$(echo "$RELEASE_JSON" | grep '"browser_download_url"' | grep '\.dmg"' | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/')

if [[ -z "$DMG_URL" ]]; then
    echo "Error: Could not find .dmg asset in the latest release." >&2
    exit 1
fi

echo "Latest version: ${VERSION}"
echo "Downloading: $(basename "$DMG_URL")"

TMP_DMG=$(mktemp /tmp/tdwh_XXXXXX.dmg)
trap 'rm -f "$TMP_DMG"' EXIT

curl -fL --progress-bar -o "$TMP_DMG" "$DMG_URL"

echo "Mounting disk image..."
MOUNT_POINT=$(mktemp -d /tmp/tdwh_mount_XXXXXX)
hdiutil attach "$TMP_DMG" -mountpoint "$MOUNT_POINT" -nobrowse -quiet

cleanup_mount() {
    hdiutil detach "$MOUNT_POINT" -quiet 2>/dev/null || true
    rmdir "$MOUNT_POINT" 2>/dev/null || true
    rm -f "$TMP_DMG"
}
trap cleanup_mount EXIT

echo "Installing to ${INSTALL_DIR}..."
if [[ -d "${INSTALL_DIR}/${APP_NAME}" ]]; then
    rm -rf "${INSTALL_DIR}/${APP_NAME}"
fi
cp -R "${MOUNT_POINT}/${APP_NAME}" "${INSTALL_DIR}/"

echo "Removing quarantine flag..."
xattr -dr com.apple.quarantine "${INSTALL_DIR}/${APP_NAME}"

echo ""
echo "Done! Tiny Data Ware House ${VERSION} has been installed."
