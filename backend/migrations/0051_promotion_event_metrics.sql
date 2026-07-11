-- Privacy-minimal promotion delivery analytics.
--
-- A short-lived signed presentation token is deduplicated once per event type. Tokens contain no
-- account, email, IP, or device identifier. Daily aggregates remain useful after receipt rows are
-- removed by the retention worker.

CREATE TABLE platform.promotion_event_receipts (
  token_id UUID NOT NULL,
  promotion_id BIGINT NOT NULL REFERENCES platform.promotions(id) ON DELETE CASCADE,
  issued_version BIGINT NOT NULL,
  event_type TEXT NOT NULL,
  recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (token_id, event_type),
  CHECK (issued_version >= 1),
  CHECK (event_type IN ('impression', 'click'))
);

CREATE INDEX promotion_event_receipts_retention_idx
  ON platform.promotion_event_receipts (recorded_at);

CREATE TABLE platform.promotion_daily_metrics (
  promotion_id BIGINT NOT NULL REFERENCES platform.promotions(id) ON DELETE CASCADE,
  metric_date DATE NOT NULL,
  impressions BIGINT NOT NULL DEFAULT 0,
  clicks BIGINT NOT NULL DEFAULT 0,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (promotion_id, metric_date),
  CHECK (impressions >= 0),
  CHECK (clicks >= 0),
  CHECK (clicks <= impressions)
);

CREATE INDEX promotion_daily_metrics_date_idx
  ON platform.promotion_daily_metrics (metric_date DESC, promotion_id);
