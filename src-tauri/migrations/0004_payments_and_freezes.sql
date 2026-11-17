-- Version 4: payment reservations, execution controls and freeze protection.
-- Optional payment metadata columns are added conditionally by the Rust runner.
CREATE UNIQUE INDEX IF NOT EXISTS idx_v4_ledger_collection_unique
 ON treasury_ledger(collection_id) WHERE collection_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_v4_ledger_payment_unique
 ON treasury_ledger(payment_id) WHERE payment_id IS NOT NULL;

CREATE TRIGGER IF NOT EXISTS trg_payment_detail_insert_only_before_reservation
BEFORE INSERT ON payment_details
WHEN COALESCE((SELECT status FROM outgoing_payments WHERE id=NEW.payment_id),'') <> 'prepared'
BEGIN SELECT RAISE(ABORT,'payment details are locked after reservation'); END;

CREATE TRIGGER IF NOT EXISTS trg_payment_detail_update_only_before_reservation
BEFORE UPDATE ON payment_details
WHEN COALESCE((SELECT status FROM outgoing_payments WHERE id=OLD.payment_id),'') <> 'prepared'
BEGIN SELECT RAISE(ABORT,'payment details are locked after reservation'); END;

CREATE TRIGGER IF NOT EXISTS trg_payment_detail_delete_only_before_reservation
BEFORE DELETE ON payment_details
WHEN COALESCE((SELECT status FROM outgoing_payments WHERE id=OLD.payment_id),'') <> 'prepared'
BEGIN SELECT RAISE(ABORT,'payment details are locked after reservation'); END;

CREATE TRIGGER IF NOT EXISTS trg_payment_promoter_matches_allocation
BEFORE INSERT ON payment_details
BEGIN
 SELECT CASE WHEN COALESCE((SELECT promoter_id FROM outgoing_payments WHERE id=NEW.payment_id),-1)
  <> COALESCE((SELECT promoter_id FROM allocations WHERE id=NEW.allocation_id),-2)
 THEN RAISE(ABORT,'payment promoter does not match allocation promoter') END;
END;

CREATE TRIGGER IF NOT EXISTS trg_payment_detail_update_promoter
BEFORE UPDATE ON payment_details
BEGIN
 SELECT CASE WHEN COALESCE((SELECT promoter_id FROM outgoing_payments WHERE id=NEW.payment_id),-1)
  <> COALESCE((SELECT promoter_id FROM allocations WHERE id=NEW.allocation_id),-2)
 THEN RAISE(ABORT,'payment promoter does not match allocation promoter') END;
END;

CREATE TRIGGER IF NOT EXISTS trg_payment_reservation_limits
BEFORE UPDATE OF status ON outgoing_payments
WHEN NEW.status='reserved' AND OLD.status='prepared'
BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM payment_details WHERE payment_id=NEW.id)
 THEN RAISE(ABORT,'cannot reserve an empty payment') END;
 SELECT CASE WHEN EXISTS(
  SELECT 1 FROM payment_details pd JOIN allocations a ON a.id=pd.allocation_id
  WHERE pd.payment_id=NEW.id AND
   (SELECT COALESCE(SUM(x.amount_minor),0) FROM payment_details x JOIN outgoing_payments p ON p.id=x.payment_id
    WHERE x.allocation_id=pd.allocation_id AND p.id<>NEW.id AND p.status IN('reserved','executed'))
   +(SELECT COALESCE(SUM(f.amount_minor),0) FROM freezes f WHERE f.allocation_id=pd.allocation_id AND f.released_at IS NULL)
   +(SELECT COALESCE(SUM(y.amount_minor),0) FROM payment_details y WHERE y.allocation_id=pd.allocation_id AND y.payment_id=NEW.id)
   >a.amount_minor
 ) THEN RAISE(ABORT,'payment reservation exceeds available allocation') END;
END;

CREATE TRIGGER IF NOT EXISTS trg_reservation_within_treasury_cash
BEFORE UPDATE OF status ON outgoing_payments
WHEN NEW.status='reserved' AND OLD.status='prepared'
BEGIN
 SELECT CASE WHEN
  (SELECT COALESCE(SUM(CASE direction WHEN 'in' THEN amount_minor ELSE -amount_minor END),0) FROM treasury_ledger)
  < (SELECT COALESCE(SUM(pd.amount_minor),0) FROM payment_details pd JOIN outgoing_payments p ON p.id=pd.payment_id
     WHERE p.status='reserved' AND p.id<>NEW.id)
    +(SELECT COALESCE(SUM(amount_minor),0) FROM payment_details WHERE payment_id=NEW.id)
 THEN RAISE(ABORT,'reserved payments exceed available treasury cash') END;
END;

CREATE TRIGGER IF NOT EXISTS trg_freeze_respects_payments
BEFORE INSERT ON freezes
WHEN NEW.released_at IS NULL
BEGIN
 SELECT CASE WHEN
  (SELECT COALESCE(SUM(f.amount_minor),0) FROM freezes f WHERE f.allocation_id=NEW.allocation_id AND f.released_at IS NULL)
  +NEW.amount_minor
  +(SELECT COALESCE(SUM(pd.amount_minor),0) FROM payment_details pd JOIN outgoing_payments p ON p.id=pd.payment_id
    WHERE pd.allocation_id=NEW.allocation_id AND p.status IN('reserved','executed'))
  >(SELECT amount_minor FROM allocations WHERE id=NEW.allocation_id)
 THEN RAISE(ABORT,'freeze conflicts with reserved or executed payments') END;
END;

CREATE TRIGGER IF NOT EXISTS trg_payment_state_transitions
BEFORE UPDATE OF status ON outgoing_payments
WHEN NOT (
 (OLD.status='prepared' AND NEW.status IN('reserved','cancelled')) OR
 (OLD.status='reserved' AND NEW.status IN('executed','cancelled')) OR OLD.status=NEW.status
)
BEGIN SELECT RAISE(ABORT,'invalid payment state transition'); END;

CREATE TRIGGER IF NOT EXISTS trg_payment_execution_requires_date
BEFORE UPDATE OF status ON outgoing_payments
WHEN NEW.status='executed' AND (NEW.execution_date IS NULL OR length(NEW.execution_date)<>10)
BEGIN SELECT RAISE(ABORT,'execution date is required'); END;

CREATE TRIGGER IF NOT EXISTS trg_freeze_terms_immutable
BEFORE UPDATE OF allocation_id,amount_minor,reason,frozen_at ON freezes
BEGIN SELECT RAISE(ABORT,'freeze terms are immutable; release and create a new freeze'); END;

CREATE TRIGGER IF NOT EXISTS trg_no_delete_freezes BEFORE DELETE ON freezes
BEGIN SELECT RAISE(ABORT,'freeze history cannot be deleted'); END;
CREATE TRIGGER IF NOT EXISTS trg_no_delete_payments BEFORE DELETE ON outgoing_payments
BEGIN SELECT RAISE(ABORT,'payment history cannot be deleted'); END;
CREATE TRIGGER IF NOT EXISTS trg_no_delete_payment_details BEFORE DELETE ON payment_details
BEGIN SELECT RAISE(ABORT,'payment details cannot be deleted'); END;
CREATE TRIGGER IF NOT EXISTS trg_no_delete_ledger BEFORE DELETE ON treasury_ledger
BEGIN SELECT RAISE(ABORT,'treasury ledger entries cannot be deleted'); END;
CREATE TRIGGER IF NOT EXISTS trg_no_update_ledger BEFORE UPDATE ON treasury_ledger
BEGIN SELECT RAISE(ABORT,'treasury ledger entries cannot be modified'); END;
CREATE TRIGGER IF NOT EXISTS trg_no_update_collection BEFORE UPDATE ON collections
BEGIN SELECT RAISE(ABORT,'collection records are immutable; use a correction entry'); END;
CREATE TRIGGER IF NOT EXISTS trg_no_delete_collection BEFORE DELETE ON collections
BEGIN SELECT RAISE(ABORT,'collection records cannot be deleted'); END;
CREATE TRIGGER IF NOT EXISTS trg_no_update_bank_entry BEFORE UPDATE ON bank_statement_entries
BEGIN SELECT RAISE(ABORT,'bank statement source lines are immutable'); END;
CREATE TRIGGER IF NOT EXISTS trg_no_delete_bank_entry BEFORE DELETE ON bank_statement_entries
BEGIN SELECT RAISE(ABORT,'bank statement source lines cannot be deleted'); END;
CREATE TRIGGER IF NOT EXISTS trg_payment_header_immutable_after_reservation
BEFORE UPDATE OF payment_reference,payment_type,promoter_id,preparation_date ON outgoing_payments
WHEN OLD.status<>'prepared'
BEGIN SELECT RAISE(ABORT,'reserved payment header is immutable'); END;
