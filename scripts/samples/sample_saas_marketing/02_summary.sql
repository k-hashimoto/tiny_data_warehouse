-- SaaS マーケティング 集計クエリ
-- 先に 01_generate.sql を実行してください

-- 日別・チャネル別: リード数・契約数・広告費・CPL・CPA
SELECT
  l.lead_date,
  ac.channel_name,
  COUNT(DISTINCT l.lead_id)                                                      AS leads,
  COUNT(DISTINCT c.contract_id)                                                  AS contracts,
  SUM(s.spend_amount)                                                            AS spend,
  ROUND(SUM(s.spend_amount)
    / NULLIF(COUNT(DISTINCT l.lead_id), 0))                                      AS cpl,
  ROUND(SUM(s.spend_amount)
    / NULLIF(COUNT(DISTINCT c.contract_id), 0))                                  AS cpa,
  ROUND(100.0 * COUNT(DISTINCT c.contract_id)
    / NULLIF(COUNT(DISTINCT l.lead_id), 0), 1)                                   AS cvr_pct
FROM sample_saas_marketing.leads l
JOIN sample_saas_marketing.ad_channels ac ON l.channel_id  = ac.channel_id
LEFT JOIN sample_saas_marketing.contracts  c  ON l.lead_id = c.lead_id
LEFT JOIN sample_saas_marketing.ad_spend   s
  ON s.spend_date = l.lead_date AND s.channel_id = l.channel_id
GROUP BY l.lead_date, ac.channel_name
ORDER BY l.lead_date DESC, ac.channel_name;
