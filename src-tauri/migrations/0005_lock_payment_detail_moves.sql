-- Version 5: prevent payment-detail updates from moving rows into a locked payment.
DROP TRIGGER IF EXISTS trg_payment_detail_update_only_before_reservation;
CREATE TRIGGER trg_payment_detail_update_only_before_reservation
BEFORE UPDATE ON payment_details
WHEN COALESCE((SELECT status FROM outgoing_payments WHERE id=OLD.payment_id),'') <> 'prepared'
 OR COALESCE((SELECT status FROM outgoing_payments WHERE id=NEW.payment_id),'') <> 'prepared'
BEGIN
 SELECT RAISE(ABORT,'payment details can only be changed while both payments are prepared');
END;
