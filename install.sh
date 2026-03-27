#!/bin/bash
#
# install.sh — One-line installer for Tiny Data Ware House (macOS)
#
# What this script does:
#   1. Fetches the latest release metadata from the GitHub API.
#   2. Extracts the download URL of the .zip asset.
#   3. Downloads the .zip to a temporary file.
#   4. Unzips the archive to extract the .app bundle.
#   5. Copies the .app bundle to /Applications (replacing any existing version).
#   6. Removes the macOS quarantine flag with `xattr -dr com.apple.quarantine`
#      so the app opens without the "damaged or incomplete" error.
#   7. Cleans up all temporary files.
#
# Requirements: macOS, curl, unzip (both are pre-installed on macOS)
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
ZIP_URL=$(echo "$RELEASE_JSON" | grep '"browser_download_url"' | grep '\.zip"' | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/')

if [[ -z "$ZIP_URL" ]]; then
    echo "Error: Could not find .zip asset in the latest release." >&2
    exit 1
fi

echo "Latest version: ${VERSION}"
echo "Downloading: $(basename "$ZIP_URL")"

TMP_ZIP=$(mktemp /tmp/tdwh_XXXXXX.zip)
TMP_DIR=$(mktemp -d /tmp/tdwh_extract_XXXXXX)

cleanup() {
    rm -f "$TMP_ZIP"
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

curl -fL --progress-bar -o "$TMP_ZIP" "$ZIP_URL"

echo "Extracting archive..."
unzip -q "$TMP_ZIP" -d "$TMP_DIR"

echo "Installing to ${INSTALL_DIR}..."
if [[ -d "${INSTALL_DIR}/${APP_NAME}" ]]; then
    rm -rf "${INSTALL_DIR}/${APP_NAME}"
fi
cp -R "${TMP_DIR}/${APP_NAME}" "${INSTALL_DIR}/"

echo "Removing quarantine flag..."
xattr -dr com.apple.quarantine "${INSTALL_DIR}/${APP_NAME}"

echo ""
echo "Done! Tiny Data Ware House ${VERSION} has been installed."
