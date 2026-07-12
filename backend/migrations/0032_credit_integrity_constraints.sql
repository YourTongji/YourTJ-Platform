-- 0032_credit_integrity_constraints.sql
-- Harden closed-loop credit invariants without requiring existing deployments
-- to rewrite historical rows during a rolling release.

ALTER TABLE credit.products
  ADD CONSTRAINT credit_products_stock_nonnegative
  CHECK (stock >= 0) NOT VALID;

ALTER TABLE credit.tasks
  ADD CONSTRAINT credit_tasks_no_self_accept
  CHECK (acceptor_id IS NULL OR acceptor_id <> creator_id) NOT VALID;

ALTER TABLE credit.purchases
  ADD CONSTRAINT credit_purchases_distinct_parties
  CHECK (buyer_id <> seller_id) NOT VALID;

ALTER TABLE credit.ledger
  ADD CONSTRAINT credit_ledger_controlled_flow_type
  CHECK (type IN ('mint', 'tip', 'escrow_hold', 'escrow_release')) NOT VALID;

-- NOT VALID constraints protect every new insert/update immediately. Validate
-- clean installations and clean existing deployments, while allowing a rolling
-- deployment with historical anomalies to finish so operators can assess them
-- through controlled reconciliation instead of mutating financial history.
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM credit.products WHERE stock < 0) THEN
    ALTER TABLE credit.products VALIDATE CONSTRAINT credit_products_stock_nonnegative;
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM credit.tasks
    WHERE acceptor_id IS NOT NULL AND acceptor_id = creator_id
  ) THEN
    ALTER TABLE credit.tasks VALIDATE CONSTRAINT credit_tasks_no_self_accept;
  END IF;

  IF NOT EXISTS (SELECT 1 FROM credit.purchases WHERE buyer_id = seller_id) THEN
    ALTER TABLE credit.purchases VALIDATE CONSTRAINT credit_purchases_distinct_parties;
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM credit.ledger
    WHERE type NOT IN ('mint', 'tip', 'escrow_hold', 'escrow_release')
  ) THEN
    ALTER TABLE credit.ledger VALIDATE CONSTRAINT credit_ledger_controlled_flow_type;
  END IF;
END
$$;

CREATE OR REPLACE FUNCTION credit.reject_ledger_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
  RAISE EXCEPTION 'credit.ledger is append-only' USING ERRCODE = '55000';
END
$$;

CREATE TRIGGER credit_ledger_reject_mutation
BEFORE UPDATE OR DELETE ON credit.ledger
FOR EACH ROW
EXECUTE FUNCTION credit.reject_ledger_mutation();
