# dbt Integration Guide

This guide explains how to set up [dbt](https://www.getdbt.com/) with Tiny Data Warehouse and use it to build tables that appear automatically in the app.

---

## How it works

Tiny Data Warehouse watches `~/.tdwh/db/dbt.db` for file changes. When `dbt run` completes and writes to that file, the Explorer panel refreshes automatically — no manual reload needed.

```
dbt run → writes to ~/.tdwh/db/dbt.db → app detects change → Explorer updates
```

---

## Prerequisites

### 1. Install dbt with the DuckDB adapter

```bash
pip install dbt-duckdb
```

Verify the installation:

```bash
dbt --version
```

### 2. Confirm the app data directory exists

The app creates `~/.tdwh/db/` on first launch. Start the app at least once before running dbt.

```bash
ls ~/.tdwh/db/
# app.db   dbt.db (created after first dbt run)
```

---

## Project setup

### Option A: Use the bundled example project

A sample dbt project is included in the [`dbt_examples/`](../dbt_examples/) directory.

```bash
cd dbt_examples
```

### Option B: Create a new project

```bash
dbt init my_project
cd my_project
```

---

## Configure profiles.yml

dbt reads connection settings from `~/.dbt/profiles.yml` (global) or `./profiles.yml` (project-local).

Create or edit the file with the following content:

```yaml
my_project:
  target: dev
  outputs:
    dev:
      type: duckdb
      path: ~/.tdwh/db/dbt.db
      threads: 1
```

> **Note:** Replace `my_project` with the `profile:` value in your `dbt_project.yml`.

To use a project-local `profiles.yml`, pass `--profiles-dir .` when running dbt commands:

```bash
dbt run --profiles-dir .
```

---

## Writing models

Models are `.sql` files placed in the `models/` directory. dbt compiles and runs them against DuckDB.

**Example — `models/staging/stg_orders.sql`:**

```sql
{{ config(materialized='table') }}

select
    order_id,
    customer_id,
    order_date,
    status
from {{ ref('raw_orders') }}
```

**Materialization types:**

| Type | Description |
|------|-------------|
| `view` | Creates a view (fast, no data copy) |
| `table` | Creates a physical table (slower build, faster query) |
| `incremental` | Appends or merges new rows only |

---

## Writing table metadata

You can attach comments (metadata) to tables and columns using `post_hook` in the `config` block of a dbt model. Comments added this way are displayed in the Tiny Data Warehouse metadata panel.

**Example — `models/examples/stg_sales.sql`:**

```sql
{{
    config(
      materialized='table',
      schema='staging',
      alias='stg_sales',
      post_hook=[
        "COMMENT ON TABLE {{ this }} IS 'Sales summary table'",
        "COMMENT ON COLUMN {{ this }}.sale_id IS 'Unique identifier for each sale record'",
        "COMMENT ON COLUMN {{ this }}.user_id IS 'Unique identifier for the user'",
        "COMMENT ON COLUMN {{ this }}.product_name IS 'Name of the product'",
        "COMMENT ON COLUMN {{ this }}.revenue IS 'Sale amount (JPY)'",
        "COMMENT ON COLUMN {{ this }}.sale_date IS 'Date the sale occurred'"
      ]
    )
}}

SELECT
    1 AS sale_id, 101 AS user_id, 'Laptop'   AS product_name, 120000 AS revenue, DATE '2025-01-10' AS sale_date

UNION ALL SELECT 2, 102, 'Mouse',      3500, DATE '2025-01-15'
UNION ALL SELECT 3, 101, 'Keyboard',   8000, DATE '2025-02-01'
UNION ALL SELECT 4, 103, 'Monitor',   45000, DATE '2025-02-20'
UNION ALL SELECT 5, 102, 'USB Hub',    2800, DATE '2025-03-05'
```

**Key points:**

- `post_hook` accepts a list of SQL statements that run in order after the model finishes building.
- `{{ this }}` is a dbt variable that expands to the fully-qualified table name (`schema.table`).
- `COMMENT ON TABLE` sets a description for the whole table; `COMMENT ON COLUMN` sets a per-column description.
- After `dbt run`, click the ℹ️ button next to a table name in the Tiny Data Warehouse Explorer to view the metadata panel with these comments.

---

## Common dbt commands

| Command | Description |
|---------|-------------|
| `dbt run` | Build all models and write results to `dbt.db` |
| `dbt run --select my_model` | Build a specific model |
| `dbt run --select staging.*` | Build all models in a directory |
| `dbt test` | Run data quality tests |
| `dbt seed` | Load CSV files in `seeds/` as tables |
| `dbt compile` | Compile SQL without executing |
| `dbt docs generate` | Generate documentation site |
| `dbt clean` | Remove compiled artifacts (`target/`, `dbt_packages/`) |

### Run and immediately check in the app

```bash
dbt run --profiles-dir .
# The Explorer panel in Tiny Data Warehouse refreshes automatically
```

### Run a single model

```bash
dbt run --select stg_orders --profiles-dir .
```

### Run models and their dependencies

```bash
# Run stg_orders and all models that depend on it
dbt run --select stg_orders+ --profiles-dir .

# Run stg_orders and all models it depends on
dbt run --select +stg_orders --profiles-dir .
```

---

## Viewing results in Tiny Data Warehouse

After `dbt run` completes:

1. The Explorer panel refreshes automatically.
2. Tables appear under the schema names defined in `dbt_project.yml` (e.g., `staging`, `marts`).
3. Click any table to browse its contents or run queries against it.

---

## Troubleshooting

**Tables don't appear after `dbt run`**

- Confirm `path` in `profiles.yml` points to `~/.tdwh/db/dbt.db`.
- Check for errors in the dbt output (`dbt run` exit code must be 0).
- Restart the app and check the Explorer panel.

**`dbt: command not found`**

- Make sure `dbt-duckdb` is installed: `pip install dbt-duckdb`
- If using a virtual environment, activate it before running dbt.

**`profile 'my_project' not found`**

- The `profile:` field in `dbt_project.yml` must match a top-level key in `profiles.yml`.
- Run with `--profiles-dir .` if `profiles.yml` is in the project directory.

---

## Further reading

- [dbt documentation](https://docs.getdbt.com/)
- [dbt-duckdb adapter](https://github.com/duckdb/dbt-duckdb)
- [DuckDB SQL reference](https://duckdb.org/docs/sql/introduction)
