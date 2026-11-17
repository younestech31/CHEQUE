from pathlib import Path
import sqlite3

root=Path(__file__).resolve().parents[1]
db=sqlite3.connect(':memory:')
db.execute('PRAGMA foreign_keys=ON')
db.executescript((root/'src-tauri/migrations/0001_initial.sql').read_text(encoding='utf-8'))
db.execute("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT(datetime('now')))")
db.executescript((root/'src-tauri/migrations/0002_cheques_and_statement_workflow.sql').read_text(encoding='utf-8'))
db.execute("INSERT INTO schema_migrations(version) VALUES(2)")
db.executescript((root/'src-tauri/migrations/0003_classification.sql').read_text(encoding='utf-8'))
db.execute("INSERT INTO schema_migrations(version) VALUES(3)")
assert db.execute("SELECT COUNT(*) FROM schema_migrations WHERE version=2").fetchone()[0]==1
assert {r[1] for r in db.execute('PRAGMA table_info(cheques)')} >= {'issue_date','notes'}
assert {r[1] for r in db.execute('PRAGMA table_info(bank_statement_entries)')} >= {'description'}
assert {r[1] for r in db.execute('PRAGMA table_info(collections)')} >= {'confirmation_reference','notes'}
assert {r[1] for r in db.execute('PRAGMA table_info(audit_log)')} >= {'after_json'}
assert db.execute("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN('cheque_deposits','collection_cheques')").fetchone()[0]==2

def blocked(fn,label):
 try: fn()
 except sqlite3.IntegrityError:
  print('PASS',label); return
 raise AssertionError('Expected blocked operation: '+label)

db.execute("INSERT INTO cheques(cheque_number,bank_name,received_date,amount_minor) VALUES('C-01','BNA','2026-11-01',100000)")
blocked(lambda:db.execute("INSERT INTO cheques(cheque_number,bank_name,received_date,amount_minor) VALUES('C-01','BNA','2026-11-01',100000)"),'duplicate cheque')
db.execute("INSERT INTO cheques(cheque_number,bank_name,received_date,amount_minor) VALUES('C-02','CPA','2026-11-01',80000)")
db.execute("INSERT INTO cheque_deposits(cheque_id,deposit_date,amount_minor) VALUES(1,'2026-11-02',60000)")
db.execute("INSERT INTO cheque_deposits(cheque_id,deposit_date,amount_minor) VALUES(2,'2026-11-02',80000)")
blocked(lambda:db.execute("INSERT INTO cheque_deposits(cheque_id,deposit_date,amount_minor) VALUES(1,'2026-11-03',50000)"),'deposit cannot exceed cheque face value')
assert db.execute('SELECT COUNT(*) FROM treasury_ledger').fetchone()[0]==0

db.execute("INSERT INTO bank_statement_entries(statement_reference,statement_date,entry_date,bank_transaction_reference,credit_minor,description) VALUES('ST-11','2026-11-30','2026-11-03','TX-1',120000,'Cheque credit')")
blocked(lambda:db.execute("INSERT INTO bank_statement_entries(statement_reference,statement_date,entry_date,bank_transaction_reference,credit_minor,description) VALUES('ST-11','2026-11-30','2026-11-03','TX-1',120000,'Cheque credit')"),'duplicate bank statement line')
db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(1,'2026-11-03',70000)")
db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(1,'2026-11-04',50000)")
blocked(lambda:db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(1,'2026-11-04',1)"),'collections cannot exceed bank credit')
blocked(lambda:db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(99,'2026-11-04',1)"),'collection requires valid credit entry')

db.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(1,1,40000)")
db.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(1,2,30000)")
db.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(2,1,20000)")
db.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(2,2,20000)")
blocked(lambda:db.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(2,1,1)"),'matched cheque cannot exceed deposited balance')
blocked(lambda:db.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(2,2,31000)"),'matched amount cannot exceed remaining collection value')

db.execute("INSERT INTO bank_statement_entries(statement_reference,statement_date,entry_date,debit_minor) VALUES('ST-11','2026-11-30','2026-11-05',10000)")
blocked(lambda:db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(2,'2026-11-05',10000)"),'debit entry cannot be confirmed as collection')
assert db.execute("SELECT SUM(CASE direction WHEN 'in' THEN amount_minor ELSE -amount_minor END) FROM treasury_ledger").fetchone()[0]==120000

db.execute("INSERT INTO clients(full_name) VALUES('Client One'),('Client Two')")
db.execute("INSERT INTO projects(promoter_id,name) VALUES(1,'Project A')")
db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,project_id,allocation_date,amount_minor,operation_type) VALUES(1,1,1,1,'2026-11-06',40000,'إيداع')")
db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,allocation_date,amount_minor,operation_type) VALUES(1,2,2,'2026-11-06',30000,'إيداع')")
blocked(lambda:db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,allocation_date,amount_minor,operation_type) VALUES(1,1,1,'2026-11-06',1,'إيداع')"),'allocation cannot exceed confirmed collection')
blocked(lambda:db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,project_id,allocation_date,amount_minor,operation_type) VALUES(1,2,1,1,'2026-11-06',1,'إيداع')"),'project must belong to selected promoter')
blocked(lambda:db.execute("UPDATE allocations SET amount_minor=1 WHERE id=1"),'allocation history is immutable')
blocked(lambda:db.execute("DELETE FROM allocations WHERE id=1"),'allocation history cannot be deleted')
assert db.execute("SELECT SUM(amount_minor) FROM allocations WHERE collection_id=1").fetchone()[0]==70000
print('All cheque, treasury and classification workflow tests passed.')
