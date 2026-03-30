-- SaaS マーケティング ダミーデータ生成
-- ファネル: デジタル広告 → リード → 契約
--
-- 生成されるテーブル:
--   ad_channels  広告チャネルマスタ（4媒体）
--   products     プロダクトマスタ（3製品）
--   ad_spend     日別・チャネル別広告費（過去90日）
--   leads        リードテーブル（日別90〜110件、過去90日）
--   contracts    契約テーブル（日別8〜13件）
--
-- 集計例は 02_summary.sql を参照

DROP TABLE IF EXISTS contracts;
DROP TABLE IF EXISTS leads;
DROP TABLE IF EXISTS ad_spend;
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS ad_channels;

-- 広告チャネルマスタ
--   lead_share   : リード獲得比率
--   contract_rate: リード→契約の転換率
CREATE TABLE ad_channels AS
SELECT * FROM (VALUES
  ('google_ads',   'Google Ads',   0.40, 0.12),
  ('facebook_ads', 'Facebook Ads', 0.30, 0.08),
  ('linkedin_ads', 'LinkedIn Ads', 0.20, 0.18),
  ('display_ads',  'Display Ads',  0.10, 0.05)
) t(channel_id, channel_name, lead_share, contract_rate);

-- プロダクトマスタ
--   mrr       : 月次収益（円）
--   lead_share: リード中の比率
CREATE TABLE products AS
SELECT * FROM (VALUES
  ('prod_a', 'Enterprise',   50000, 0.20),
  ('prod_b', 'Professional', 20000, 0.50),
  ('prod_c', 'Starter',       5000, 0.30)
) t(product_id, product_name, mrr, lead_share);

-- 日別・チャネル別広告費（決定論的な疑似乱数で変動を表現）
CREATE TABLE ad_spend AS
SELECT
  d::DATE AS spend_date,
  channel_id,
  channel_name,
  CASE channel_id
    WHEN 'google_ads'   THEN 45000 + (EXTRACT(DOY FROM d::DATE)::BIGINT * 317 + 1) % 20000
    WHEN 'facebook_ads' THEN 28000 + (EXTRACT(DOY FROM d::DATE)::BIGINT * 251 + 2) % 15000
    WHEN 'linkedin_ads' THEN 38000 + (EXTRACT(DOY FROM d::DATE)::BIGINT * 197 + 3) % 18000
    WHEN 'display_ads'  THEN  9000 + (EXTRACT(DOY FROM d::DATE)::BIGINT * 173 + 4) %  5000
  END AS spend_amount
FROM generate_series(
  (current_date - INTERVAL '89 days'),
  current_date,
  INTERVAL '1 day'
) gs(d)
CROSS JOIN ad_channels;

-- リードテーブル（日別90〜110件、チャネル・プロダクト分布は比率どおり）
CREATE TABLE leads AS
WITH date_series AS (
  SELECT
    d::DATE                                   AS lead_date,
    (ROW_NUMBER() OVER () - 1)::BIGINT        AS day_offset
  FROM generate_series(
    (current_date - INTERVAL '89 days'),
    current_date,
    INTERVAL '1 day'
  ) gs(d)
),
daily_counts AS (
  SELECT
    lead_date,
    day_offset,
    (95 + (day_offset * 7 + 3) % 16)::INT AS daily_count
  FROM date_series
),
-- 各日のリード行を展開
lead_rows AS (
  SELECT lead_date, n
  FROM daily_counts,
  generate_series(1, daily_count) gs(n)
),
numbered AS (
  SELECT
    ROW_NUMBER() OVER (ORDER BY lead_date, n) AS lead_id,
    lead_date,
    n
  FROM lead_rows
)
SELECT
  lead_id,
  'U' || LPAD(lead_id::TEXT, 7, '0') AS user_id,
  lead_date,
  -- チャネル割当: Google 40% / Facebook 30% / LinkedIn 20% / Display 10%
  CASE lead_id % 10
    WHEN 0 THEN 'google_ads'
    WHEN 1 THEN 'google_ads'
    WHEN 2 THEN 'google_ads'
    WHEN 3 THEN 'google_ads'
    WHEN 4 THEN 'facebook_ads'
    WHEN 5 THEN 'facebook_ads'
    WHEN 6 THEN 'facebook_ads'
    WHEN 7 THEN 'linkedin_ads'
    WHEN 8 THEN 'linkedin_ads'
    WHEN 9 THEN 'display_ads'
  END AS channel_id,
  -- プロダクト割当: Enterprise 20% / Professional 50% / Starter 30%
  CASE (lead_id * 7 + 1) % 10
    WHEN 0 THEN 'prod_a'
    WHEN 1 THEN 'prod_a'
    WHEN 2 THEN 'prod_b'
    WHEN 3 THEN 'prod_b'
    WHEN 4 THEN 'prod_b'
    WHEN 5 THEN 'prod_b'
    WHEN 6 THEN 'prod_b'
    WHEN 7 THEN 'prod_c'
    WHEN 8 THEN 'prod_c'
    WHEN 9 THEN 'prod_c'
  END AS product_id
FROM numbered;

-- 契約テーブル（チャネル別転換率で絞り込み、リード獲得から1〜14日後に契約）
CREATE TABLE contracts AS
SELECT
  ROW_NUMBER() OVER (ORDER BY l.lead_id) AS contract_id,
  l.lead_id,
  l.user_id,
  l.channel_id,
  l.product_id,
  -- 契約日: リード日 + 1〜14日のラグ
  l.lead_date + (1 + (l.lead_id * 11 + 5) % 14)::INT AS contract_date,
  p.mrr
FROM leads l
JOIN products     p  ON l.product_id = p.product_id
JOIN ad_channels  ac ON l.channel_id = ac.channel_id
WHERE
  -- チャネル別転換率を決定論的に適用
  (l.lead_id * 13 + 7) % 100 < (ac.contract_rate * 100)::INT
  -- 契約日が今日以前のものだけ（将来の予測は含めない）
  AND l.lead_date + (1 + (l.lead_id * 11 + 5) % 14)::INT <= current_date;

-- 生成確認
SELECT 'leads'     AS tbl, COUNT(*) AS cnt, MIN(lead_date)     AS min_date, MAX(lead_date)     AS max_date FROM leads
UNION ALL
SELECT 'contracts',        COUNT(*),        MIN(contract_date), MAX(contract_date)              FROM contracts;
