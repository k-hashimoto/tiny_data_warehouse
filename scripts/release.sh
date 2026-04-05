#!/bin/bash
#
# scripts/release.sh — バージョン更新・ビルド済みアプリのGitHub Releaseアップロード
#
# 使い方:
#   ./scripts/release.sh --prepare          # バージョンファイルを更新してビルド準備
#   ./scripts/release.sh                    # ビルド済みバイナリをアップロード
#   ./scripts/release.sh [--tag 0.1.1]      # タグを明示指定してアップロード
#
# リリースフロー:
#   1. staging→main PRをマージ → create-release ワークフローが自動でリリース作成
#   2. git pull
#   3. ./scripts/release.sh --prepare       # tauri.conf.json / Cargo.toml を更新
#   4. task build                           # 正しいバージョンでビルド
#   5. ./scripts/release.sh                 # バイナリをアップロード
#
# 必要なもの: gh (GitHub CLI), zip, python3
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
PREPARE=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --tag)     TAG="$2"; shift 2 ;;
        --prepare) PREPARE=true; shift ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

# バージョン取得（最新のGitHub Releaseタグを使用）
if [[ -z "$TAG" ]]; then
    TAG=$(gh release list --repo "${REPO}" --limit 1 --json tagName --jq '.[0].tagName' 2>/dev/null || echo "")
    if [[ -z "$TAG" ]]; then
        echo "Error: GitHub Releaseが見つかりません。--tag オプションでタグを指定してください。" >&2
        exit 1
    fi
    echo "==> 最新リリースタグを使用: ${TAG}"
fi

# --prepare モード: バージョンファイルを更新してビルド準備
if [[ "$PREPARE" == "true" ]]; then
    echo "==> tauri.conf.json と Cargo.toml のバージョンを ${TAG} に更新中..."

    # tauri.conf.json
    python3 - <<PYEOF
import json, re

with open("src-tauri/tauri.conf.json", "r") as f:
    content = f.read()

data = json.loads(content)
data["version"] = "${TAG}"

with open("src-tauri/tauri.conf.json", "w") as f:
    json.dump(data, f, indent=2, ensure_ascii=False)
    f.write("\n")
PYEOF

    # Cargo.toml (src-tauri)
    python3 - <<PYEOF
import re

with open("src-tauri/Cargo.toml", "r") as f:
    content = f.read()

# [package] セクションのversion行を更新
content = re.sub(
    r'^(version\s*=\s*)"[^"]+"',
    r'\1"${TAG}"',
    content,
    count=1,
    flags=re.MULTILINE
)

with open("src-tauri/Cargo.toml", "w") as f:
    f.write(content)
PYEOF

    echo "    tauri.conf.json: version = ${TAG}"
    echo "    src-tauri/Cargo.toml: version = ${TAG}"

    # バージョン変更をコミット＆プッシュ
    echo ""
    git add src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
    if git diff --cached --quiet; then
        echo "==> バージョンは既に ${TAG} です。コミットをスキップします。"
    else
        echo "==> バージョン変更をコミット中..."
        git commit -m "chore: bump version to ${TAG}"
        git push
    fi
    echo ""
    echo "次のステップ:"
    echo "  task build"
    echo "  ./scripts/release.sh"
    exit 0
fi

# アップロードモード
APP_PATH="${BUNDLE_DIR}/${APP_NAME}"
if [[ ! -d "$APP_PATH" ]]; then
    echo "Error: ${APP_PATH} が見つかりません。先に task build を実行してください。" >&2
    exit 1
fi

# ビルドされたバイナリのバージョンと一致するか確認
BUILT_VERSION=$(python3 -c "import json; c=json.load(open('src-tauri/tauri.conf.json')); print(c.get('version', ''))")
if [[ "$BUILT_VERSION" != "$TAG" ]]; then
    echo "Warning: tauri.conf.json のバージョン (${BUILT_VERSION}) がタグ (${TAG}) と一致しません。" >&2
    echo "  先に ./scripts/release.sh --prepare && task build を実行してください。" >&2
    exit 1
fi

# zip ファイル名: TinyDataWarehouse_0.1.1_aarch64.zip
ZIP_NAME="TinyDataWarehouse_${TAG}_${ARCH_LABEL}.zip"
TMP_ZIP="/tmp/${ZIP_NAME}"

echo "==> アーカイブを作成中: ${ZIP_NAME}"
# -y: シンボリックリンクをそのまま保持 (macOS .app に必要)
(cd "${BUNDLE_DIR}" && zip -qry "${TMP_ZIP}" "${APP_NAME}")

echo "==> GitHub Release を確認中 (tag: ${TAG})..."
if ! gh release view "${TAG}" --repo "${REPO}" &>/dev/null; then
    echo "Error: GitHub Release '${TAG}' が見つかりません。" >&2
    echo "mainへのマージ後に create-release ワークフローがリリースを自動作成します。" >&2
    exit 1
fi
echo "    リリース '${TAG}' が見つかりました"

# 同名アセットが存在すれば削除
if gh release view "${TAG}" --repo "${REPO}" --json assets --jq '.[].name' 2>/dev/null | grep -qF "${ZIP_NAME}"; then
    echo "==> 既存アセット '${ZIP_NAME}' を削除中..."
    gh release delete-asset "${TAG}" "${ZIP_NAME}" --repo "${REPO}" --yes
fi

echo "==> アセットをアップロード中..."
gh release upload "${TAG}" "${TMP_ZIP}" --repo "${REPO}" --clobber

rm -f "${TMP_ZIP}"

echo ""
echo "Done! https://github.com/${REPO}/releases/tag/${TAG}"
