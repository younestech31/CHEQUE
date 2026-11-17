from pathlib import Path
import sqlite3

root=Path(__file__).resolve().parents[1]
db=sqlite3.connect(':memory:')
db.execute('PRAGMA foreign_keys=ON')
for name in ('0001_initial.sql','0002_cheques_and_statement_workflow.sql','0003_classification.sql'):
 db.executescript((root/'src-tauri/migrations'/name).read_text(encoding='utf-8'))

def blocked(fn,label):
 try: fn()
 except sqlite3.IntegrityError:
  print('PASS',label); return
 raise AssertionError('Expected blocked operation: '+label)

db.execute("INSERT INTO clients(full_name) VALUES('Client')")
db.execute("INSERT INTO bank_statement_entries(statement_reference,statement_date,entry_date,credit_minor) VALUES('ST-1','2026-11-01','2026-11-01',100000)")
db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(1,'2026-11-01',100000)")
db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,allocation_date,amount_minor,operation_type) VALUES(1,1,1,'2026-11-02',60000,'Deposit')")
blocked(lambda:db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,allocation_date,amount_minor,operation_type) VALUES(1,1,1,'2026-11-02',50000,'Deposit')"),'allocation ceiling')
db.execute("INSERT INTO freezes(allocation_id,amount_minor,reason,frozen_at) VALUES(1,20000,'Pending document','2026-11-03')")
blocked(lambda:db.execute("INSERT INTO freezes(allocation_id,amount_minor,reason,frozen_at) VALUES(1,50000,'Excess freeze','2026-11-03')"),'freeze ceiling')
db.execute("INSERT INTO outgoing_payments(payment_reference,payment_type,promoter_id,preparation_date) VALUES('PAY-001','periodic',1,'2026-11-04')")
db.execute("INSERT INTO payment_details(payment_id,allocation_id,amount_minor) VALUES(1,1,10000)")
db.execute("UPDATE outgoing_payments SET status='executed',execution_date='2026-11-05' WHERE id=1")
assert db.execute("SELECT amount_minor FROM treasury_ledger WHERE payment_id=1 AND direction='out'").fetchone()[0]==10000
assert db.execute("SELECT SUM(amount_minor) FROM treasury_ledger WHERE direction='in'").fetchone()[0]==100000
print('All core schema, freeze and execution-ledger tests passed.')
