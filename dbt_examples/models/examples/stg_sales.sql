-- 売上サマリー（サンプルデータ）
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
