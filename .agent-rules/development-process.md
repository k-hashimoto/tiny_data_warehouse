# 開発プロセス

## 初回セットアップ時の手順

### 1. フォルダ作成
```bash
mkdir -p docs_for_ai
mkdir -p .steering
```

### 2. 永続的ドキュメント作成（`docs_for_ai/`）

アプリケーション全体の設計を定義します。
各ドキュメントを作成後、必ず確認・承認を得てから次に進みます。

1. `docs_for_ai/product-requirements.md` - プロダクト要求定義書
2. `docs_for_ai/functional-design.md` - 機能設計書
3. `docs_for_ai/architecture.md` - 技術仕様書
4. `docs_for_ai/repository-structure.md` - リポジトリ構造定義書
5. `docs_for_ai/development-guidelines.md` - 開発ガイドライン
6. `docs_for_ai/glossary.md` - ユビキタス言語定義

**重要：** 1ファイルごとに作成後、必ず確認・承認を得てから次のファイル作成を行う

### 3. 初回実装用のステアリングファイル作成

初回実装用のディレクトリを作成し、実装に必要なドキュメントを配置します。

```bash
mkdir -p .steering/[YYYYMMDD]-initial-implementation
```

作成するドキュメント：
1. `.steering/[YYYYMMDD]-initial-implementation/requirements.md` - 初回実装の要求
2. `.steering/[YYYYMMDD]-initial-implementation/design.md` - 実装設計
3. `.steering/[YYYYMMDD]-initial-implementation/tasklist.md` - 実装タスク

### 4. 環境セットアップ

### 5. 実装開始

`Codex MCP` を呼び出し、`.steering/[YYYYMMDD]-initial-implementation/tasklist.md` と `design.md` の内容を渡して実装を委譲します。

```
mcp__codex__codex:
  prompt: tasklist.md + design.md の内容
  cwd: プロジェクトルートの絶対パス
  sandbox: workspace-write
  approval-policy: on-failure
  base-instructions: docs_for_ai/development-guidelines.md の内容
```

### 6. 品質チェック

---

## 機能追加・修正時の手順

### 1. 影響分析

- 永続的ドキュメント（`docs_for_ai/`）への影響を確認
- 変更が基本設計に影響する場合は `docs_for_ai/` を更新

### 2. フィーチャーブランチ作成

実装作業を開始する前に、`staging` から新しいブランチを作成します。

```bash
git checkout staging
git pull
git checkout -b feature/[開発タイトル]
```

**重要:** Codex MCP を呼び出す前に必ずこの手順を完了すること。`staging` ブランチで直接作業しない。

### 3. ステアリングディレクトリ作成

新しい作業用のディレクトリを作成します。

```bash
mkdir -p .steering/[YYYYMMDD]-[開発タイトル]
```

**例：**
```bash
mkdir -p .steering/20250115-add-tag-feature
```

### 4. 作業ドキュメント作成

作業単位のドキュメントを作成します。
各ドキュメント作成後、必ず確認・承認を得てから次に進みます。

1. `.steering/[YYYYMMDD]-[開発タイトル]/requirements.md` - 要求内容
2. `.steering/[YYYYMMDD]-[開発タイトル]/design.md` - 設計
3. `.steering/[YYYYMMDD]-[開発タイトル]/tasklist.md` - タスクリスト

**重要：** 1ファイルごとに作成後、必ず確認・承認を得てから次のファイル作成を行う

### 5. 永続的ドキュメント更新（必要な場合のみ）

変更が基本設計に影響する場合、該当する `docs_for_ai/` 内のドキュメントを更新します。

### 6. 実装開始

`Codex MCP` を呼び出し、`.steering/[YYYYMMDD]-[開発タイトル]/tasklist.md` と `design.md` の内容を渡して実装を委譲します。

```
mcp__codex__codex:
  prompt: tasklist.md + design.md の内容
  cwd: プロジェクトルートの絶対パス
  sandbox: workspace-write
  approval-policy: on-failure
  base-instructions: docs_for_ai/development-guidelines.md の内容
```

### 7. 品質チェック
