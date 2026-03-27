#!/bin/bash
#
# install.sh — One-line installer for Tiny Data Ware House (macOS)
#
# What this script does:
#   1. Fetches the latest release metadata from the GitHub API.
#   2. Extracts the download URL of the .dmg asset.
#   3. Downloads the .dmg to a temporary file.
#   4. Mounts the disk image using hdiutil.
#   5. Copies the .app bundle to /Applications (replacing any existing version).
#   6. Removes the macOS quarantine flag with `xattr -dr com.apple.quarantine`
#      so the app opens without the "damaged or incomplete" error.
#   7. Unmounts the disk image and cleans up all temporary files.
#
# Requirements: macOS, curl, hdiutil (both are pre-installed on macOS)
#
set -euo pipefail

REPO="k-hashimoto/tiny_data_warehouse"
APP_NAME="Tiny Data Ware House.app"
INSTALL_DIR="/Applications"

echo "Fetching latest release info..."
# Try /releases/latest first (stable releases only).
# If it returns no tag (e.g. only pre-releases exist), fall back to /releases
# which includes pre-releases, and pick the first (most recent) entry.
RELEASE_JSON=$(curl -sSL "https://api.github.com/repos/${REPO}/releases/latest")
if ! echo "$RELEASE_JSON" | grep -q '"tag_name"'; then
    RELEASE_JSON=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases" | \
        python3 -c "import sys,json; r=json.load(sys.stdin); print(json.dumps(r[0]))")
fi

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
