#!/bin/bash
#
# scripts/release.sh — ビルド済みアプリを GitHub Release にアップロードする
#
# 使い方:
#   ./scripts/release.sh [--tag v0.1.0] [--notes "リリースノート"]
#
# オプションを省略した場合:
#   --tag    src-tauri/tauri.conf.json のバージョンを使用
#   --notes  空欄
#
# 動作:
#   1. src-tauri/target/release/bundle/macos/*.app を zip 化
#   2. 指定タグの GitHub Release が存在しなければ作成 (pre-release)
#   3. 同名アセットが既に存在する場合は削除してから再アップロード
#
# 必要なもの: gh (GitHub CLI), zip
#
set -euo pipefail

REPO="k-hashimoto/tiny_data_warehouse"
BUNDLE_DIR="src-tauri/target/release/bundle/macos"
APP_NAME="TinyDataWarehouse.app"
ARCH=$(uname -m)  # arm64 or x86_64
ARCH_LABEL="${ARCH/arm64/aarch64}"
ARCH_LABEL="${ARCH_LABEL/x86_64/x86_64}"

# --- 引数パース ---
TAG=""
NOTES=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --tag)   TAG="$2";   shift 2 ;;
        --notes) NOTES="$2"; shift 2 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

# バージョン取得
if [[ -z "$TAG" ]]; then
    VERSION=$(python3 -c "import json; c=json.load(open('src-tauri/tauri.conf.json')); print(c.get('version', c.get('package',{}).get('version','')))")
    TAG="${VERSION}"
fi

APP_PATH="${BUNDLE_DIR}/${APP_NAME}"
if [[ ! -d "$APP_PATH" ]]; then
    echo "Error: ${APP_PATH} が見つかりません。先に task build を実行してください。" >&2
    exit 1
fi

# zip ファイル名: TinyDataWarehouse_0.1.0_aarch64.zip
ZIP_NAME="TinyDataWarehouse_${TAG}_${ARCH_LABEL}.zip"
TMP_ZIP="/tmp/${ZIP_NAME}"

echo "==> アーカイブを作成中: ${ZIP_NAME}"
# -y: シンボリックリンクをそのまま保持 (macOS .app に必要)
(cd "${BUNDLE_DIR}" && zip -qry "${TMP_ZIP}" "${APP_NAME}")

echo "==> GitHub Release を確認中 (tag: ${TAG})..."
if gh release view "${TAG}" --repo "${REPO}" &>/dev/null; then
    echo "    既存のリリースが見つかりました"

    # 同名アセットが存在すれば削除
    if gh release view "${TAG}" --repo "${REPO}" --json assets --jq '.[].name' 2>/dev/null | grep -qF "${ZIP_NAME}"; then
        echo "==> 既存アセット '${ZIP_NAME}' を削除中..."
        gh release delete-asset "${TAG}" "${ZIP_NAME}" --repo "${REPO}" --yes
    fi
else
    echo "==> リリースを新規作成中 (pre-release)..."
    gh release create "${TAG}" \
        --repo "${REPO}" \
        --title "${TAG}" \
        --notes "${NOTES}" \
        --prerelease
fi

echo "==> アセットをアップロード中..."
gh release upload "${TAG}" "${TMP_ZIP}" --repo "${REPO}" --clobber

rm -f "${TMP_ZIP}"

echo ""
echo "Done! https://github.com/${REPO}/releases/tag/${TAG}"
