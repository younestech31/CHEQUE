from pathlib import Path
import sqlite3
root=Path(__file__).resolve().parents[1]
db=sqlite3.connect(':memory:')
db.execute('PRAGMA foreign_keys=ON')
db.executescript((root/'src-tauri/migrations/0001_initial.sql').read_text(encoding='utf-8'))
db.execute("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT(datetime('now')))")
for table,column,definition in [('cheques','issue_date','TEXT'),('cheques','notes','TEXT'),('bank_statement_entries','description','TEXT'),('collections','confirmation_reference','TEXT'),('collections','notes','TEXT'),('audit_log','after_json','TEXT')]:
 columns={r[1] for r in db.execute('PRAGMA table_info(' + table + ')')}
 if column not in columns: db.execute('ALTER TABLE ' + table + ' ADD COLUMN ' + column + ' ' + definition)
db.executescript((root/'src-tauri/migrations/0002_cheques_and_statement_workflow.sql').read_text(encoding='utf-8'))
db.execute("INSERT INTO schema_migrations(version) VALUES(2)")
if 'operation_type' not in {r[1] for r in db.execute('PRAGMA table_info(allocations)')}:
 db.execute("ALTER TABLE allocations ADD COLUMN operation_type TEXT NOT NULL DEFAULT 'غير محدد'")
db.executescript((root/'src-tauri/migrations/0003_classification.sql').read_text(encoding='utf-8'))
db.execute("INSERT INTO schema_migrations(version) VALUES(3)")
for column in ('external_reference','notes'):
 if column not in {r[1] for r in db.execute('PRAGMA table_info(outgoing_payments)')}:
  db.execute('ALTER TABLE outgoing_payments ADD COLUMN ' + column + ' TEXT')
db.executescript((root/'src-tauri/migrations/0004_payments_and_freezes.sql').read_text(encoding='utf-8'))
db.execute("INSERT INTO schema_migrations(version) VALUES(4)")
db.executescript((root/'src-tauri/migrations/0005_lock_payment_detail_moves.sql').read_text(encoding='utf-8'))
db.execute("INSERT INTO schema_migrations(version) VALUES(5)")
db.executescript((root/'src-tauri/migrations/0006_treasury_reconciliation.sql').read_text(encoding='utf-8'))
db.execute("INSERT INTO schema_migrations(version) VALUES(6)")
def blocked(fn,label):
 try: fn()
 except sqlite3.IntegrityError: print('PASS',label); return
 raise AssertionError('Expected blocked operation: '+label)
db.execute("INSERT INTO cheques(cheque_number,bank_name,received_date,amount_minor) VALUES('C-01','BNA','2026-11-01',100000)")
blocked(lambda:db.execute("INSERT INTO cheques(cheque_number,bank_name,received_date,amount_minor) VALUES('C-01','BNA','2026-11-01',100000)"),'duplicate cheque')
db.execute("INSERT INTO cheques(cheque_number,bank_name,received_date,amount_minor) VALUES('C-02','CPA','2026-11-01',80000)")
db.execute("INSERT INTO cheque_deposits(cheque_id,deposit_date,amount_minor) VALUES(1,'2026-11-02',60000)")
db.execute("INSERT INTO cheque_deposits(cheque_id,deposit_date,amount_minor) VALUES(2,'2026-11-02',80000)")
blocked(lambda:db.execute("INSERT INTO cheque_deposits(cheque_id,deposit_date,amount_minor) VALUES(1,'2026-11-03',50000)"),'deposit exceeds cheque face value')
assert db.execute('SELECT COUNT(*) FROM treasury_ledger').fetchone()[0]==0
db.execute("INSERT INTO bank_statement_entries(statement_reference,statement_date,entry_date,bank_transaction_reference,credit_minor,description) VALUES('ST-11','2026-11-30','2026-11-03','TX-1',120000,'Cheque credit')")
blocked(lambda:db.execute("INSERT INTO bank_statement_entries(statement_reference,statement_date,entry_date,bank_transaction_reference,credit_minor,description) VALUES('ST-11','2026-11-30','2026-11-03','TX-1',120000,'Cheque credit')"),'duplicate bank statement line')
db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(1,'2026-11-03',70000)")
db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(1,'2026-11-04',50000)")
blocked(lambda:db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(1,'2026-11-04',1)"),'collections exceed bank credit')
blocked(lambda:db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(99,'2026-11-04',1)"),'unknown credit entry')
db.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(1,1,40000)")
db.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(1,2,30000)")
db.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(2,1,20000)")
db.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(2,2,20000)")
blocked(lambda:db.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(2,1,1)"),'cheque matching exceeds deposit')
blocked(lambda:db.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(2,2,31000)"),'cheque matching exceeds collection')
db.execute("INSERT INTO bank_statement_entries(statement_reference,statement_date,entry_date,debit_minor) VALUES('ST-11','2026-11-30','2026-11-05',10000)")
blocked(lambda:db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(2,'2026-11-05',10000)"),'debit entry cannot be collected')
db.execute("INSERT INTO clients(full_name) VALUES('Client One'),('Client Two')")
db.execute("INSERT INTO projects(promoter_id,name) VALUES(1,'Project A')")
db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,project_id,allocation_date,amount_minor,operation_type) VALUES(1,1,1,1,'2026-11-06',40000,'إيداع')")
db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,allocation_date,amount_minor,operation_type) VALUES(1,2,2,'2026-11-06',30000,'إيداع')")
blocked(lambda:db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,allocation_date,amount_minor,operation_type) VALUES(1,1,1,'2026-11-06',1,'إيداع')"),'allocation exceeds collection')
blocked(lambda:db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,project_id,allocation_date,amount_minor,operation_type) VALUES(2,2,1,1,'2026-11-06',1,'إيداع')"),'project promoter mismatch')
blocked(lambda:db.execute("UPDATE allocations SET amount_minor=1 WHERE id=1"),'allocation immutable')
blocked(lambda:db.execute("DELETE FROM allocations WHERE id=1"),'allocation deletion blocked')
db.execute("INSERT INTO freezes(allocation_id,amount_minor,reason,frozen_at) VALUES(1,10000,'سبب اختبار','2026-11-07')")
db.execute("INSERT INTO outgoing_payments(payment_reference,payment_type,promoter_id,preparation_date) VALUES('PAY-01','periodic',1,'2026-11-08')")
db.execute("INSERT INTO payment_details(payment_id,allocation_id,amount_minor) VALUES(1,1,30000)")
db.execute("UPDATE outgoing_payments SET status='reserved' WHERE id=1")
blocked(lambda:db.execute("INSERT INTO payment_details(payment_id,allocation_id,amount_minor) VALUES(1,1,1)"),'reserved payment details locked')
db.execute("INSERT INTO outgoing_payments(payment_reference,payment_type,promoter_id,preparation_date) VALUES('PAY-MOVE','periodic',1,'2026-11-08')")
db.execute("INSERT INTO payment_details(payment_id,allocation_id,amount_minor) VALUES(2,1,5000)")
blocked(lambda:db.execute("UPDATE payment_details SET payment_id=1 WHERE payment_id=2"),'cannot move details into a reserved payment')
db.execute("UPDATE payment_details SET amount_minor=6000 WHERE payment_id=2")
assert db.execute("SELECT amount_minor FROM payment_details WHERE payment_id=2").fetchone()[0] == 6000
blocked(lambda:db.execute("INSERT INTO freezes(allocation_id,amount_minor,reason,frozen_at) VALUES(1,20001,'تجميد زائد','2026-11-09')"),'freeze cannot overlap reserved amount')
db.execute("UPDATE outgoing_payments SET status='executed',execution_date='2026-11-10' WHERE id=1")
assert db.execute("SELECT SUM(amount_minor) FROM treasury_ledger WHERE payment_id=1 AND direction='out'").fetchone()[0]==30000
blocked(lambda:db.execute("UPDATE outgoing_payments SET status='cancelled' WHERE id=1"),'executed payment cannot be cancelled')
blocked(lambda:db.execute("DELETE FROM treasury_ledger WHERE payment_id=1"),'ledger immutable')
db.execute("INSERT INTO treasury_ledger(movement_type,movement_date,amount_minor,direction) VALUES('correction_out','2026-11-11',80000,'out')")
db.execute("INSERT INTO outgoing_payments(payment_reference,payment_type,promoter_id,preparation_date) VALUES('PAY-02','exceptional',2,'2026-11-12')")
db.execute("INSERT INTO payment_details(payment_id,allocation_id,amount_minor) VALUES(3,2,30000)")
blocked(lambda:db.execute("UPDATE outgoing_payments SET status='reserved' WHERE id=3"),'reservation cannot exceed actual treasury cash')
# Backup verification prerequisites: required application schema and migrations must exist.
required_tables = db.execute("""
 SELECT COUNT(*) FROM sqlite_master
 WHERE type='table' AND name IN (
  'schema_migrations','cheques','bank_statement_entries','collections',
  'allocations','outgoing_payments','payment_details','treasury_ledger',
  'treasury_opening_balance','bank_debit_reconciliations'
 )
""").fetchone()[0]
assert required_tables == 10, required_tables
assert db.execute(
 "SELECT COUNT(*) FROM schema_migrations WHERE version IN (2,3,4,5,6)"
).fetchone()[0] == 5
assert db.execute("PRAGMA integrity_check").fetchone()[0] == 'ok'
assert db.execute("PRAGMA foreign_key_check").fetchall() == []
db.execute("INSERT INTO treasury_opening_balance(id,amount_minor,balance_date,statement_reference) VALUES(1,50000,'2026-11-12','OPEN-1')")
cash=db.execute("SELECT COALESCE(SUM(CASE direction WHEN 'in' THEN amount_minor ELSE -amount_minor END),0) FROM treasury_ledger WHERE movement_date >= (SELECT balance_date FROM treasury_opening_balance WHERE id=1)").fetchone()[0]
assert cash==50000,cash
db.execute("UPDATE outgoing_payments SET status='reserved' WHERE id=3")
assert db.execute("SELECT status FROM outgoing_payments WHERE id=3").fetchone()[0]=='reserved'

print('All cheque, treasury, classification, freeze, payment and backup-schema tests passed.')

# Match an executed payment without posting a duplicate cash movement.
db.execute("INSERT INTO bank_statement_entries(statement_reference,statement_date,entry_date,debit_minor) VALUES('ST-11','2026-11-30','2026-11-13',30000)")
ledger_before=db.execute("SELECT SUM(CASE direction WHEN 'in' THEN amount_minor ELSE -amount_minor END) FROM treasury_ledger").fetchone()[0]
db.execute("INSERT INTO bank_debit_reconciliations(bank_entry_id,amount_minor,reconciliation_type,payment_id) VALUES(3,30000,'payment',1)")
ledger_after=db.execute("SELECT SUM(CASE direction WHEN 'in' THEN amount_minor ELSE -amount_minor END) FROM treasury_ledger").fetchone()[0]
assert ledger_after==ledger_before,(ledger_before,ledger_after)
blocked(lambda:db.execute("INSERT INTO bank_debit_reconciliations(bank_entry_id,amount_minor,reconciliation_type,payment_id) VALUES(3,1,'payment',1)"),'bank debit reconciliation ceiling')
db.execute("INSERT INTO bank_debit_reconciliations(bank_entry_id,amount_minor,reconciliation_type) VALUES(2,10000,'external')")
assert db.execute("SELECT amount_minor,direction FROM treasury_ledger WHERE statement_entry_id=2").fetchone()==(10000,'out')
blocked(lambda:db.execute("INSERT INTO bank_debit_reconciliations(bank_entry_id,amount_minor,reconciliation_type) VALUES(2,10000,'external')"),'external debit cannot be posted twice')
blocked(lambda:db.execute("UPDATE bank_debit_reconciliations SET amount_minor=1 WHERE bank_entry_id=2"),'debit reconciliation immutable')
assert db.execute("PRAGMA integrity_check").fetchone()[0]=='ok'
assert db.execute("PRAGMA foreign_key_check").fetchall()==[]
