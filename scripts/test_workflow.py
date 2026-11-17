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
blocked(lambda:db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(3,'2026-11-05',10000)"),'debit entry cannot be collected')

db.execute("INSERT INTO clients(full_name) VALUES('Client One'),('Client Two')")
db.execute("INSERT INTO projects(promoter_id,name) VALUES(1,'Project A')")
db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,project_id,allocation_date,amount_minor,operation_type) VALUES(1,1,1,1,'2026-11-06',40000,'إيداع')")
db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,allocation_date,amount_minor,operation_type) VALUES(1,2,2,'2026-11-06',30000,'إيداع')")
blocked(lambda:db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,allocation_date,amount_minor,operation_type) VALUES(1,1,1,'2026-11-06',1,'إيداع')"),'allocation exceeds confirmed collection')
blocked(lambda:db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,project_id,allocation_date,amount_minor,operation_type) VALUES(1,2,1,1,'2026-11-06',1,'إيداع')"),'project belongs to another promoter')
blocked(lambda:db.execute("UPDATE allocations SET amount_minor=1 WHERE id=1"),'allocation is immutable')
blocked(lambda:db.execute("DELETE FROM allocations WHERE id=1"),'allocation cannot be deleted')

db.execute("INSERT INTO freezes(allocation_id,amount_minor,reason,frozen_at) VALUES(1,10000,'سبب اختبار','2026-11-07')")
db.execute("INSERT INTO outgoing_payments(payment_reference,payment_type,promoter_id,preparation_date) VALUES('PAY-01','periodic',1,'2026-11-08')")
db.execute("INSERT INTO payment_details(payment_id,allocation_id,amount_minor) VALUES(1,1,30000)")
db.execute("UPDATE outgoing_payments SET status='reserved' WHERE id=1")
blocked(lambda:db.execute("INSERT INTO payment_details(payment_id,allocation_id,amount_minor) VALUES(1,1,1)"),'reserved payment details are immutable')
blocked(lambda:db.execute("INSERT INTO freezes(allocation_id,amount_minor,reason,frozen_at) VALUES(1,20001,'تجميد زائد','2026-11-09')"),'freeze cannot overlap reserved amount')
db.execute("UPDATE outgoing_payments SET status='executed',execution_date='2026-11-10' WHERE id=1")
assert db.execute("SELECT SUM(amount_minor) FROM treasury_ledger WHERE payment_id=1 AND direction='out'").fetchone()[0]==30000
blocked(lambda:db.execute("UPDATE outgoing_payments SET status='cancelled' WHERE id=1"),'executed payment cannot be cancelled')
blocked(lambda:db.execute("DELETE FROM treasury_ledger WHERE payment_id=1"),'ledger entries cannot be deleted')
print('All cheque, treasury, classification, freeze and payment tests passed.')
