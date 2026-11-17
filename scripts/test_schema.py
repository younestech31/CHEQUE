from pathlib import Path
import sqlite3
root=Path(__file__).resolve().parents[1]
db=sqlite3.connect(':memory:')
db.execute('PRAGMA foreign_keys=ON')
db.executescript((root/'src-tauri/migrations/0001_initial.sql').read_text(encoding='utf-8'))
for table,column,definition in [('cheques','issue_date','TEXT'),('cheques','notes','TEXT'),('bank_statement_entries','description','TEXT'),('collections','confirmation_reference','TEXT'),('collections','notes','TEXT'),('audit_log','after_json','TEXT')]:
 columns={r[1] for r in db.execute('PRAGMA table_info(' + table + ')')}
 if column not in columns: db.execute('ALTER TABLE ' + table + ' ADD COLUMN ' + column + ' ' + definition)
db.executescript((root/'src-tauri/migrations/0002_cheques_and_statement_workflow.sql').read_text(encoding='utf-8'))
if 'operation_type' not in {r[1] for r in db.execute('PRAGMA table_info(allocations)')}:
 db.execute("ALTER TABLE allocations ADD COLUMN operation_type TEXT NOT NULL DEFAULT 'غير محدد'")
db.executescript((root/'src-tauri/migrations/0003_classification.sql').read_text(encoding='utf-8'))
for column in ('external_reference','notes'):
 if column not in {r[1] for r in db.execute('PRAGMA table_info(outgoing_payments)')}:
  db.execute('ALTER TABLE outgoing_payments ADD COLUMN ' + column + ' TEXT')
db.executescript((root/'src-tauri/migrations/0004_payments_and_freezes.sql').read_text(encoding='utf-8'))
db.executescript((root/'src-tauri/migrations/0005_lock_payment_detail_moves.sql').read_text(encoding='utf-8'))
def blocked(fn,label):
 try: fn()
 except sqlite3.IntegrityError: print('PASS',label); return
 raise AssertionError('Expected blocked operation: '+label)
db.execute("INSERT INTO clients(full_name) VALUES('Client')")
db.execute("INSERT INTO bank_statement_entries(statement_reference,statement_date,entry_date,credit_minor) VALUES('ST-1','2026-11-01','2026-11-01',100000)")
db.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor) VALUES(1,'2026-11-01',100000)")
db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,allocation_date,amount_minor,operation_type) VALUES(1,1,1,'2026-11-02',60000,'إيداع')")
blocked(lambda:db.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,allocation_date,amount_minor,operation_type) VALUES(1,1,1,'2026-11-02',50000,'إيداع')"),'allocation ceiling')
db.execute("INSERT INTO freezes(allocation_id,amount_minor,reason,frozen_at) VALUES(1,20000,'وثيقة ناقصة','2026-11-03')")
blocked(lambda:db.execute("INSERT INTO freezes(allocation_id,amount_minor,reason,frozen_at) VALUES(1,50000,'تجميد زائد','2026-11-03')"),'freeze ceiling')
db.execute("INSERT INTO outgoing_payments(payment_reference,payment_type,promoter_id,preparation_date) VALUES('PAY-001','periodic',1,'2026-11-04')")
db.execute("INSERT INTO payment_details(payment_id,allocation_id,amount_minor) VALUES(1,1,10000)")
db.execute("UPDATE outgoing_payments SET status='reserved' WHERE id=1")
db.execute("UPDATE freezes SET released_at='2026-11-05',release_note='رفع التجميد' WHERE id=1")
assert db.execute("SELECT released_at FROM freezes WHERE id=1").fetchone()[0]=='2026-11-05'
db.execute("UPDATE outgoing_payments SET status='executed',execution_date='2026-11-05' WHERE id=1")
assert db.execute("SELECT amount_minor FROM treasury_ledger WHERE payment_id=1 AND direction='out'").fetchone()[0]==10000
assert db.execute("SELECT SUM(CASE direction WHEN 'in' THEN amount_minor ELSE -amount_minor END) FROM treasury_ledger").fetchone()[0]==90000
blocked(lambda:db.execute("INSERT INTO freezes(allocation_id,amount_minor,reason,frozen_at) VALUES(1,51000,'يتجاوز المستحق المتبقي','2026-11-06')"),'freeze cannot overlap executed payment')
report = db.execute("""
SELECT a.amount_minor,
 COALESCE((SELECT SUM(f.amount_minor) FROM freezes f WHERE f.allocation_id=a.id AND f.released_at IS NULL),0),
 COALESCE((SELECT SUM(pd.amount_minor) FROM payment_details pd JOIN outgoing_payments p ON p.id=pd.payment_id WHERE pd.allocation_id=a.id AND p.status='reserved'),0),
 COALESCE((SELECT SUM(pd.amount_minor) FROM payment_details pd JOIN outgoing_payments p ON p.id=pd.payment_id WHERE pd.allocation_id=a.id AND p.status='executed'),0),
 a.amount_minor
 -COALESCE((SELECT SUM(f.amount_minor) FROM freezes f WHERE f.allocation_id=a.id AND f.released_at IS NULL),0)
 -COALESCE((SELECT SUM(pd.amount_minor) FROM payment_details pd JOIN outgoing_payments p ON p.id=pd.payment_id WHERE pd.allocation_id=a.id AND p.status IN('reserved','executed')),0)
FROM allocations a WHERE a.id=1
""").fetchone()
assert report == (60000, 0, 0, 10000, 50000), report

# Exercise the exact financial-report joins and promoter filter against seeded data.
report_rows = db.execute("""
 SELECT a.id, pr.name, cl.full_name, pj.name, a.operation_type,
        c.collection_date, e.statement_reference, a.allocation_date,
        a.amount_minor,
        COALESCE((SELECT SUM(f.amount_minor) FROM freezes f
                  WHERE f.allocation_id=a.id AND f.released_at IS NULL), 0),
        COALESCE((SELECT SUM(pd.amount_minor) FROM payment_details pd
                  JOIN outgoing_payments op ON op.id=pd.payment_id
                  WHERE pd.allocation_id=a.id AND op.status='reserved'), 0),
        COALESCE((SELECT SUM(pd.amount_minor) FROM payment_details pd
                  JOIN outgoing_payments op ON op.id=pd.payment_id
                  WHERE pd.allocation_id=a.id AND op.status='executed'), 0),
        a.amount_minor
        - COALESCE((SELECT SUM(f.amount_minor) FROM freezes f
                    WHERE f.allocation_id=a.id AND f.released_at IS NULL), 0)
        - COALESCE((SELECT SUM(pd.amount_minor) FROM payment_details pd
                    JOIN outgoing_payments op ON op.id=pd.payment_id
                    WHERE pd.allocation_id=a.id AND op.status IN ('reserved','executed')), 0)
 FROM allocations a
 JOIN promoters pr ON pr.id=a.promoter_id
 JOIN clients cl ON cl.id=a.client_id
 JOIN collections c ON c.id=a.collection_id
 JOIN bank_statement_entries e ON e.id=c.bank_entry_id
 LEFT JOIN projects pj ON pj.id=a.project_id
 WHERE (? IS NULL OR a.promoter_id=?)
 ORDER BY pr.name COLLATE NOCASE, a.allocation_date DESC, a.id DESC
""", (1, 1)).fetchall()
assert len(report_rows) == 1, report_rows
assert report_rows[0][1:9] == ('HALA CONSTRUCTION', 'Client', None, 'إيداع', '2026-11-01', 'ST-1', '2026-11-02', 60000), report_rows
assert report_rows[0][9:] == (0, 0, 10000, 50000), report_rows
assert db.execute("""
 SELECT COUNT(*) FROM allocations a
 JOIN promoters pr ON pr.id=a.promoter_id
 WHERE (? IS NULL OR a.promoter_id=?)
""", (2, 2)).fetchone()[0] == 0

print('All core schema, freeze, execution-ledger and financial-report tests passed.')
