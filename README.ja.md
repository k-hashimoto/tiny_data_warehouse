# Tiny Data Warehouse

[DuckDB](https://duckdb.org/) を搭載した軽量デスクトップ SQL クライアントです。[Tauri](https://tauri.app/) と React で構築されており、個人のデータ探索・ローカル分析、そして [dbt](https://www.getdbt.com/) との連携を想定して設計されています。

> 🇺🇸 [English README is here](./README.md)

---

## 機能

- **SQL エディタ** — Monaco ベースのエディタ。シンタックスハイライト、マルチタブ、クエリ履歴に対応
- **テーブルエクスプローラー** — ローカル DuckDB データベースのスキーマとテーブルをツリー表示
- **CSV インポート / エクスポート** — CSV ファイルをテーブルとして取り込み、クエリ結果を CSV で書き出し
- **dbt 連携** — dbt の出力データベース（`dbt.db`）の変更をリアルタイムで自動検出・反映
- **スクリプト管理** — よく使う SQL をスクリプトとして保存・リネーム・再利用
- **テーブルメタデータ** — テーブルやカラムにコメントを付けてドキュメント化
- **ダークモード** — ライト / ダークテーマの切り替え
- **リサイズ可能なレイアウト** — パネルをドラッグして作業スペースをカスタマイズ

---

## 技術スタック

| レイヤー | 技術 |
|---|---|
| デスクトップフレームワーク | [Tauri 2](https://tauri.app/) |
| データベースエンジン | [DuckDB](https://duckdb.org/) |
| フロントエンド | React 19 + TypeScript |
| SQL エディタ | [Monaco Editor](https://microsoft.github.io/monaco-editor/) |
| UI コンポーネント | [shadcn/ui](https://ui.shadcn.com/) + Tailwind CSS v4 |
| 状態管理 | [Zustand](https://zustand-demo.pmnd.rs/) |
| ビルドツール | [Vite](https://vitejs.dev/) |

---

## セットアップ

### 必要環境

- [Node.js](https://nodejs.org/)（v18 以降）
- [Rust](https://www.rust-lang.org/tools/install)（stable ツールチェーン）
- OS に応じた [Tauri CLI の依存関係](https://tauri.app/start/prerequisites/)

### 開発サーバーの起動

```bash
# 依存関係のインストール
npm install

# 開発サーバー起動（アプリウィンドウが自動的に開きます）
npm run tauri dev
```

### ビルド

```bash
# リリースバイナリのビルド
npm run tauri build

# または Taskfile を使う場合
task build
```

ビルドされたアプリは `src-tauri/target/release/bundle/` に出力されます。

---

## データの保存場所

すべてのデータはローカルの `~/.tdwh/` 以下に保存されます：

```
~/.tdwh/
└── db/
    ├── app.db   # メインの DuckDB データベース
    └── dbt.db   # dbt の出力データベース（自動検出）
```

外部サーバーへのデータ送信は一切ありません。

---

## dbt 連携について

Tiny Data Warehouse は `~/.tdwh/db/dbt.db` の変更を監視しています。`dbt run` が完了すると、エクスプローラーパネルが自動的に更新され、最新のモデルが反映されます。手動でのリロードは不要です。

dbt プロジェクトを連携させるには、dbt の出力先（`profiles.yml` の `path` 設定）を `~/.tdwh/db/` に向けるように設定してください。

---

## プロジェクト構成

```
tiny_data_warehouse/
├── src/                        # React フロントエンド
│   ├── App.tsx                 # ルートレイアウト（リサイズパネル）
│   ├── components/
│   │   ├── Explorer/           # テーブルツリー、dbt セクション、スクリプト一覧
│   │   ├── QueryEditor/        # Monaco エディタ、タブバー
│   │   ├── ResultsPanel/       # クエリ結果テーブル
│   │   ├── QueryHistory/       # 履歴ビューア
│   │   ├── CsvImport/          # CSV インポートダイアログ
│   │   └── StatusBar/          # 下部ステータスバー
│   └── store/                  # Zustand 状態管理
├── src-tauri/                  # Rust バックエンド（Tauri）
│   └── src/
│       ├── commands/           # Tauri コマンドハンドラー
│       │   ├── query.rs        # SQL 実行
│       │   ├── explorer.rs     # テーブル / スキーマ一覧
│       │   ├── csv.rs          # CSV インポート / エクスポート
│       │   ├── scripts.rs      # スクリプト管理
│       │   ├── metadata.rs     # テーブル / カラムコメント
│       │   └── config.rs       # エディタ設定
│       └── db/
│           ├── worker.rs       # 非同期 DuckDB ワーカースレッド
│           ├── connection.rs   # DuckDB コネクションラッパー
│           └── types.rs        # 共有データ型
└── Taskfile.yml                # タスクランナー
```

---

## ライセンス

MIT
