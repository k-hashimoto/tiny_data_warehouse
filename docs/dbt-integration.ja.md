# dbt 連携ガイド

このガイドでは、[dbt](https://www.getdbt.com/) を Tiny Data Warehouse と連携させる方法と、テーブルをアプリに自動反映させる手順を説明します。

---

## 仕組み

Tiny Data Warehouse は `~/.tdwh/db/dbt.db` のファイル変更を監視しています。`dbt run` が完了してファイルが更新されると、エクスプローラーパネルが自動でリフレッシュされます。手動でのリロードは不要です。

```
dbt run → ~/.tdwh/db/dbt.db に書き込み → アプリが変更を検知 → エクスプローラー更新
```

---

## 前提条件

### 1. dbt（DuckDB アダプター付き）をインストール

```bash
pip install dbt-duckdb
```

インストールの確認：

```bash
dbt --version
```

### 2. アプリのデータディレクトリを確認

`~/.tdwh/db/` はアプリの初回起動時に自動作成されます。dbt を実行する前に、アプリを一度起動しておいてください。

```bash
ls ~/.tdwh/db/
# app.db   dbt.db（初回の dbt run 後に作成されます）
```

---

## プロジェクトのセットアップ

### 方法 A：同梱のサンプルプロジェクトを使う

[`dbt_examples/`](../dbt_examples/) ディレクトリにサンプルの dbt プロジェクトが含まれています。

```bash
cd dbt_examples
```

### 方法 B：新規プロジェクトを作成する

```bash
dbt init my_project
cd my_project
```

---

## profiles.yml の設定

dbt は接続設定を `~/.dbt/profiles.yml`（グローバル）または `./profiles.yml`（プロジェクトローカル）から読み込みます。

以下の内容でファイルを作成または編集してください：

```yaml
my_project:
  target: dev
  outputs:
    dev:
      type: duckdb
      path: ~/.tdwh/db/dbt.db
      threads: 1
```

> **注意:** `my_project` は `dbt_project.yml` の `profile:` の値に合わせてください。

プロジェクトローカルの `profiles.yml` を使う場合は、dbt コマンドに `--profiles-dir .` を追加します：

```bash
dbt run --profiles-dir .
```

---

## モデルの書き方

モデルは `models/` ディレクトリに配置した `.sql` ファイルです。dbt がコンパイルして DuckDB に対して実行します。

**例 — `models/staging/stg_orders.sql`:**

```sql
{{ config(materialized='table') }}

select
    order_id,
    customer_id,
    order_date,
    status
from {{ ref('raw_orders') }}
```

**マテリアライゼーションの種類:**

| 種類 | 説明 |
|------|------|
| `view` | ビューを作成（高速、データコピーなし） |
| `table` | 物理テーブルを作成（ビルドは遅いがクエリは高速） |
| `incremental` | 新しい行のみを追記・マージ |

---

## テーブルメタデータを書き込む

dbt モデルの `config` ブロックで `post_hook` を使うと、テーブルやカラムにコメント（メタデータ）を付与できます。付与したコメントは Tiny Data Warehouse のメタデータパネルに表示されます。

**例 — `models/examples/stg_sales.sql`:**

```sql
{{
    config(
      materialized='table',
      schema='staging',
      alias='stg_sales',
      post_hook=[
        "COMMENT ON TABLE {{ this }} IS '売上データのサマリーテーブル'",
        "COMMENT ON COLUMN {{ this }}.sale_id IS '売上レコードの一意識別子'",
        "COMMENT ON COLUMN {{ this }}.user_id IS 'ユーザーの一意識別子'",
        "COMMENT ON COLUMN {{ this }}.product_name IS '商品名'",
        "COMMENT ON COLUMN {{ this }}.revenue IS '売上金額（円）'",
        "COMMENT ON COLUMN {{ this }}.sale_date IS '売上発生日'"
      ]
    )
}}

SELECT
    1 AS sale_id, 101 AS user_id, 'ノートPC'  AS product_name, 120000 AS revenue, DATE '2025-01-10' AS sale_date

UNION ALL SELECT 2, 102, 'マウス',       3500, DATE '2025-01-15'
UNION ALL SELECT 3, 101, 'キーボード',  8000, DATE '2025-02-01'
UNION ALL SELECT 4, 103, 'モニター',   45000, DATE '2025-02-20'
UNION ALL SELECT 5, 102, 'USBハブ',    2800, DATE '2025-03-05'
```

**ポイント:**

- `post_hook` にはリスト形式で複数の SQL 文を記述できます。モデルのビルド完了後に順番に実行されます。
- `{{ this }}` は dbt の変数で、現在のモデルのテーブル名（`スキーマ.テーブル名`）に展開されます。
- `COMMENT ON TABLE` でテーブル全体の説明、`COMMENT ON COLUMN` でカラムごとの説明を設定します。
- `dbt run` 後、Tiny Data Warehouse のエクスプローラーでテーブル名横の ℹ️ ボタンをクリックすると、メタデータパネルにコメントが表示されます。

---

## よく使う dbt コマンド

| コマンド | 説明 |
|---------|------|
| `dbt run` | 全モデルをビルドして `dbt.db` に書き込む |
| `dbt run --select my_model` | 特定のモデルだけビルド |
| `dbt run --select staging.*` | ディレクトリ内の全モデルをビルド |
| `dbt test` | データ品質テストを実行 |
| `dbt seed` | `seeds/` の CSV ファイルをテーブルとして読み込む |
| `dbt compile` | 実行せずに SQL をコンパイルのみ |
| `dbt docs generate` | ドキュメントサイトを生成 |
| `dbt clean` | コンパイル済みファイルを削除（`target/`、`dbt_packages/`） |

### 実行してすぐアプリで確認する

```bash
dbt run --profiles-dir .
# Tiny Data Warehouse のエクスプローラーが自動でリフレッシュされます
```

### 単一モデルを実行する

```bash
dbt run --select stg_orders --profiles-dir .
```

### 依存関係を含めてモデルを実行する

```bash
# stg_orders と、それに依存する全モデルを実行
dbt run --select stg_orders+ --profiles-dir .

# stg_orders と、その依存元の全モデルを実行
dbt run --select +stg_orders --profiles-dir .
```

---

## Tiny Data Warehouse で結果を確認する

`dbt run` が完了すると：

1. エクスプローラーパネルが自動でリフレッシュされます。
2. `dbt_project.yml` で定義したスキーマ名（例：`staging`、`marts`）の下にテーブルが表示されます。
3. テーブルをクリックして内容を確認したり、クエリを実行したりできます。

---

## トラブルシューティング

**`dbt run` 後もテーブルが表示されない**

- `profiles.yml` の `path` が `~/.tdwh/db/dbt.db` を指しているか確認してください。
- dbt の出力にエラーがないか確認してください（`dbt run` の終了コードが 0 である必要があります）。
- アプリを再起動してエクスプローラーを確認してください。

**`dbt: command not found`**

- `pip install dbt-duckdb` でインストール済みか確認してください。
- 仮想環境を使っている場合は、dbt 実行前に有効化してください。

**`profile 'my_project' not found`**

- `dbt_project.yml` の `profile:` の値が `profiles.yml` のトップレベルのキーと一致しているか確認してください。
- `profiles.yml` がプロジェクトディレクトリにある場合は `--profiles-dir .` を付けて実行してください。

---

## 参考リンク

- [dbt ドキュメント](https://docs.getdbt.com/)
- [dbt-duckdb アダプター](https://github.com/duckdb/dbt-duckdb)
- [DuckDB SQL リファレンス](https://duckdb.org/docs/sql/introduction)
