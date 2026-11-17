from pathlib import Path
import sqlite3

root=Path(__file__).resolve().parents[1]
db=sqlite3.connect(':memory:')
db.execute('PRAGMA foreign_keys=ON')
for name in ('0001_initial.sql','0002_cheques_and_statement_workflow.sql','0002_cheques_and_statement_workflow.sql'):
 db.executescript((root/'src-tauri/migrations'/name).read_text(encoding='utf-8'))

def blocked(fn,label):
 try: fn()
 except sqlite3.IntegrityError:
  print('PASS',label); return
 raise AssertionError('Expected blocked operation: '+label)

db.execute("INSERT INTO cheques(cheque_number,bank_name,received_date,amount_minor) VALUES('C-01','BNA','2026-11-01',100000)")
blocked(lambda:db.execute("INSERT INTO cheques(cheque_number,bank_name,received_date,amount_minor) VALUES('C-01','BNA','2026-11-01',100000)"),'duplicate cheque')
db.execute("INSERT INTO cheque_deposits(cheque_id,deposit_date,amount_minor) VALUES(1,'2026-11-02',60000)")
blocked(lambda:db.execute("INSERT INTO cheque_deposits(cheque_id,deposit_date,amount_minor) VALUES(1,'2026-11-03',50000)"),'deposit cannot exceed cheque face value')
assert db.execute('SELECT COUNT(*) FROM treasury_ledger').fetchone()[0]==0

db.execute("INSERT INTO bank_statement_entries(statement_reference,statement_date,entry_date,bank_transaction_reference,credit_minor) VALUES('ST-11','2026-11-30','2026-11-03','TX-1',120000)")
blocked(lambda:db.execute("INSERT INTO bank_statement_entries(statement_reference,statement_date,entry_date,bank_transaction_reference,credit_minor) VALUES('ST-11','2026-11-30','2026-11-03','TX-1',120000)"),'duplicate bank statement line')
db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(1,'2026-11-03',70000)")
db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(1,'2026-11-04',50000)")
blocked(lambda:db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(1,'2026-11-04',1)"),'collections cannot exceed bank credit')
blocked(lambda:db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(99,'2026-11-04',1)"),'collection requires valid credit entry')
assert db.execute("SELECT SUM(amount_minor) FROM treasury_ledger WHERE direction='in'").fetchone()[0]==120000

db.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(1,1,40000)")
blocked(lambda:db.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(2,1,30000)"),'matched amount cannot exceed deposited amount')
blocked(lambda:db.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(1,1,30000)"),'matched amount cannot exceed cheque face value')
db.execute("INSERT INTO bank_statement_entries(statement_reference,statement_date,entry_date,debit_minor) VALUES('ST-11','2026-11-30','2026-11-05',10000)")
blocked(lambda:db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(2,'2026-11-05',10000)"),'debit entry cannot be confirmed as collection')
print('All cheque and treasury workflow tests passed.')
